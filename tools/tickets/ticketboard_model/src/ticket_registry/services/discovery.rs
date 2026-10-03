//! Repo-root resolution.
//!
//! The positional CLI argument wins; otherwise `repository_layout::find_repository_root_from`
//! walks up from the cwd to the folder holding `.ai/tickets/ROOT`; otherwise `None` — the UI
//! then shows the
//! full-window refusal that states both mechanisms and offers the native folder
//! picker. Pure functions, unit-tested; no egui types here.

use std::path::{Path, PathBuf};

/// True when `root` directly contains the ticket registry.
pub fn has_tickets_dir(root: &Path) -> bool {
    root.join(repository_layout::TICKETS_DIR).is_dir()
}

/// Resolve the repo root. The positional CLI arg wins unconditionally — even when
/// it lacks `.ai/tickets/`: the caller validates and refuses loudly instead of
/// silently falling back to discovery (the operator named that path on purpose).
pub fn resolve_repo_root(arg: Option<PathBuf>, cwd: Option<&Path>) -> Option<PathBuf> {
    if let Some(arg) = arg {
        return Some(arg);
    }
    cwd.and_then(|cwd| repository_layout::find_repository_root_from(cwd).ok())
}

/// First non-flag argument, as a path.
pub fn positional_arg<I: IntoIterator<Item = String>>(args: I) -> Option<PathBuf> {
    args.into_iter()
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from)
}

#[cfg(test)]
#[path = "tests/discovery.rs"]
mod tests;
