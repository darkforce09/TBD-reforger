//! Accumulating many checks into one exit status.
//!
//! `gate-grep.sh`'s contract left this to the caller: *"Callers decide whether to `return 1`
//! immediately or accumulate a FAIL flag."* In practice every consumer accumulated, and every one
//! of them hand-rolled the same `fails=$((fails+1))` counter and the same summary line — with the
//! usual consequence that they do not all agree on the wording or on whether a "did not run" is
//! counted separately.
//!
//! [`Report`] is that pattern, once. It keeps the two failure kinds apart in the summary, because
//! "3 violations" and "3 checks that never executed" call for completely different next actions
//! and a merged count hides the second behind the first.

use crate::verdict::Verdict;

/// Accumulates verdicts, prints each failure as it lands, and yields one exit status.
pub struct Report {
    label: String,
    ran: u32,
    failed: u32,
    did_not_run: u32,
}

impl Report {
    pub fn new(label: impl Into<String>) -> Report {
        Report {
            label: label.into(),
            ran: 0,
            failed: 0,
            did_not_run: 0,
        }
    }

    /// Record one verdict, printing its rendered failure immediately so output order follows
    /// check order even when a later check is slow.
    pub fn check(&mut self, verdict: Verdict) -> &mut Report {
        self.ran += 1;
        match verdict {
            Verdict::Held => {}
            Verdict::Failed(ref f) => {
                self.failed += 1;
                println!("{f}");
            }
            Verdict::DidNotRun(_, ref f) => {
                self.did_not_run += 1;
                println!("{f}");
            }
        }
        self
    }

    /// True only when every check ran AND held.
    pub fn clean(&self) -> bool {
        self.failed == 0 && self.did_not_run == 0
    }

    /// Print the summary and yield the four-outcome exit status.
    ///
    /// A check that did not run outranks a violation: if some inputs were never examined, the
    /// "0 violations" over the rest is not a result anyone should act on.
    pub fn finish(self) -> i32 {
        if self.clean() {
            println!("\n{}: OK — {} check(s), all held", self.label, self.ran);
            return 0;
        }
        if self.did_not_run > 0 {
            eprintln!(
                "\n{}: FAIL — {} violation(s), {} check(s) DID NOT RUN (of {})",
                self.label, self.failed, self.did_not_run, self.ran
            );
            eprintln!(
                "  A check that did not run is not a pass. Fix the missing input or absent tool"
            );
            eprintln!("  before reading anything into the checks that did complete.");
            return 2;
        }
        eprintln!(
            "\n{}: FAIL — {} violation(s) of {}",
            self.label, self.failed, self.ran
        );
        1
    }

    /// Summary counts, for tests and for callers that must map onto a legacy exit contract.
    pub fn counts(&self) -> (u32, u32, u32) {
        (self.ran, self.failed, self.did_not_run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::{Kind, NotRun};

    #[test]
    fn all_held_is_clean_and_zero() {
        let mut r = Report::new("t");
        r.check(Verdict::Held).check(Verdict::Held);
        assert!(r.clean());
        assert_eq!(r.counts(), (2, 0, 0));
        assert_eq!(r.finish(), 0);
    }

    #[test]
    fn violations_exit_one() {
        let mut r = Report::new("t");
        r.check(Verdict::Held).check(Verdict::failed("bad"));
        assert!(!r.clean());
        assert_eq!(r.finish(), 1);
    }

    #[test]
    fn a_did_not_run_outranks_violations() {
        // The whole point: 0 violations over inputs nobody read is not a pass, and a mixed run
        // must not be reported with the milder of the two codes.
        let mut r = Report::new("t");
        r.check(Verdict::failed("bad")).check(Verdict::did_not_run(
            "x",
            Kind::Ban,
            NotRun::ToolAbsent("grep".into()),
        ));
        assert_eq!(r.counts(), (2, 1, 1));
        assert_eq!(r.finish(), 2);
    }

    #[test]
    fn did_not_run_alone_still_exits_two() {
        let mut r = Report::new("t");
        r.check(Verdict::did_not_run(
            "x",
            Kind::Pin,
            NotRun::ToolAbsent("ssh".into()),
        ));
        assert_eq!(r.finish(), 2);
    }

    #[test]
    fn an_empty_report_is_clean() {
        // Deliberate: "no checks defined" is a wiring bug, but it is not this type's job to
        // detect it. `xtask verify self` (T-853 Phase 7) is what asserts every gate is reachable.
        let r = Report::new("t");
        assert!(r.clean());
    }
}
