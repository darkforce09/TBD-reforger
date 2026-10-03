//! The crate's error: why a document read or a wave-lock parse refused.
//!
//! **Role:** [`Error`] and [`Result`], the refusals of
//! [`resolve_repo_rel`](crate::document_viewer::services::document_loading::resolve_repo_rel) and
//! [`parse_lock`](crate::wave_plan::services::lock_file::parse_lock). A corpus refusal keeps its
//! own type, [`LoadError`](crate::ticket_registry::models::corpus::LoadError), since it names
//! the offending file.
//! **Position:** returned by the document viewer and the wave plan; the views show the text.
//! **Signals & state:** none.
//! **Invariants:** each variant displays its refusal text verbatim, with no prefix, because the
//! views render it as the reason.

/// Why a ticketboard model operation refused; the display is the verbatim refusal text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A document path is absolute, escapes the repository root or names the root itself.
    #[error("{0}")]
    DocumentRefused(String),
    /// The wave-lock text is not a lock: the TOML or shape error, verbatim.
    #[error("{0}")]
    WaveLockUnparsable(String),
}

/// The crate's result type.
pub type Result<T, E = Error> = std::result::Result<T, E>;
