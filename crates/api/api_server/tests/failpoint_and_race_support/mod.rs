//! Shared support for the failure-injection and controlled-race suites.
//!
//! **Role:** the one toolbox the `failure_injection*` and `controlled_races*` binaries share: the
//! failpoint suite lock and arming helpers ([`failpoint_arming`]), a held PostgreSQL row lock, the
//! wait for its blocked waiters and the backend of a transaction paused at a failpoint
//! ([`row_lock_barrier`]), the runner that plays a two-party race
//! in both orders ([`interleavings`]), and the checks of the persisted state every observed result
//! is held to ([`persisted_invariants`]).
//! **Position:** compiled into each suite that writes `mod failpoint_and_race_support;`; it
//! depends only on `api_server` (the `failpoints` feature is on in every test build), `sqlx`,
//! `tokio` and `axum`, never on `tests/common`, so a suite chooses its own fixtures.
//! **Signals & state:** none of its own; arming goes through the process-global registry of
//! `api_failpoints`, which the suite lock serialises.
//! **Invariants:** every case that arms a failpoint, or passes a point another case may arm, holds
//! [`lock_suite`] for its whole body and declares the lock before any guard; every wait is bounded
//! and panics with the boundary it waited for; a check of the persisted state reads committed rows
//! only and names the violated invariant in its error.

// Each suite binary compiles its own copy of this module and uses a different subset of it, so
// an item one suite leaves unused is not dead code, but rustc judges each binary on its own and
// the gate runs `clippy --all-targets -- -D warnings`.
#![allow(dead_code)]

pub(crate) mod failpoint_arming;
pub(crate) mod interleavings;
pub(crate) mod persisted_invariants;
pub(crate) mod row_lock_barrier;

// The same reason one level up: a re-export no single suite names is still the surface every
// other suite reaches through.
#[allow(unused_imports)]
pub(crate) use self::{
    failpoint_arming::{
        ArmGuard, CATALOGUE, FailAction, Failpoint, FailpointArming, FailpointSuiteLock,
        PauseHandle, PausedFailpoint, assert_injected_failure, lock_suite, reach,
    },
    interleavings::{Interleaving, run_in_both_orders},
    persisted_invariants::{
        AuditEvidence, RowCounts, audit_evidence, check_arma_identity_held_once,
        check_event_seats_and_places, check_fleet_outcome_recorded_once,
        check_publication_sequence, check_refresh_families, check_review_decided_once,
    },
    row_lock_barrier::{
        BLOCKED_WAIT_BOUND, RowLockHolder, paused_transaction_backend, wait_for_blocked,
    },
};
