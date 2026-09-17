//! The four-outcome verdict — the whole reason this crate exists.
//!
//! ── WHY A TYPE AND NOT A BOOL ────────────────────────────────────────────────────────────────
//!
//! `scripts/mod/lib/gate-grep.sh` states the problem exactly: *"a boolean cannot carry four
//! outcomes"*. Written as `if grep PAT FILE; then fail; fi`, four distinct results collapse into
//! two:
//!
//!   exit 0    match found          -> ban violated       -> correctly FAILED
//!   exit 1    no match             -> ban holds          -> correctly passed
//!   exit 2    TARGET FILE MISSING  -> check never ran    -> printed OK
//!   exit 127  SEARCH TOOL ABSENT   -> check never ran    -> printed OK
//!
//! The last two are this program's signature defect — *a tool reporting success over an input it
//! never examined* — and they are what kept `verify-no-python` green over a `rg: command not
//! found` for four waves (T-620).
//!
//! In bash the fix is discipline: every helper must remember to stat its targets and inspect the
//! raw exit status. Discipline did not propagate — T-216 fixed this inline in one gate and every
//! gate written afterwards was born with the same two holes, which is why gate-grep.sh was
//! extracted as a library in the first place.
//!
//! Here it is not discipline. [`Verdict`] has no `From<bool>`, no `Deref<Target = bool>`, no
//! `is_ok()`, and no `PartialEq<bool>`. There is no expression that turns it into a two-way
//! branch by accident. A caller must `match`, and `match` is exhaustive, so **"the check did not
//! run" cannot be silently folded into "the check passed" — that is now a compile error rather
//! than a code review.** Adding a variant to [`NotRun`] later breaks every incomplete `match` in
//! the workspace, which is the propagation mechanism gate-grep.sh wanted and could not have.
//!
//! ── ON OUTPUT COMPATIBILITY ──────────────────────────────────────────────────────────────────
//!
//! [`Finding`] renders byte-for-byte what the bash helpers printed, down to the six-space
//! continuation indent and the em-dash. Ports are accepted by diffing old and new stdout, and
//! `wave.sh` scrapes these logs — so the text is a contract, not decoration.

use std::fmt;
use std::path::PathBuf;

/// Which side of the check failed, for the one line of prose that differs between them.
///
/// `gate-grep.sh` threaded this as a bare string (`"ban"` / `"pin"`) through `_gate_files_present`
/// and `_gate_tool_fail`. It is an enum here for the usual reason: there is no third spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The pattern must NOT appear.
    Ban,
    /// The pattern MUST appear.
    Pin,
}

impl Kind {
    /// The noun the bash helpers interpolated into "The {what} could not run."
    pub fn noun(self) -> &'static str {
        match self {
            Kind::Ban => "ban",
            Kind::Pin => "pin",
        }
    }
}

/// Why a check could not reach a verdict.
///
/// Every variant means the same thing to a caller — *the input was never examined* — but they
/// send a reader to three different places, which is why `gate-grep.sh` refused to merge them and
/// why they stay separate here.
#[derive(Debug)]
pub enum NotRun {
    /// A target path does not exist, or is not a regular file.
    ///
    /// `_gate_files_present`: "A moved or deleted file must not read as a clean result."
    TargetMissing(PathBuf),
    /// A target exists but could not be read (permissions, I/O error, a broken LFS pointer).
    ///
    /// Distinct from [`NotRun::TargetMissing`] because "it is not there" and "it is there and I
    /// was not allowed to look" are different bugs with different fixes.
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A required external program is not on `PATH`.
    ///
    /// The honest form of `grep`'s exit 127. Kept as a variant even though this crate no longer
    /// shells out to `grep`, because the process helpers in [`crate::proc`] still spawn `git`,
    /// `ssh`, `cargo` and the game binaries.
    ToolAbsent(String),
    /// A program ran and failed in a way that means the check did not complete — as opposed to
    /// completing and reporting a violation.
    ToolError {
        tool: String,
        status: i32,
        stderr: String,
    },
    /// A child died on a signal. **Never a `Failed`.**
    ///
    /// `ExitStatus::code()` is `None` here, and the bash idiom `cmd || rc=$?` turns that into
    /// `128+n`, which a `case` arm then reads as an ordinary failure. Under eight parallel
    /// worktrees the OOM killer is a routine visitor, and "the kernel shot the gate" must not be
    /// reported as "the gate found a problem".
    Signalled { tool: String, signal: i32 },
    /// A command exceeded its deadline. See [`crate::proc`] on why this kills the process group.
    Timeout { tool: String, secs: u64 },
}

impl NotRun {
    /// The second line of the rendered failure — the one that names the cause.
    fn explain(&self, kind: Kind) -> String {
        let what = kind.noun();
        match self {
            NotRun::TargetMissing(_) => format!(
                "The {what} could not run. A moved or deleted file must not read as a clean result."
            ),
            NotRun::Unreadable { source, .. } => format!(
                "Unreadable: {source}. Refusing to report OK on a {what} that did not execute."
            ),
            NotRun::ToolAbsent(tool) => format!(
                "`{tool}` is ABSENT. Refusing to report OK on a {what} that did not execute."
            ),
            NotRun::ToolError { tool, status, .. } => format!(
                "`{tool}` exited {status} (read/pattern error). Refusing to report OK on a {what} that did not execute."
            ),
            NotRun::Signalled { tool, signal } => format!(
                "`{tool}` was killed by signal {signal} — the process died, it did not report. Refusing to report OK on a {what} that did not execute."
            ),
            NotRun::Timeout { tool, secs } => format!(
                "`{tool}` exceeded {secs}s and was killed. Refusing to report OK on a {what} that did not execute."
            ),
        }
    }

    /// The trailing clause on the headline, mirroring bash's `— target file missing: <path>`.
    fn headline_suffix(&self) -> String {
        match self {
            NotRun::TargetMissing(p) => format!(" — target file missing: {}", p.display()),
            NotRun::Unreadable { path, .. } => format!(" — unreadable target: {}", path.display()),
            NotRun::ToolAbsent(tool) => format!(" — {tool} not found"),
            NotRun::ToolError { tool, status, .. } => format!(" — {tool} exited {status}"),
            NotRun::Signalled { tool, signal } => format!(" — {tool} killed by signal {signal}"),
            NotRun::Timeout { tool, secs } => format!(" — {tool} timed out after {secs}s"),
        }
    }
}

/// A rendered failure: one headline plus zero or more six-space-indented continuation lines.
///
/// The indent is `gate-grep.sh`'s, preserved so ported gates diff clean against the scripts they
/// replace.
#[derive(Debug, Clone)]
pub struct Finding {
    pub headline: String,
    pub detail: Vec<String>,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FAIL: {}", self.headline)?;
        for line in &self.detail {
            write!(f, "\n      {line}")?;
        }
        Ok(())
    }
}

/// The outcome of one check.
///
/// See the module docs. The `must_use` message is deliberately the failure mode it prevents.
#[must_use = "a Verdict that is never inspected is a check that never ran — match it or pass it to Report::check"]
#[derive(Debug)]
pub enum Verdict {
    /// The check ran and the invariant holds.
    Held,
    /// The check ran and found a violation.
    Failed(Finding),
    /// The check could not run. Structured cause kept alongside the rendered text so tests can
    /// assert on the *reason* rather than string-matching the prose.
    DidNotRun(NotRun, Finding),
}

impl Verdict {
    /// Build a `Failed` from a bare message, matching bash's plain `FAIL: $msg`.
    pub fn failed(msg: impl Into<String>) -> Verdict {
        Verdict::Failed(Finding {
            headline: msg.into(),
            detail: Vec::new(),
        })
    }

    /// Build a `DidNotRun`, rendering the two-line bash form for the given cause and kind.
    pub fn did_not_run(msg: impl Into<String>, kind: Kind, cause: NotRun) -> Verdict {
        let finding = Finding {
            headline: format!("{}{}", msg.into(), cause.headline_suffix()),
            detail: vec![cause.explain(kind)],
        };
        Verdict::DidNotRun(cause, finding)
    }

    /// Four-outcome exit status: `Held` 0, `Failed` 1, `DidNotRun` **2**.
    ///
    /// Prefer this for new gates — a distinct code for "did not run" is the point of the whole
    /// exercise, and CI can then tell a real violation from a broken checkout.
    pub fn into_exit(self) -> i32 {
        match self {
            Verdict::Held => 0,
            Verdict::Failed(_) => 1,
            Verdict::DidNotRun(..) => 2,
        }
    }

    /// Legacy two-outcome status: `Held` 0, everything else 1.
    ///
    /// **Only for ports that must diff byte-identically against a bash script whose exit contract
    /// callers already depend on** — `gate_ban`/`gate_require` returned 1 for both failure kinds.
    /// Named the long way round so that choosing it is visible in review rather than a `!held()`
    /// nobody notices. New code wants [`Verdict::into_exit`].
    pub fn into_exit_legacy_binary(self) -> i32 {
        match self {
            Verdict::Held => 0,
            Verdict::Failed(_) | Verdict::DidNotRun(..) => 1,
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::Held => Ok(()),
            Verdict::Failed(finding) | Verdict::DidNotRun(_, finding) => finding.fmt(f),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_bare_failure_like_bash() {
        let v = Verdict::failed("agent must never eval a request");
        assert_eq!(v.to_string(), "FAIL: agent must never eval a request");
    }

    #[test]
    fn renders_missing_target_like_bash() {
        // gate-grep.sh `_gate_files_present`, byte-for-byte including the six-space indent.
        let v = Verdict::did_not_run(
            "socket must be 0600",
            Kind::Pin,
            NotRun::TargetMissing(PathBuf::from("scripts/gone.sh")),
        );
        assert_eq!(
            v.to_string(),
            "FAIL: socket must be 0600 — target file missing: scripts/gone.sh\n      \
             The pin could not run. A moved or deleted file must not read as a clean result."
        );
    }

    #[test]
    fn ban_and_pin_differ_only_in_the_noun() {
        // Only TargetMissing uses the "The {noun} could not run." phrasing; the tool-failure
        // causes end with "on a {noun} that did not execute", matching `_gate_tool_fail`.
        let ban = Verdict::did_not_run("m", Kind::Ban, NotRun::ToolAbsent("grep".into()));
        let pin = Verdict::did_not_run("m", Kind::Pin, NotRun::ToolAbsent("grep".into()));
        assert!(
            ban.to_string().ends_with("on a ban that did not execute."),
            "{ban}"
        );
        assert!(
            pin.to_string().ends_with("on a pin that did not execute."),
            "{pin}"
        );

        let ban_missing =
            Verdict::did_not_run("m", Kind::Ban, NotRun::TargetMissing(PathBuf::from("x")));
        let pin_missing =
            Verdict::did_not_run("m", Kind::Pin, NotRun::TargetMissing(PathBuf::from("x")));
        assert!(ban_missing.to_string().contains("The ban could not run"));
        assert!(pin_missing.to_string().contains("The pin could not run"));
    }

    #[test]
    fn exit_codes_separate_did_not_run_from_failed() {
        assert_eq!(Verdict::Held.into_exit(), 0);
        assert_eq!(Verdict::failed("x").into_exit(), 1);
        let dnr = Verdict::did_not_run("x", Kind::Ban, NotRun::ToolAbsent("grep".into()));
        assert_eq!(
            dnr.into_exit(),
            2,
            "did-not-run must be distinguishable from a violation"
        );
    }

    #[test]
    fn legacy_binary_exit_matches_gate_grep() {
        // gate_ban/gate_require returned 1 for BOTH failure kinds; ports pin that.
        assert_eq!(Verdict::Held.into_exit_legacy_binary(), 0);
        assert_eq!(Verdict::failed("x").into_exit_legacy_binary(), 1);
        let dnr = Verdict::did_not_run("x", Kind::Pin, NotRun::ToolAbsent("grep".into()));
        assert_eq!(dnr.into_exit_legacy_binary(), 1);
    }

    #[test]
    fn signal_death_is_did_not_run_never_failed() {
        // THE CONTRACT bash gets wrong. `cmd || rc=$?` yields 128+n and a `case` arm reads it as
        // an ordinary failure; under eight worktrees the OOM killer makes this routine.
        let v = Verdict::did_not_run(
            "world boot",
            Kind::Pin,
            NotRun::Signalled {
                tool: "ArmaReforgerServer".into(),
                signal: 9,
            },
        );
        assert!(matches!(
            v,
            Verdict::DidNotRun(NotRun::Signalled { signal: 9, .. }, _)
        ));
        assert_eq!(v.into_exit(), 2);
    }

    #[test]
    fn held_renders_nothing() {
        assert_eq!(Verdict::Held.to_string(), "");
    }
}
