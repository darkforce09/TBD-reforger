//! Historical wave plan lookup for the wave driver.
//!
//! **Role:** `tickets_at` returns the tickets wave `n` held in the wave lock committed at `rev`.
//!
//! **Position:** called by the wave-base oracles in `base`; delegates to
//! [`ticket_wave_lock::archived_wave_plans`].
//!
//! **Signals & state:** none; reads git blobs relative to the current directory, which `Ctx::enter`
//! set to the repository root.
//!
//! **Invariants:** a revision or wave the lock cannot answer for yields an empty list, which the
//! callers treat as silence, never as confirmation.

pub(crate) fn tickets_at(rev: &str, n: i64) -> Vec<String> {
    ticket_wave_lock::archived_wave_plans::tickets_at(std::path::Path::new("."), rev, n)
}
