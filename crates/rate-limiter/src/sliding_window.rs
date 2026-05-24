use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use crate::RateLimiter;

#[derive(Debug)]
pub struct SlidingWindow {
    window_size: Duration,
    max_requests: u32,
    start_time: Instant,
    // state holds 3 values packed into 64 bits:
    // [32 bits: window_id] | [16 bits: prev_count] | [16 bits: curr_count]
    state: AtomicU64,
}

impl SlidingWindow {
    pub fn new(window_size: Duration, max_requests: usize) -> Self {
        // Since we use 16 bits for counting, max_requests cannot exceed 65535
        assert!(
            max_requests <= u16::MAX as usize,
            "max_requests must fit in 16 bits"
        );

        assert!(
            window_size.as_millis() > 0,
            "window_size must be greater than 0"
        );

        Self {
            window_size,
            max_requests: max_requests as u32,
            start_time: Instant::now(),
            state: AtomicU64::new(0),
        }
    }

    fn pack(window_id: u32, prev_count: u16, curr_count: u16) -> u64 {
        ((window_id as u64) << 32) | ((prev_count as u64) << 16) | (curr_count as u64)
    }

    fn unpack(state: u64) -> (u32, u16, u16) {
        let window_id = (state >> 32) as u32;
        let prev_count = ((state >> 16) & 0xFFFF) as u16;
        let curr_count = (state & 0xFFFF) as u16;
        (window_id, prev_count, curr_count)
    }
}

impl RateLimiter for SlidingWindow {
    // Note: Changed to &self as with the TokenBucket to enable lock-free usage
    fn allow(&self) -> bool {
        let now_ms = self.start_time.elapsed().as_millis();
        let window_ms = self.window_size.as_millis();

        // Calculate which time window we are currently in
        let current_window_id = (now_ms / window_ms) as u32;

        // How far along are we in the current window? (0.0 to 1.0)
        let progress = (now_ms % window_ms) as f64 / window_ms as f64;

        // The weight of the previous window shrinks as we move forward
        let prev_weight = 1.0 - progress;

        self.state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                let (mut saved_window_id, mut prev_count, mut curr_count) = Self::unpack(state);

                // If time has moved to a new window, shift our counters
                if current_window_id != saved_window_id {
                    let diff = current_window_id.wrapping_sub(saved_window_id);
                    if diff == 1 {
                        // Moved exactly to the next window
                        prev_count = curr_count;
                    } else {
                        // Skipped one or more windows completely
                        prev_count = 0;
                    }
                    curr_count = 0;
                    saved_window_id = current_window_id;
                }

                // Cloudflare's Sliding Window Counter approximation formula
                let estimated_requests = (prev_count as f64 * prev_weight) + (curr_count as f64);

                if estimated_requests >= self.max_requests as f64 {
                    // Limit reached, abort CAS loop
                    None
                } else {
                    // Allowed, increment current window counter
                    curr_count += 1;
                    Some(Self::pack(saved_window_id, prev_count, curr_count))
                }
            })
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_window_allow() {
        let limiter = SlidingWindow::new(Duration::from_secs(1), 5);
        assert!(limiter.allow());
    }

    #[test]
    fn test_window_requests_allow() {
        let limiter = SlidingWindow::new(Duration::from_secs(1), 5);
        for _ in 0..5 {
            assert!(limiter.allow());
        }
        assert!(!limiter.allow());
    }

    #[test]
    fn test_window_requests_deny() {
        let limiter = SlidingWindow::new(Duration::from_secs(1), 2);
        assert!(limiter.allow());
        assert!(limiter.allow());
        assert!(!limiter.allow());
    }

    #[test]
    fn test_window_deny() {
        let limiter = SlidingWindow::new(Duration::from_secs(1), 2);

        // Fill the current window (window 0)
        assert!(limiter.allow());
        assert!(limiter.allow());

        // Wait 900ms. We are still in window 0.
        sleep(Duration::from_millis(900));
        assert!(!limiter.allow());

        // Wait another 150ms to cross safely into window 1.
        // Total time ~1050ms.
        // Prev count = 2, weight is approx 0.95. Estimated = 2 * 0.95 = 1.9.
        // Since 1.9 < 2.0, this should now be allowed.
        sleep(Duration::from_millis(150));
        assert!(limiter.allow());
    }
}
