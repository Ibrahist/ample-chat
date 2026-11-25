use std::time::Instant;

/// Token-bucket rate limiter per WebSocket connection.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    max_tokens: f64,
    refill_rate_per_sec: f64,
    tokens: f64,
    last_update: Instant,
}

impl RateLimiter {
    pub fn new(max_tokens: u32, refill_rate_per_sec: f64) -> Self {
        Self {
            max_tokens: max_tokens as f64,
            refill_rate_per_sec,
            tokens: max_tokens as f64,
            last_update: Instant::now(),
        }
    }

    /// Attempts to consume 1 token. Returns true if allowed, false if rate limited.
    pub fn check_and_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        // Refill tokens based on elapsed time
        self.tokens = (self.tokens + elapsed * self.refill_rate_per_sec).min(self.max_tokens);

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
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
    fn test_rate_limiter_allows_burst() {
        let mut limiter = RateLimiter::new(5, 1.0);
        for _ in 0..5 {
            assert!(limiter.check_and_consume(), "token should be available");
        }
        assert!(
            !limiter.check_and_consume(),
            "burst limit should be exhausted"
        );
    }
}
