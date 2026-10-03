//! Why an audit line could not be appended.
//!
//! **Role:** the crate's error type and its `Result` alias.
//! **Position:** the transactional appends of [`crate::required_audit`] run on their caller's
//! business transaction and hand back sqlx's own [`sqlx::Error`] untouched, so the caller's `?`
//! classifies an audit failure exactly like the business write beside it; a missing actor account
//! is [`sqlx::Error::RowNotFound`]. [`crate::audit_writer::write_audit`] never fails. A caller
//! that keeps an audit failure apart from its other errors wraps it in [`Error::Append`].
//! **Signals & state:** none; plain data.
//! **Invariants:** [`Error::Append`] is the transparent sqlx error, so its message and its causes
//! read exactly as sqlx renders them.

/// Why an audit line could not be appended.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The insert into `audit_logs` failed, or the actor's account does not exist
    /// ([`sqlx::Error::RowNotFound`]).
    #[error(transparent)]
    Append(#[from] sqlx::Error),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
