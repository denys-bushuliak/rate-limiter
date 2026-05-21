use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use crate::RateLimiter;

#[derive(Debug)]
pub struct FixedWindow {
    window_size: Duration,
    max_requests: u32,
    start_time: Instant,
    // state holds 2 values packed into 64 bits:
    // [32 bits: window_id] | [32 bits: request_count]
    state: AtomicU64,
}

impl FixedWindow {
    pub fn new(window_size: Duration, max_requests: usize) -> Self {
        Self {
            window_size,
            max_requests: max_requests as u32,
            start_time: Instant::now(),
            state: AtomicU64::new(0),
        }
    }

    fn pack(window_id: u32, requests: u32) -> u64 {
        ((window_id as u64) << 32) | (requests as u64)
    }

    fn unpack(state: u64) -> (u32, u32) {
        let window_id = (state >> 32) as u32;
        let requests = (state & 0xFFFFFFFF) as u32;
        (window_id, requests)
    }
}

impl RateLimiter for FixedWindow {
    // Note: Changed to &self to enable lock-free usage
    fn allow(&self) -> bool {
        let window_ms = self.window_size.as_millis();

        // Prevent division by zero if window_size is 0
        if window_ms == 0 {
            return false;
        }

        // Calculate the current window ID based on elapsed time
        let current_window_id = (self.start_time.elapsed().as_millis() / window_ms) as u32;

        self.state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                let (saved_window_id, mut requests) = Self::unpack(state);

                // If we moved to a new window, reset the counter
                if current_window_id != saved_window_id {
                    requests = 0;
                }

                // Check if we are still under the limit for the active window
                if requests < self.max_requests {
                    // Allowed: increment the request counter and update state
                    Some(Self::pack(current_window_id, requests + 1))
                } else {
                    // Blocked: max requests reached for this window
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
    fn allows_requests_under_limit() {
        let limiter = FixedWindow::new(Duration::from_secs(10), 3);

        assert!(limiter.allow(), "First request should be allowed");
        assert!(limiter.allow(), "Second request should be allowed");
        assert!(limiter.allow(), "Third request should be allowed");
    }

    #[test]
    fn blocks_requests_over_limit() {
        let limiter = FixedWindow::new(Duration::from_secs(10), 2);

        assert!(limiter.allow(), "First request should be allowed");
        assert!(limiter.allow(), "Second request should be allowed");

        assert!(!limiter.allow(), "Third request should be blocked");
        assert!(!limiter.allow(), "Fourth request should be blocked");
    }

    #[test]
    fn resets_counter_after_window_expires() {
        let window = Duration::from_millis(50);
        let limiter = FixedWindow::new(window, 1);

        assert!(limiter.allow(), "First request should be allowed");
        assert!(!limiter.allow(), "Second request should be blocked");

        // Wait for the window to expire
        sleep(Duration::from_millis(60));

        assert!(limiter.allow(), "Request in new window should be allowed");
    }

    #[test]
    fn handles_zero_capacity() {
        let limiter = FixedWindow::new(Duration::from_secs(10), 0);

        assert!(
            !limiter.allow(),
            "Request should be blocked immediately with zero capacity"
        );
    }

    // Multithreading test to verify lock-free behavior
    #[test]
    fn test_concurrent_lock_free_access() {
        // Window is large enough so it won't reset during the test
        let bucket = Arc::new(FixedWindow::new(Duration::from_secs(10), 50));
        let mut handles = vec![];

        // Start 10 threads, each attempting to acquire tokens
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

        // Exactly 50 requests should succeed across all threads
        assert_eq!(
            total_success, 50,
            "Total successful requests must exactly match the capacity"
        );
    }
}
