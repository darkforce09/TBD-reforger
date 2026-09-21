//! Repo-root resolution (T-915.1 §Read architecture).
//!
//! The positional CLI argument wins; otherwise walk up from the cwd looking for a
//! directory containing `.ai/tickets/`; otherwise `None` — the UI then shows the
//! full-window refusal that states both mechanisms and offers the native folder
//! picker. Pure functions, unit-tested; no egui types here.

use std::path::{Path, PathBuf};

/// True when `root` directly contains the ticket registry.
pub fn has_tickets_dir(root: &Path) -> bool {
    root.join(ticket_engine::repository::TICKETS_DIR).is_dir()
}

/// Walk from `start` upward (including `start` itself) to the filesystem root,
/// returning the first directory that contains `.ai/tickets/`.
pub fn walk_up_for_tickets(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| has_tickets_dir(dir))
        .map(Path::to_path_buf)
}

/// Resolve the repo root. The positional CLI arg wins unconditionally — even when
/// it lacks `.ai/tickets/`: the caller validates and refuses loudly instead of
/// silently falling back to discovery (the operator named that path on purpose).
pub fn resolve_repo_root(arg: Option<PathBuf>, cwd: Option<&Path>) -> Option<PathBuf> {
    if let Some(arg) = arg {
        return Some(arg);
    }
    cwd.and_then(walk_up_for_tickets)
}

/// First non-flag argument, as a path.
pub fn positional_arg<I: IntoIterator<Item = String>>(args: I) -> Option<PathBuf> {
    args.into_iter()
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from)
}

#[cfg(test)]
#[path = "tests/discovery_tests.rs"]
mod tests;
