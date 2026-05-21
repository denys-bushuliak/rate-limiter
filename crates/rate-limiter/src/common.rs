/**
 * A trait representing a limiter that can be used to limit the rate of requests.
 */
pub trait RateLimiter: Send + Sync {
    /**
     * Checks if a request is allowed based on the limiter's rules.
     *
     * Returns `true` if the request is allowed, `false` otherwise.
     */
    fn allow(&mut self) -> bool;
}
