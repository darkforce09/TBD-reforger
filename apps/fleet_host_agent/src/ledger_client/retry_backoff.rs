//! Exponential backoff with jitter for retrying the API.

use std::time::Duration;

/// Doublings after which the step stops growing; the maximum caps it long before.
const MAX_DOUBLINGS: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackoffPolicy {
    /// The step after the first failure.
    pub initial: Duration,
    /// The largest step.
    pub maximum: Duration,
}

/// Delays whose step doubles from `initial` up to `maximum` with every consecutive failure.
/// Each delay is drawn uniformly from the upper half of its step, so agents that failed
/// together do not retry together, and no delay is shorter than half its step.
#[derive(Debug, Clone)]
pub struct JitteredBackoff {
    policy: BackoffPolicy,
    failures: u32,
}

impl JitteredBackoff {
    pub fn new(policy: BackoffPolicy) -> Self {
        Self {
            policy,
            failures: 0,
        }
    }

    /// The delay before the next attempt, counting one more consecutive failure.
    pub fn next_delay(&mut self) -> Duration {
        let step = self.step();
        self.failures = self.failures.saturating_add(1);
        let half = step / 2;
        half + random_share_of(step - half)
    }

    /// Starts again from `initial` after a success.
    pub fn reset(&mut self) {
        self.failures = 0;
    }

    fn step(&self) -> Duration {
        let doubling = 2u32.saturating_pow(self.failures.min(MAX_DOUBLINGS));
        self.policy
            .initial
            .saturating_mul(doubling)
            .min(self.policy.maximum)
    }
}

fn random_share_of(span: Duration) -> Duration {
    let nanoseconds = u64::try_from(span.as_nanos()).unwrap_or(u64::MAX);
    Duration::from_nanos(rand::random_range(0..=nanoseconds))
}

#[cfg(test)]
#[path = "tests/retry_backoff.rs"]
mod tests;
