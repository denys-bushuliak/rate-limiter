use crate::common::RateLimiter;

pub type Tokens = f64;

#[derive(Debug, Clone)]
pub struct TokenBucket {
    rate: RequestsPerSecond,
    capacity: Tokens,
    available_tokens: Tokens,
    last_refill: std::time::Instant,
}

pub type RequestsPerSecond = f64;

impl TokenBucket {
    /**
     * Creates a new TokenBucket with the given rate and capacity.
     *
     * # Arguments
     *
     * * `rate` - The rate at which tokens are added to the requests per second.
     * * `capacity` - The maximum number of requests the bucket can hold.
     */
    pub fn new(rate: RequestsPerSecond, capacity: Tokens) -> TokenBucket {
        TokenBucket {
            rate,
            capacity: capacity.clone(),
            available_tokens: capacity,
            last_refill: std::time::Instant::now(),
        }
    }
}

impl RateLimiter for TokenBucket {
    fn allow(&mut self) -> bool {
        self.available_tokens += self.last_refill.elapsed().as_secs_f64() * self.rate;
        self.available_tokens = self.available_tokens.min(self.capacity);

        self.last_refill = std::time::Instant::now();

        if self.available_tokens >= 1.0 {
            self.available_tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_with_full_capacity() {
        let mut bucket = TokenBucket::new(1.0, 1.0);
        assert!(bucket.allow());
        assert!(!bucket.allow());
    }

    #[test]
    fn test_token_bucket_with_empty_capacity() {
        let mut bucket = TokenBucket::new(1.0, 0.0);
        assert!(!bucket.allow());
    }
}
