//! Why a load or an ingest into the world residency is refused.
//!
//! **Role:** the crate's one error type and its `Result` alias: the chunk residency refused the
//! payload or the chunk container ([`chunk_scheduler::Error`]).
//! **Position:** returned by the loads and ingests of [`crate::world_residency`] and
//! [`crate::chunk_residency_delegates`].
//! **Signals & state:** none; plain data.
//! **Invariants:** the residency variant wraps its error transparently, so the message is the
//! residency's.

/// Why a load or an ingest is refused.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The chunk residency refused a payload or a chunk container.
    #[error(transparent)]
    Residency(#[from] chunk_scheduler::Error),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
