//! What one virtual client leaves behind when its run ends.
//!
//! - **Role:** the per-client record the report folds: the client's address, its exchange records
//!   and its counts of skipped, unsent and guard-delayed slots and late switches.
//! - **Position:** each virtual client of the load generator returns one; the report assembles
//!   them.
//! - **Signals & state:** none; a plain value.
//! - **Invariants:** holds counts and records only, never a token, a header or a body.

use crate::latency_recording::RequestRecord;

/// What a client leaves behind when its run ends.
#[derive(Debug)]
pub struct ClientOutcome {
    /// The index of the client's source address in the plan.
    pub address: usize,
    /// Every exchange the client sent, refreshes included.
    pub records: Vec<RequestRecord>,
    /// Paced slots that found no account yet and were skipped.
    pub skipped_slots: u64,
    /// Paced slots left unsent when the run ended.
    pub unsent_slots: u64,
    /// Requests the address's guard held back past their due instant.
    pub guard_delayed: u64,
    /// Account switches whose refresh was still running at the switch instant.
    pub late_switches: u64,
}
