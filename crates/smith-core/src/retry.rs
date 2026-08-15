// crates/smith-core/src/retry.rs
/// Retry policy for an RPA step.
#[derive(Debug, Clone, Default)]
pub struct RetryPolicy {
    /// Maximum number of retries (0 — no retries).
    pub max_retries: u32,
    /// Delay between retries in milliseconds.
    pub delay_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_defaults() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 0);
        assert_eq!(policy.delay_ms, 0);
    }

    #[test]
    fn test_retry_policy_custom() {
        let policy = RetryPolicy {
            max_retries: 5,
            delay_ms: 1000,
        };
        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.delay_ms, 1000);
    }
}
