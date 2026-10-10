//! The wave-close ledger: the `wave N CLOSED` commit subjects in git history.
//!
//! **Role:** recognises a close subject ([`wave_close_subject_ok`]), reads the number a commit
//! claims ([`wave_close_number_in`]), finds the revert that disavows a close
//! ([`wave_close_disavowed_in`]) and derives the newest standing close ([`newest_close_base`]);
//! [`git_in`] and [`git_in_lossy`] run the `git` reads.
//! **Position:** the platform wave driver's base derivation, its close ceremony and the
//! preflight read the marker ledger through these rules; the ceremony also checks that the ticket
//! manager's wave base agrees with [`newest_close_base`].
//! **Signals & state:** none in memory; every call runs `git` in the given checkout.
//! **Invariants:** the subject is the authority — `git log --grep` only prefilters, and each
//! match is confirmed by [`wave_close_subject_ok`]; a close that a later commit reverts with
//! git's exact revert trailer does not count; a shallow clone refuses the derivation, because
//! the ledger must be complete.

use process_runner::Run;
use std::path::Path;

/// Git prefilter for anchored wave-close subjects; the subject parser confirms each match.
pub(crate) const WAVE_CLOSE_MARKER_RE: &str = r"^wave [0-9]+ CLOSED([:]|$| —| -)";

/// Accept the anchored `wave N CLOSED` subject and its supported suffix delimiters.
pub(crate) fn wave_close_subject_ok(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("wave ") else {
        return false;
    };
    // The number runs up to the first space, or to the end when there is none.
    let n = match rest.find(' ') {
        Some(i) => &rest[..i],
        None => rest,
    };
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let rest = &rest[n.len()..];
    rest == " CLOSED"
        || rest.starts_with(" CLOSED:")
        || rest.starts_with(" CLOSED —")
        || rest.starts_with(" CLOSED -")
}

/// Read the claimed wave number from a valid close subject in the supplied repository.
pub(crate) fn wave_close_number_in(root: &Path, rev: &str) -> Option<i64> {
    let s = git_in(root, &["log", "-1", "--format=%s", rev])?;
    if !wave_close_subject_ok(&s) {
        return None;
    }
    let rest = s.strip_prefix("wave ")?;
    let n = match rest.find(' ') {
        Some(i) => &rest[..i],
        None => rest,
    };
    n.parse().ok()
}

/// The first commit after `rev` up to `HEAD` whose message carries git's exact revert trailer
/// for `rev` (`This reverts commit <full sha>.`); `None` when the close stands or `rev` does not
/// resolve.
pub(crate) fn wave_close_disavowed_in(root: &Path, rev: &str) -> Option<String> {
    let full = git_in(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{rev}^{{commit}}"),
        ],
    )
    .filter(|s| !s.is_empty())?;
    let needle = format!("This reverts commit {full}.");
    let list = git_in_lossy(
        root,
        &[
            "rev-list",
            "--fixed-strings",
            &format!("--grep={needle}"),
            &format!("{full}..HEAD"),
        ],
    );
    for c in list.lines().filter(|l| !l.is_empty()) {
        let body = git_in(root, &["log", "-1", "--format=%B", c]).unwrap_or_default();
        if body.contains(&needle) {
            return Some(c.to_string());
        }
    }
    None
}

/// Read Git stdout at an explicit repository root; return None on execution or status failure.
pub(crate) fn git_in(root: &Path, args: &[&str]) -> Option<String> {
    let out = Run::new("git").args(args).cwd(root).output().ok()?;
    if out.code != 0 {
        return None;
    }
    Some(out.stdout.trim_end_matches('\n').to_string())
}

/// Read Git stdout at an explicit repository root, retaining stdout regardless of exit status.
pub(crate) fn git_in_lossy(root: &Path, args: &[&str]) -> String {
    match Run::new("git").args(args).cwd(root).output() {
        Ok(out) => out.stdout.trim_end_matches('\n').to_string(),
        Err(_) => String::new(),
    }
}

/// Return the newest reachable, non-disavowed close number, including HEAD.
/// Shallow history is refused because the complete marker ledger is required.
/// A repository with no reachable close returns None.
pub(crate) fn newest_close_base(root: &Path) -> Result<Option<i64>, String> {
    if git_in_lossy(root, &["rev-parse", "--is-shallow-repository"]).trim() == "true" {
        return Err(
            "shallow clone: the close-marker ledger is unreadable — fetch full history \
             (fetch-depth: 0 in CI) before reading the close ledger"
                .into(),
        );
    }
    let list = git_in_lossy(
        root,
        &[
            "rev-list",
            "--extended-regexp",
            &format!("--grep={WAVE_CLOSE_MARKER_RE}"),
            "HEAD",
        ],
    );
    for sha in list.lines().filter(|l| !l.is_empty()) {
        // `--grep` matches the whole message; the subject is the authority, confirmed through
        // `wave_close_number_in`.
        let Some(n) = wave_close_number_in(root, sha) else {
            continue;
        };
        if wave_close_disavowed_in(root, sha).is_some() {
            continue;
        }
        return Ok(Some(n));
    }
    Ok(None)
}
