//! Why the mission document refuses an incoming update.
//!
//! **Role:** [`Error`], the refusals of [`crate::MissionDocCore::apply_update`], and [`Result`].
//! **Position:** re-exported at the crate root; the Mission Creator's restore and merge paths
//! test the result and drop a refused update.
//! **Signals & state:** none; plain values.
//! **Invariants:** each message is one readable sentence naming what was refused and why.

/// Why the mission document refuses an incoming Yjs update.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a Yjs v1 update.
    #[error("{0}")]
    UpdateDecode(String),

    /// The update carries more edits under this document's own client id than it has made.
    #[error(
        "client id collision: incoming update carries blocks authored by client {client} up to \
         clock {claimed}, but that is this document's own id and it has only issued {issued}. \
         Another writer is authoring as us; applying this would interleave two authors into one \
         history. Give every peer its own id — MissionDocCore::new() mints one."
    )]
    ClientIdCollision {
        /// This document's client id.
        client: u64,
        /// The clock the update claims for that client.
        claimed: u32,
        /// The clock this document has issued.
        issued: u32,
    },

    /// `yrs` refused to integrate the decoded update.
    #[error("{0}")]
    UpdateApply(String),
}

/// The result of applying an update to the mission document.
pub type Result<T> = std::result::Result<T, Error>;
