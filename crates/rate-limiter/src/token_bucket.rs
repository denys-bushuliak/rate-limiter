use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use crate::common::RateLimiter;

#[derive(Debug)]
pub struct TokenBucket {
    rate: u32,
    capacity: u32,
    start_time: Instant,
    state: AtomicU64,
}

impl TokenBucket {
    pub fn new(rate: u32, capacity: u32) -> TokenBucket {
        // Store the initial capacity in milli-tokens (multiply by 1000)
        let init_state = Self::pack(0, capacity * 1000);

        TokenBucket {
            rate,
            capacity,
            start_time: Instant::now(),
            state: AtomicU64::new(init_state),
        }
    }

    fn pack(time: u32, tokens: u32) -> u64 {
        ((time as u64) << 32) | (tokens as u64)
    }

    fn unpack(state: u64) -> (u32, u32) {
        let last_refill = (state >> 32) as u32;
        let available_tokens = (state & 0xFFFFFFFF) as u32;
        (last_refill, available_tokens)
    }
}

impl RateLimiter for TokenBucket {
    fn allow(&self) -> bool {
        let now_ms = self.start_time.elapsed().as_millis() as u32;

        self.state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                let (last_time_ms, available_tokens) = Self::unpack(state);

                // Safe subtraction even after u32 overflow (after ~49 days)
                let elapsed_ms = now_ms.wrapping_sub(last_time_ms);

                // Calculate in u64 to prevent overflow during multiplication
                let added_milli = (elapsed_ms as u64) * (self.rate as u64);
                let mut total_milli = (available_tokens as u64) + added_milli;
                let cap_milli = (self.capacity as u64) * 1000;

                if total_milli > cap_milli {
                    total_milli = cap_milli;
                }

                let mut current_milli = total_milli as u32;

                // 1 token = 1000 milli-tokens
                if current_milli >= 1000 {
                    current_milli -= 1000;
                    // Return the new state to be written
                    Some(Self::pack(now_ms, current_milli))
                } else {
                    // Not enough tokens, cancel the operation (returns Err)
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
    use std::thread;
    use std::time::Duration;

    // 1. Basic capacity tests
    #[test]
    fn test_initial_capacity() {
        let bucket = TokenBucket::new(10, 3);

        assert!(bucket.allow(), "First request should pass");
        assert!(bucket.allow(), "Second request should pass");
        assert!(bucket.allow(), "Third request should pass");
        assert!(!bucket.allow(), "Fourth request should be blocked");
    }

    #[test]
    fn test_empty_capacity() {
        let bucket = TokenBucket::new(10, 0);
        assert!(
            !bucket.allow(),
            "A bucket with 0 capacity should not allow any requests"
        );
    }

    // 2. Time logic tests (token refill)
    #[test]
    fn test_token_refill_over_time() {
        // Rate: 10 tokens per second. That is 1 token every 100 ms.
        let bucket = TokenBucket::new(10, 2);

        // Deplete the bucket
        assert!(bucket.allow());
        assert!(bucket.allow());
        assert!(!bucket.allow());

        // Wait 150 ms. During this time, 1.5 tokens should be restored (enough for 1 request)
        thread::sleep(Duration::from_millis(150));

        assert!(
            bucket.allow(),
            "After 150 ms, at least 1 token should be available"
        );
        assert!(
            !bucket.allow(),
            "The second token should not have had time to restore yet (needs 200 ms)"
        );
    }

    #[test]
    fn test_capacity_is_never_exceeded() {
        let bucket = TokenBucket::new(100, 2);

        // Wait 50 ms. With rate=100 this would yield 5 tokens,
        // but the capacity is limited to 2.
        thread::sleep(Duration::from_millis(50));

        assert!(bucket.allow());
        assert!(bucket.allow());
        assert!(
            !bucket.allow(),
            "The number of tokens must not exceed the maximum capacity (2)"
        );
    }

    // 3. Multithreading test (most important for Atomics)
    #[test]
    fn test_concurrent_lock_free_access() {
        // Capacity of 50 tokens
        let bucket = Arc::new(TokenBucket::new(100, 50));
        let mut handles = vec![];

        // Start 10 threads trying to consume tokens simultaneously
        for _ in 0..10 {
            let b = Arc::clone(&bucket);
            handles.push(thread::spawn(move || {
                let mut success_count = 0;
                // Each thread makes 20 attempts
                for _ in 0..20 {
                    if b.allow() {
                        success_count += 1;
                    }
                }
                success_count
            }));
        }

        // Collect results from all threads
        let total_success: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();

        // Since threads execute almost instantly (in microseconds),
        // new tokens won't have time to generate.
        // Therefore, the total number of successful requests from all threads should equal the capacity (50).
        // (Adding a margin up to 52 in case testing runs on a very slow CI machine)
        assert!(
            total_success >= 50 && total_success <= 52,
            "Under concurrent conditions, the number of successful requests ({}) must match the capacity",
            total_success
        );
    }
}
