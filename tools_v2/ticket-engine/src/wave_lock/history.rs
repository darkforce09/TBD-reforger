//! Shared Git-history rules for ticket wave numbering and platform verification.

use std::path::Path;

/// Git prefilter for anchored wave-close subjects; the subject parser confirms each match.
pub const WAVE_CLOSE_MARKER_RE: &str = r"^wave [0-9]+ CLOSED([:]|$| —| -)";

/// Accept the anchored `wave N CLOSED` subject and its supported suffix delimiters.
pub fn wave_close_subject_ok(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("wave ") else {
        return false;
    };
    // `n="${rest%% *}"` — up to the first space, or all of it when there is none.
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
pub fn wave_close_number_in(root: &Path, rev: &str) -> Option<i64> {
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

/// Find a later commit containing Git’s exact revert trailer for this close commit.
pub fn wave_close_disavowed_in(root: &Path, rev: &str) -> Option<String> {
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
pub fn git_in(root: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string(),
    )
}

/// Read Git stdout at an explicit repository root, retaining stdout regardless of exit status.
pub fn git_in_lossy(root: &Path, args: &[&str]) -> String {
    match std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
    {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string(),
        Err(_) => String::new(),
    }
}

/// Return the newest reachable, non-disavowed close number, including HEAD.
/// Shallow history is refused because the complete marker ledger is required.
/// A repository with no reachable close returns None.
pub fn newest_close_base(root: &Path) -> Result<Option<i64>, String> {
    if git_in_lossy(root, &["rev-parse", "--is-shallow-repository"]).trim() == "true" {
        return Err(
            "shallow clone: the close-marker ledger is unreadable — fetch full history \
             (fetch-depth: 0 in CI) before wave repack/check"
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
        // git's --grep matches the WHOLE message; the subject is the authority — same
        // confirm-then-use shape as prev_wave_close, sharing wave_close_subject_ok through
        // wave_close_number_in.
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

/// Return the highest reachable, non-disavowed close claim.
/// This can differ from the newest claim when concurrent programs close out of numerical order.
/// Reverted markers do not consume labels; shallow history is refused.
pub fn max_close_claim(root: &Path) -> Result<Option<i64>, String> {
    if git_in_lossy(root, &["rev-parse", "--is-shallow-repository"]).trim() == "true" {
        return Err(
            "shallow clone: the close-marker ledger is unreadable — fetch full history              (fetch-depth: 0 in CI) before wave repack/check"
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
    let mut high: Option<i64> = None;
    for sha in list.lines().filter(|l| !l.is_empty()) {
        let Some(n) = wave_close_number_in(root, sha) else {
            continue;
        };
        if wave_close_disavowed_in(root, sha).is_some() {
            continue;
        }
        if high.map(|h| n > h).unwrap_or(true) {
            high = Some(n);
        }
    }
    Ok(high)
}
