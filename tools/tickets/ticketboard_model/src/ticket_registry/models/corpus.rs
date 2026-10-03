//! The loaded corpus and its refusal.
//!
//! **Role:** `Corpus`, `LoadedTicket`, `Counts`, `LoadError` and `LoadResult`.
//! **Position:** produced by `crate::ticket_registry::services::corpus_loading`; read by every
//! feature and by the desktop application.
//! **Signals & state:** none; plain data.
//! **Invariants:** parents and children add up to the total; a `LoadError` names the one refusing
//! file and carries its error verbatim.

use std::{fmt, path::PathBuf};
use ticket_model::Ticket;
/// One parsed ticket plus the file it came from (the refusal / reveal surface).
#[derive(Debug)]
pub struct LoadedTicket {
    /// The parsed ticket.
    pub ticket: Ticket,
    /// The ticket file.
    pub path: PathBuf,
}

/// Footer acceptance surface: `total` equals `ls .ai/tickets/T-*.toml | wc -l` at
/// load time, and `parents + children == total` by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// Ticket files loaded.
    pub total: usize,
    /// Parent tickets (undotted ids).
    pub parents: usize,
    /// Child tickets (dotted ids).
    pub children: usize,
}

#[derive(Debug)]
/// Every ticket of the registry, in file-name order, with the counts the footer shows.
pub struct Corpus {
    /// The loaded tickets.
    pub tickets: Vec<LoadedTicket>,
    /// The ticket counts.
    pub counts: Counts,
}

/// Fail-closed load refusal: the offending file and the VERBATIM error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    /// The offending file.
    pub file: PathBuf,
    /// The verbatim read or parse error.
    pub error: String,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.display(), self.error)
    }
}

/// The corpus, or the refusal naming the first bad file.
pub type LoadResult = Result<Corpus, LoadError>;
