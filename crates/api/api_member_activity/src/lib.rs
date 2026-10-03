//! Member activity: the aggregates and queues that API member-level facts feed.
//!
//! **Role:** keeps each member's deployment and attendance figures, the leaderboard and the event
//! eligibility re-evaluation queue current as matches, identities and memberships change.
//! **Position:** above [`api_audit_log`], `api_foundation` and `api_identifiers`; called inside
//! the business transactions of the identity and access, match telemetry, operations and
//! administration domains, by the API's background workers and by the integration suites; names
//! no domain.
//! **Signals & state:** none in memory; the `users` counters, the `leaderboard_totals` view and
//! the `event_reservation_reevaluations` rows are its state.
//! **Invariants:** every writer runs on its caller's transaction and honours the caller's lock
//! order; the leaderboard refresh is the last statement before commit.

mod error;
pub mod leaderboard_view;
pub mod participation_attribution;
pub mod prelude;
pub mod reevaluation_queue;
pub mod user_stats;

pub use error::{Error, Result};
