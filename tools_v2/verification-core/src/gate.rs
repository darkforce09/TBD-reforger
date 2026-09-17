//! The four-outcome helpers — a 1:1 port of `gate-grep.sh`'s six entry points.
//!
//! | bash                | here                    |
//! |---------------------|-------------------------|
//! | `gate_ban`          | [`ban`]                 |
//! | `gate_require`      | [`require`]             |
//! | `gate_ban_str`      | [`ban_str`]             |
//! | `gate_require_str`  | [`require_str`]         |
//! | `gate_probe_file`   | [`probe_files`]         |
//! | `gate_probe_str`    | [`probe_str`]           |
//!
//! Measured 2026-08-12 across `scripts/`: 19 `gate_require`, 18 `gate_probe_file`, 11 `gate_ban`,
//! 7 `gate_probe_str`, 4 `gate_require_str`, 2 `gate_ban_str` — 61 call sites in 12 consumers.
//!
//! ── THE `probe` UPGRADE ──────────────────────────────────────────────────────────────────────
//!
//! `gate_probe_*` existed for compound conditions (`grep A && ! grep B`) where neither outcome is
//! a failure alone. Its own comment warns that such a chain *"short-circuits to clean the moment
//! one of its greps cannot run, which is the fail-open shape again in a costume"* — and then can
//! only ask the caller to remember to check for a status above 1.
//!
//! [`probe_str`] and [`probe_files`] return `Result<bool, NotRun>` instead of a raw integer, so
//! `?` propagates "did not run" automatically. The footgun bash could warn about but not prevent
//! is closed by the type.

use std::path::Path;

use crate::pattern::Pattern;
use crate::verdict::{Kind, NotRun, Verdict};

/// Read every target, failing closed on the first one that is missing or unreadable.
///
/// `_gate_files_present` in bash, except that it also does the read — because classification here
/// requires the bytes, and "I could not open it" must never reach a comparison as an empty string.
fn read_all(files: &[&Path]) -> Result<String, NotRun> {
    let mut joined = String::new();
    for f in files {
        // `is_file()` first: a directory or a dangling symlink would otherwise surface as an
        // opaque io::Error, and "the path is not a file" is the far more common real cause.
        if !f.is_file() {
            return Err(NotRun::TargetMissing(f.to_path_buf()));
        }
        match std::fs::read_to_string(f) {
            Ok(text) => {
                joined.push_str(&text);
                // Guarantee a boundary so a pattern cannot match across two files' contents —
                // `grep FILE1 FILE2` can never do that, and neither may this.
                if !joined.ends_with('\n') {
                    joined.push('\n');
                }
            }
            Err(source) => {
                return Err(NotRun::Unreadable {
                    path: f.to_path_buf(),
                    source,
                });
            }
        }
    }
    Ok(joined)
}

/// `pattern` must NOT appear in `files`.
pub fn ban(msg: &str, pattern: &Pattern, files: &[&Path]) -> Verdict {
    match read_all(files) {
        Err(cause) => Verdict::did_not_run(msg, Kind::Ban, cause),
        Ok(text) if pattern.is_match(&text) => Verdict::failed(msg),
        // No match — and, crucially, we know it holds *because the search ran*.
        Ok(_) => Verdict::Held,
    }
}

/// `pattern` MUST appear in `files`.
pub fn require(msg: &str, pattern: &Pattern, files: &[&Path]) -> Verdict {
    match read_all(files) {
        Err(cause) => Verdict::did_not_run(msg, Kind::Pin, cause),
        Ok(text) if pattern.is_match(&text) => Verdict::Held,
        Ok(_) => Verdict::failed(msg),
    }
}

/// `pattern` must NOT appear in an in-memory subject.
///
/// No [`NotRun`] path exists: the subject is already in hand, so there is nothing that could fail
/// to be examined. Returning a `Verdict` anyway keeps every gate in one vocabulary.
pub fn ban_str(msg: &str, pattern: &Pattern, subject: &str) -> Verdict {
    if pattern.is_match(subject) {
        Verdict::failed(msg)
    } else {
        Verdict::Held
    }
}

/// `pattern` MUST appear in an in-memory subject.
pub fn require_str(msg: &str, pattern: &Pattern, subject: &str) -> Verdict {
    if pattern.is_match(subject) {
        Verdict::Held
    } else {
        Verdict::failed(msg)
    }
}

/// Compound-condition escape hatch over an in-memory subject.
///
/// Infallible today, but returns `Result` for symmetry with [`probe_files`] so that a caller
/// written against one can be switched to the other without restructuring its `?`s.
pub fn probe_str(pattern: &Pattern, subject: &str) -> Result<bool, NotRun> {
    Ok(pattern.is_match(subject))
}

/// Compound-condition escape hatch over files. `?` propagates "did not run".
pub fn probe_files(pattern: &Pattern, files: &[&Path]) -> Result<bool, NotRun> {
    Ok(pattern.is_match(&read_all(files)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A scratch file that cleans itself up. Avoids a dev-dependency for six tests.
    struct Tmp(std::path::PathBuf);
    impl Tmp {
        fn new(name: &str, body: &str) -> Tmp {
            let mut p = std::env::temp_dir();
            p.push(format!("tbd-gate-{}-{}", std::process::id(), name));
            let mut f = std::fs::File::create(&p).unwrap();
            f.write_all(body.as_bytes()).unwrap();
            Tmp(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn pat(p: &str) -> Pattern {
        Pattern::regex(p).unwrap()
    }

    #[test]
    fn ban_holds_when_absent() {
        let f = Tmp::new("ban-clean", "all good here\n");
        assert!(matches!(
            ban("no evil", &pat("evil"), &[f.path()]),
            Verdict::Held
        ));
    }

    #[test]
    fn ban_fails_when_present() {
        let f = Tmp::new("ban-dirty", "some evil here\n");
        let v = ban("no evil", &pat("evil"), &[f.path()]);
        assert!(matches!(v, Verdict::Failed(_)));
        assert_eq!(v.to_string(), "FAIL: no evil");
    }

    #[test]
    fn require_fails_when_absent() {
        let f = Tmp::new("req-missing", "nothing relevant\n");
        assert!(matches!(
            require("must pin", &pat("PINNED"), &[f.path()]),
            Verdict::Failed(_)
        ));
    }

    #[test]
    fn require_holds_when_present() {
        let f = Tmp::new("req-ok", "PINNED = 1\n");
        assert!(matches!(
            require("must pin", &pat("PINNED"), &[f.path()]),
            Verdict::Held
        ));
    }

    /// THE DEFECT THIS CRATE EXISTS FOR. A missing target must never read as a clean ban.
    #[test]
    fn missing_target_is_did_not_run_not_held() {
        let v = ban(
            "no evil",
            &pat("evil"),
            &[Path::new("/nonexistent/tbd-gate/nope")],
        );
        assert!(matches!(v, Verdict::DidNotRun(NotRun::TargetMissing(_), _)));
        assert_ne!(v.into_exit(), 0, "a check that did not run must not exit 0");
    }

    #[test]
    fn missing_target_on_require_is_also_did_not_run() {
        let v = require(
            "must pin",
            &pat("x"),
            &[Path::new("/nonexistent/tbd-gate/nope")],
        );
        assert!(matches!(v, Verdict::DidNotRun(NotRun::TargetMissing(_), _)));
    }

    #[test]
    fn a_directory_target_is_missing_not_unreadable() {
        let v = ban("no evil", &pat("evil"), &[Path::new("/tmp")]);
        assert!(matches!(v, Verdict::DidNotRun(NotRun::TargetMissing(_), _)));
    }

    #[test]
    fn one_missing_among_many_fails_the_whole_check() {
        let f = Tmp::new("multi", "clean\n");
        let v = ban(
            "no evil",
            &pat("evil"),
            &[f.path(), Path::new("/nonexistent/tbd-gate/x")],
        );
        assert!(matches!(v, Verdict::DidNotRun(NotRun::TargetMissing(_), _)));
    }

    #[test]
    fn patterns_cannot_match_across_a_file_boundary() {
        // `grep A B` can never match text spanning the two files; nor may we.
        let a = Tmp::new("join-a", "prefix");
        let b = Tmp::new("join-b", "suffix\n");
        let v = ban("no join", &pat("prefixsuffix"), &[a.path(), b.path()]);
        assert!(matches!(v, Verdict::Held));
    }

    #[test]
    fn str_helpers_need_no_files() {
        assert!(matches!(ban_str("no x", &pat("x"), "clean"), Verdict::Held));
        assert!(matches!(
            ban_str("no x", &pat("x"), "has x"),
            Verdict::Failed(_)
        ));
        assert!(matches!(
            require_str("want x", &pat("x"), "has x"),
            Verdict::Held
        ));
        assert!(matches!(
            require_str("want x", &pat("x"), "clean"),
            Verdict::Failed(_)
        ));
    }

    #[test]
    fn probe_propagates_did_not_run_instead_of_short_circuiting_clean() {
        // The compound-condition footgun gate_probe_* could only warn about.
        let got: Result<bool, NotRun> = probe_files(&pat("x"), &[Path::new("/nonexistent/tbd/x")]);
        assert!(matches!(got, Err(NotRun::TargetMissing(_))));
    }

    #[test]
    fn probe_reports_both_true_and_false_when_it_ran() {
        let f = Tmp::new("probe", "alpha\n");
        assert!(probe_files(&pat("alpha"), &[f.path()]).unwrap());
        assert!(!probe_files(&pat("beta"), &[f.path()]).unwrap());
        assert!(probe_str(&pat("alpha"), "alpha").unwrap());
    }
}
