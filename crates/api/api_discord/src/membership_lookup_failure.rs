//! A guild-member read through the bot token that produced no usable answer.
//!
//! **Role:** the failure [`crate::discord_client::DiscordService::fetch_member_with_bot`]
//! reports instead of a membership: a reason, how long to wait, and whether Discord rate-limited
//! the read.
//! **Position:** `api_discord`; the REST reconciliation of `api_identity_and_access` records it on
//! the membership snapshot and counts its outcome in the metrics registry.
//! **Signals & state:** none; a plain value.
//! **Invariants:** `reason` is a fixed phrase, never a token or a response body; `retry_after` is at
//! least one second on a 429 and 60 seconds otherwise.

use chrono::Duration;

use api_http_layer::observability::metrics_registry::DiscordReconcileOutcome;

/// Why a Discord member read produced no observation, and when to read again.
pub struct MembershipLookupFailure {
    /// The failure recorded as the snapshot's `last_error`; never a token or a response body.
    pub reason: &'static str,
    /// How long the snapshot, and on a 429 the shared request schedule, waits before the next read.
    pub retry_after: Duration,
    /// Discord answered 429.
    pub rate_limited: bool,
}

impl MembershipLookupFailure {
    /// A failure without a usable Discord answer, read again after 60 seconds.
    pub fn unavailable(reason: &'static str) -> Self {
        Self {
            reason,
            retry_after: Duration::seconds(60),
            rate_limited: false,
        }
    }

    /// The reconciliation outcome this failure reports: `rate_limited` on a 429, else
    /// `unavailable`.
    pub fn outcome(&self) -> DiscordReconcileOutcome {
        if self.rate_limited {
            DiscordReconcileOutcome::RateLimited
        } else {
            DiscordReconcileOutcome::Unavailable
        }
    }
}
