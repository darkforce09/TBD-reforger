//! Repo-root resolution (T-915.1 §Read architecture).
//!
//! The positional CLI argument wins; otherwise walk up from the cwd looking for a
//! directory containing `.ai/tickets/`; otherwise `None` — the UI then shows the
//! full-window refusal that states both mechanisms and offers the native folder
//! picker. Pure functions, unit-tested; no egui types here.

use std::path::{Path, PathBuf};

/// The registry directory every mechanism looks for, relative to a repo root.
pub const TICKETS_SUBDIR: &str = ".ai/tickets";

/// True when `root` directly contains `.ai/tickets/`.
pub fn has_tickets_dir(root: &Path) -> bool {
    root.join(TICKETS_SUBDIR).is_dir()
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
mod tests {
    use super::*;
    use crate::testutil::Scratch;
    use std::fs;

    fn mk_repo(root: &Path) {
        fs::create_dir_all(root.join(TICKETS_SUBDIR)).unwrap();
    }

    #[test]
    fn arg_wins_over_ancestor() {
        let s = Scratch::new("arg-wins");
        let repo_a = s.path().join("a");
        mk_repo(&repo_a);
        let repo_b = s.path().join("b");
        mk_repo(&repo_b);
        let cwd = repo_b.join("deep/inside");
        fs::create_dir_all(&cwd).unwrap();
        // cwd sits inside repo_b, but the explicit arg (repo_a) wins.
        let got = resolve_repo_root(Some(repo_a.clone()), Some(&cwd));
        assert_eq!(got, Some(repo_a));
    }

    #[test]
    fn arg_wins_even_when_invalid() {
        let s = Scratch::new("arg-invalid");
        let repo = s.path().join("repo");
        mk_repo(&repo);
        let bogus = s.path().join("not-a-repo");
        fs::create_dir_all(&bogus).unwrap();
        let cwd = repo.join("sub");
        fs::create_dir_all(&cwd).unwrap();
        // No silent fallback to the valid ancestor: the caller refuses instead.
        let got = resolve_repo_root(Some(bogus.clone()), Some(&cwd));
        assert_eq!(got, Some(bogus.clone()));
        assert!(!has_tickets_dir(&bogus));
    }

    #[test]
    fn ancestor_found_from_nested_cwd() {
        let s = Scratch::new("walk-up");
        let repo = s.path().join("repo");
        mk_repo(&repo);
        let cwd = repo.join("apps/ticketboard/src");
        fs::create_dir_all(&cwd).unwrap();
        assert_eq!(resolve_repo_root(None, Some(&cwd)), Some(repo.clone()));
        // The repo root itself also resolves (ancestors() includes self).
        assert_eq!(walk_up_for_tickets(&repo), Some(repo));
    }

    #[test]
    fn nothing_found_is_none() {
        let s = Scratch::new("none");
        let cwd = s.path().join("plain/dir");
        fs::create_dir_all(&cwd).unwrap();
        assert_eq!(resolve_repo_root(None, Some(&cwd)), None);
        assert_eq!(resolve_repo_root(None, None), None);
    }

    #[test]
    fn positional_arg_skips_flags() {
        let args = vec!["--verbose".to_string(), "/some/repo".to_string()];
        assert_eq!(positional_arg(args), Some(PathBuf::from("/some/repo")));
        assert_eq!(positional_arg(Vec::new()), None);
    }
}
