# Library Reference: `rate-limiter`

The `rate-limiter` crate provides a high-performance, lock-free implementation of various rate-limiting algorithms. All implementations are designed to be thread-safe (`Send + Sync`) and utilize atomic operations for maximum efficiency in concurrent environments.

## The `RateLimiter` Trait

The core of the library is the `RateLimiter` trait. Any implementation of this trait provides a way to check if a request should be allowed.

```rust
pub trait RateLimiter: Send + Sync {
    /// Checks if a request is allowed based on the limiter's rules.
    /// Returns `true` if the request is allowed, `false` otherwise.
    fn allow(&self) -> bool;
}
```

---

## Implementations

### 1. Fixed Window (`FixedWindow`)

The Fixed Window algorithm divides time into fixed-size intervals (windows). For each window, a counter tracks the number of requests. When the window expires, the counter resets.

*   **Mechanism**: Uses an atomic 64-bit state to pack the `window_id` and `request_count`.
*   **Complexity**: $O(1)$ for `allow()`.
*   **Limitations**: Can allow up to $2 \times \text{capacity}$ requests in a single window boundary (bursting).

**Constructor:**
```rust
pub fn new(window_size: Duration, max_requests: usize) -> Self
```

#### Implementation Details
The state is managed using a lock-free `fetch_update` on an `AtomicU64`. The state is packed as:
`[32 bits: window_id] | [32 bits: request_count]`

---

### 2. Leaky Bucket (`LeakyBucket`)

The Leaky Bucket algorithm mimics a bucket with a small hole at the bottom. Requests enter the bucket, and they "leak" out at a constant rate. If the bucket is full, new requests are rejected.

*   **Mechanism**: Tracks the amount of "water" (requests) in the bucket and the last time it was updated.
*   **Complexity**: $O(1)$ for `allow()`.

**Constructor:**
```rust
pub fn new(capacity: u32, leak_rate: u32) -> Self
```

#### Implementation Details
The state is managed using an atomic 64-bit value packed as:
`[32 bits: last_updated_time_ms] | [32 bits: current_bucket_size]`

---

### 3. Sliding Window Counter (`SlidingWindow`)

The Sliding Window algorithm provides a more accurate approximation of a sliding window by combining the counts of the current window and the previous window, weighted by the progress into the current window.

*   **Mechanism**: Uses an approximation formula (similar to Cloudflare's) to estimate the current request rate.
*   **Complexity**: $O(1)$ for `allow()`.

**Constructor:**
```rust
pub fn new(window_size: Duration, max_requests: usize) -> Self
```

#### Implementation Details
The state is managed using an atomic 64-bit value packed as:
`[32 bits: window_id] | [16 bits: prev_count] | [16 bits: curr_count]`

**Approximation Formula:**
$$\text{estimated\_requests} = (\text{prev\_count} \times (1 - \text{progress})) + \text{curr\_count}$$
where $\text{progress}$ is the fraction of the current window that has elapsed.

---

### 4. Token Bucket (`TokenBucket`)

The Token Bucket algorithm allows for bursts of requests by adding tokens to a bucket at a constant rate. Each request consumes one token.

*   **Mechanism**: Tokens are added at a specified rate. If a token is available, the request is allowed and a token is removed.
*   **Complexity**: $O(1)$ for `allow()`.

**Constructor:**
```rust
pub fn new(rate: u32, capacity: u32) -> TokenBucket
```

#### Implementation Details
The state is managed using an atomic 64-bit value packed as:
`[32 bits: last_refill_time_ms] | [32 bits: available_tokens_in_milli]`

Tokens are tracked in "milli-tokens" (1 token = 1000 milli-tokens) to allow for smooth, fractional token replenishment between integer time steps.
