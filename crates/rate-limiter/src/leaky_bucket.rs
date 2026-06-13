use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use crate::RateLimiter;

#[derive(Debug)]
pub struct LeakyBucket {
    capacity: u32,
    _some_field: PhantomData<u32>,
    leak_rate: u32,
    start_time: Instant,
    // state holds 2 values packed into 64 bits:
    // [32 bits: time in ms] | [32 bits: bucket_size]
    state: AtomicU64,
}

impl LeakyBucket {
    pub fn new(capacity: u32, leak_rate: u32) -> Self {
        // Initial bucket size is 0
        let init_state = Self::pack(0, 0);

        Self {
            capacity,
            leak_rate,
            start_time: Instant::now(),
            state: AtomicU64::new(init_state),
            _some_field: PhantomData,
        }
    }

    fn pack(time: u32, size: u32) -> u64 {
        ((time as u64) << 32) | (size as u64)
    }

    fn unpack(state: u64) -> (u32, u32) {
        let last_updated = (state >> 32) as u32;
        let bucket_size = (state & 0xFFFFFFFF) as u32;
        (last_updated, bucket_size)
    }
}

impl RateLimiter for LeakyBucket {
    // Note: Changed to &self to enable lock-free usage
    fn allow(&self) -> bool {
        let now_ms = self.start_time.elapsed().as_millis() as u32;

        self.state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                let (last_time_ms, current_size) = Self::unpack(state);

                // Safely calculate elapsed time, even after u32 overflow
                let elapsed_ms = now_ms.wrapping_sub(last_time_ms);

                // Note: The original variable was called elapsed_secs,
                // but it stored milliseconds. We continue to use milliseconds.
                // Calculate in u64 to prevent overflow
                let leaked = (elapsed_ms as u64) * (self.leak_rate as u64);

                // Subtract leaked items, flooring at 0
                let new_size = (current_size as u64).saturating_sub(leaked) as u32;

                // Check if the bucket has room for 1 more request
                if new_size < self.capacity {
                    // Allowed: increment size and write new state
                    Some(Self::pack(now_ms, new_size + 1))
                } else {
                    // Blocked: bucket is full, abort the CAS loop
                    None
                }
            })
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread::{self, sleep};
    use std::time::Duration;

    #[test]
    fn allows_requests_under_capacity() {
        let limiter = LeakyBucket::new(3, 1);

        assert!(limiter.allow());
        assert!(limiter.allow());
        assert!(limiter.allow());
    }

    #[test]
    fn blocks_when_bucket_is_full() {
        let limiter = LeakyBucket::new(2, 1);

        assert!(limiter.allow());
        assert!(limiter.allow());

        assert!(!limiter.allow());
    }

    #[test]
    fn leaks_over_time_allowing_new_requests() {
        let limiter = LeakyBucket::new(1, 10);

        assert!(limiter.allow());
        assert!(!limiter.allow());

        sleep(Duration::from_millis(150));

        assert!(limiter.allow());
    }

    // Multithreading test to verify lock-free behavior
    #[test]
    fn test_concurrent_lock_free_access() {
        // Capacity of 50, leak_rate is 0 so items never leave the bucket
        let bucket = Arc::new(LeakyBucket::new(50, 0));
        let mut handles = vec![];

        for _ in 0..10 {
            let b = Arc::clone(&bucket);
            handles.push(thread::spawn(move || {
                let mut success_count = 0;
                for _ in 0..20 {
                    if b.allow() {
                        success_count += 1;
                    }
                }
                success_count
            }));
        }

        let total_success: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();

        // Since the leak rate is 0, exactly 50 requests should be allowed
        // across all threads combined.
        assert_eq!(
            total_success, 50,
            "Total successful requests must exactly match the capacity"
        );
    }
}
