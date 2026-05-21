use std::time::Instant;

use crate::RateLimiter;

pub struct LeakyBucket {
    capacity: f64,
    leak_rate: f64,
    bucket_size: f64,
    last_updated: std::time::Instant,
}

impl LeakyBucket {
    pub fn new(capacity: f64, leak_rate: f64) -> Self {
        Self {
            capacity,
            leak_rate,
            bucket_size: 0.0,
            last_updated: std::time::Instant::now(),
        }
    }
}

impl RateLimiter for LeakyBucket {
    fn allow(&mut self) -> bool {
        let now = Instant::now();
        let elapsed_secs = now.duration_since(self.last_updated).as_secs_f64();

        let leaked = elapsed_secs * self.leak_rate;

        self.bucket_size = if self.bucket_size > leaked {
            self.bucket_size - leaked
        } else {
            0.0
        };

        self.last_updated = now;

        if self.bucket_size + 1.0 <= self.capacity {
            self.bucket_size += 1.0;
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
    fn allows_requests_under_capacity() {
        let mut limiter = LeakyBucket::new(3.0, 1.0);

        assert!(limiter.allow());
        assert!(limiter.allow());
        assert!(limiter.allow());
    }

    #[test]
    fn blocks_when_bucket_is_full() {
        let mut limiter = LeakyBucket::new(2.0, 0.001);

        assert!(limiter.allow());
        assert!(limiter.allow());

        assert!(!limiter.allow());
    }

    #[test]
    fn leaks_over_time_allowing_new_requests() {
        let mut limiter = LeakyBucket::new(1.0, 10.0);

        assert!(limiter.allow());
        assert!(!limiter.allow());

        sleep(Duration::from_millis(150));

        assert!(limiter.allow());
    }
}
