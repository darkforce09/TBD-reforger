//! Where a recorded run keeps what it saw: the observation journal and the browser inbox.
//!
//! **Role:** declares [`journal`] (JSONL plus raw artifacts by SHA-256) and [`browser_inbox`]
//! (the page reads the orchestrator saves, accepted only inside their step's window).
//!
//! **Position:** both live in the run folder `target/staging/<check>/<run>/`, created by
//! `procedure_runner/recording.rs` and filled by `procedure_runner/runner.rs`.
//!
//! **Signals & state:** the journal holds its open file; the inbox holds only its folder.
//!
//! **Invariants:** every observation the runner makes is journaled with the digest of its raw
//! bytes; no credential is ever archived.

pub(crate) mod browser_inbox;
pub(crate) mod journal;
