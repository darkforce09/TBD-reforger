//! The fail-closed load of every ticket file.
//!
//! **Role:** `load_corpus`, which parses every `T-*.toml` under `.ai/tickets/` with
//! `ticket_model::parse_ticket_toml`, and `is_child_id`.
//! **Position:** called by `crate::application_state::background_loading` on the load thread and by
//! the test fixtures.
//! **Signals & state:** none; reads files and returns a `LoadResult`.
//! **Invariants:** a load is complete or refused, never partial, and never writes a ticket file;
//! files are read in name order so the first failure is deterministic.

pub use crate::ticket_registry::models::corpus::*;
use repository_layout::TICKETS_DIR;
use std::{
    fs,
    path::{Path, PathBuf},
};
use ticket_model::TicketId;
/// Child = dotted id (`T-<n>.<m>`); parent = undotted (`T-<n>`).
pub fn is_child_id(id: &TicketId) -> bool {
    id.as_str().contains('.')
}

/// The `T-*.toml` files under `dir`, sorted by name so the first failure is
/// deterministic. Mirrors the `T-*.toml` shell glob the acceptance line counts with
/// (`ROOT`, `schema.json`, `queue.json`, … never match).
fn ticket_files(dir: &Path) -> Result<Vec<PathBuf>, LoadError> {
    let err = |e: std::io::Error| LoadError {
        file: dir.to_path_buf(),
        error: e.to_string(),
    };
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(err)? {
        let path = entry.map_err(err)?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("T-") && name.ends_with(".toml") && path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Load every ticket under `repo_root/.ai/tickets/`. The first unreadable or
/// unparsable file aborts the whole load.
pub fn load_corpus(repo_root: &Path) -> LoadResult {
    let dir = repo_root.join(TICKETS_DIR);
    if !dir.is_dir() {
        return Err(LoadError {
            file: dir.clone(),
            error: format!("no {TICKETS_DIR}/ directory under {}", repo_root.display()),
        });
    }
    let files = ticket_files(&dir)?;
    let mut tickets = Vec::with_capacity(files.len());
    let mut parents = 0usize;
    let mut children = 0usize;
    for path in files {
        let text = fs::read_to_string(&path).map_err(|e| LoadError {
            file: path.clone(),
            error: e.to_string(),
        })?;
        let ticket = ticket_model::parse_ticket_toml(&text).map_err(|error| LoadError {
            file: path.clone(),
            error,
        })?;
        if is_child_id(ticket.id()) {
            children += 1;
        } else {
            parents += 1;
        }
        tickets.push(LoadedTicket { ticket, path });
    }
    Ok(Corpus {
        counts: Counts {
            total: tickets.len(),
            parents,
            children,
        },
        tickets,
    })
}

#[cfg(test)]
#[path = "tests/corpus_loading.rs"]
mod tests;
