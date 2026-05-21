use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use crate::RateLimiter;

pub struct SlidingWindow {
    window_size: Duration,
    max_requests: usize,
    requests: VecDeque<Instant>,
}

impl SlidingWindow {
    pub fn new(window_size: Duration, max_requests: usize) -> Self {
        Self {
            window_size,
            max_requests,
            requests: VecDeque::new(),
        }
    }
}

impl RateLimiter for SlidingWindow {
    fn allow(&mut self) -> bool {
        let now = Instant::now();

        while self
            .requests
            .get(0)
            .and_then(|req_time| {
                if (now - *req_time) >= self.window_size {
                    Some(())
                } else {
                    None
                }
            })
            .is_some()
        {
            self.requests.pop_front();
        }

        if self.requests.len() < self.max_requests {
            self.requests.push_back(now);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;

    use super::*;

    #[test]
    fn test_window_allow() {
        let mut limiter = SlidingWindow::new(Duration::from_secs(1), 5);
        assert!(limiter.allow());
    }

    #[test]
    fn test_window_requests_allow() {
        let mut limiter = SlidingWindow::new(Duration::from_secs(1), 5);
        for _ in 0..5 {
            assert!(limiter.allow());
        }
        assert!(!limiter.allow());
    }

    #[test]
    fn test_window_requests_deny() {
        let mut limiter = SlidingWindow::new(Duration::from_secs(1), 2);
        assert!(limiter.allow());
        assert!(limiter.allow());
        assert!(!limiter.allow());
    }

    #[test]
    fn test_window_deny() {
        let mut limiter = SlidingWindow::new(Duration::from_secs(1), 2);
        assert!(limiter.allow());
        assert!(limiter.allow());

        sleep(Duration::from_millis(900));
        assert!(!limiter.allow());

        sleep(Duration::from_millis(100));
        assert!(limiter.allow());
    }
}
