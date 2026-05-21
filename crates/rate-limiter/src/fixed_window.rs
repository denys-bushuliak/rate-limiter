use std::time::{Duration, Instant};

use crate::RateLimiter;

pub struct FixedWindow {
    window_size: Duration,
    max_requests: usize,
    requests: usize,
    window_start: Instant,
}

impl FixedWindow {
    pub fn new(window_size: Duration, max_requests: usize) -> Self {
        Self {
            window_size,
            max_requests,
            requests: 0,
            window_start: Instant::now(),
        }
    }
}

impl RateLimiter for FixedWindow {
    fn allow(&mut self) -> bool {
        let now = Instant::now();
        if now - self.window_start >= self.window_size {
            self.window_start = now;
            self.requests = 0;
        }
        if self.requests < self.max_requests {
            self.requests += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn allows_requests_under_limit() {
        let mut limiter = FixedWindow::new(Duration::from_secs(10), 3);

        assert!(limiter.allow(), "First request should be allowed");
        assert!(limiter.allow(), "Second request should be allowed");
        assert!(limiter.allow(), "Third request should be allowed");
    }

    #[test]
    fn blocks_requests_over_limit() {
        let mut limiter = FixedWindow::new(Duration::from_secs(10), 2);

        assert!(limiter.allow(), "First request should be allowed");
        assert!(limiter.allow(), "Second request should be allowed");

        assert!(!limiter.allow(), "Third request should be blocked");
        assert!(!limiter.allow(), "Fourth request should be blocked");
    }

    #[test]
    fn resets_counter_after_window_expires() {
        let window = Duration::from_millis(50);
        let mut limiter = FixedWindow::new(window, 1);

        assert!(limiter.allow(), "First request should be allowed");
        assert!(!limiter.allow(), "Second request should be blocked");

        // Wait for the window to expire
        sleep(Duration::from_millis(60));

        assert!(limiter.allow(), "Request in new window should be allowed");
    }

    #[test]
    fn handles_zero_capacity() {
        let mut limiter = FixedWindow::new(Duration::from_secs(10), 0);

        assert!(
            !limiter.allow(),
            "Request should be blocked immediately with zero capacity"
        );
    }
}
