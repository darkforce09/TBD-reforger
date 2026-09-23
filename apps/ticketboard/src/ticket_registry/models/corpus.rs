use std::{fmt, path::PathBuf};
use ticket_engine::Ticket;
/// One parsed ticket plus the file it came from (the refusal / reveal surface).
#[derive(Debug)]
pub struct LoadedTicket {
    pub ticket: Ticket,
    pub path: PathBuf,
}

/// Footer acceptance surface: `total` equals `ls .ai/tickets/T-*.toml | wc -l` at
/// load time, and `parents + children == total` by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub total: usize,
    pub parents: usize,
    pub children: usize,
}

#[derive(Debug)]
pub struct Corpus {
    pub tickets: Vec<LoadedTicket>,
    pub counts: Counts,
}

/// Fail-closed load refusal: the offending file and the VERBATIM error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub file: PathBuf,
    pub error: String,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.display(), self.error)
    }
}

pub type LoadResult = Result<Corpus, LoadError>;
