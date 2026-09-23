//! The documentation gates: README coverage with its Contents check, Markdown placement with its
//! size limit, and the link check.
//!
//! **Role:** holds one module per gate and the machinery every gate shares: the tracked-file tree
//! ([`tracked_tree`]), the `--path` scope ([`gate_scope`]), the repository regions
//! ([`path_regions`]), fenced-block recognition ([`markdown_fences`]), and [`GateRun`], which
//! carries a gate's verdicts to [`verification_core::Report`].
//!
//! **Position:** `cargo xtask verify readme-coverage`, `cargo xtask verify markdown-placement`
//! and `cargo xtask verify link-check` reach [`readme_coverage::verify_readme_coverage`],
//! [`markdown_placement::verify_markdown_placement`] and [`link_check::verify_link_check`]
//! through the verify dispatcher. Every path a gate judges comes from `git ls-files`, and every
//! region it applies comes from [`crate::core::repository_layout::documentation`].
//!
//! **Signals & state:** none held; a run lists the tracked files once, judges them, prints its
//! verdicts and returns its exit status.
//!
//! **Invariants:** a gate that could not list the tracked files, could not read a file it judges,
//! or whose scope selects nothing reports "did not run" (exit 2), never a pass; exit 1 means at
//! least one judged item broke a rule; exit 0 means every judged item held. A gate is one module
//! that builds a [`GateRun`] from [`prepare`]'s tree and scope, so another gate registers here the
//! same way.

pub(crate) mod link_check;
pub(crate) mod markdown_placement;
pub(crate) mod readme_coverage;

mod gate_scope;
mod markdown_fences;
mod path_regions;
mod tracked_tree;

#[cfg(test)]
#[path = "tests/fixture_checkout.rs"]
mod fixture_checkout;

use std::path::Path;

use verification_core::{Kind, NotRun, Report, Verdict};

use gate_scope::GateScope;
use tracked_tree::TrackedTree;

/// What one run of a documentation gate concluded, in print order.
struct GateRun {
    /// The name the report's summary line carries.
    label: &'static str,
    /// Lines printed before the verdicts: what the gate judges and over which scope.
    header: Vec<String>,
    /// One verdict per judged item, in the order the gate judged them.
    verdicts: Vec<Verdict>,
    /// Lines printed after the verdicts: each rule's totals.
    totals: Vec<String>,
}

impl GateRun {
    /// A run with a header and nothing judged yet.
    fn new(label: &'static str, header: Vec<String>) -> GateRun {
        GateRun {
            label,
            header,
            verdicts: Vec::new(),
            totals: Vec::new(),
        }
    }

    /// A run that judged nothing, carrying the one verdict that says why.
    fn stopped(label: &'static str, header: Vec<String>, verdict: Verdict) -> GateRun {
        GateRun {
            verdicts: vec![verdict],
            ..GateRun::new(label, header)
        }
    }

    /// Print the header, every verdict through the shared report, the totals and the summary,
    /// and yield the exit status: 0 when every verdict held, 1 on a violation, 2 when any check
    /// did not run.
    fn print(self) -> u8 {
        for line in &self.header {
            println!("{line}");
        }
        let mut report = Report::new(self.label);
        for verdict in self.verdicts {
            report.check(verdict);
        }
        for line in &self.totals {
            println!("{line}");
        }
        match report.finish() {
            0 => 0,
            1 => 1,
            _ => 2,
        }
    }
}

/// Running counts for one rule of a gate.
#[derive(Debug, Default)]
struct Tally {
    /// Items the rule judged.
    judged: usize,
    /// Judged items that broke the rule.
    failed: usize,
    /// Judged items that could not be read.
    unread: usize,
}

impl Tally {
    /// Count one judged item by its verdict.
    fn count(&mut self, verdict: &Verdict) {
        self.judged += 1;
        match verdict {
            Verdict::Held => {}
            Verdict::Failed(_) => self.failed += 1,
            Verdict::DidNotRun(..) => self.unread += 1,
        }
    }
}

/// The header line that names a run's scope and how many files the listing held.
fn scope_line(scope: &GateScope, tree: &TrackedTree) -> String {
    format!(
        "    scope: {}; git listed {} tracked file(s)",
        scope.describe(),
        tree.file_count()
    )
}

/// The tracked tree and the resolved scope a gate judges, or the stopped run that says why the
/// gate cannot judge anything.
///
/// A listing that failed, a listing that holds no file, and a `--path` value the tree refuses
/// are all "did not run": the gate never examined the files the operator asked about.
fn prepare(
    label: &'static str,
    kind: Kind,
    repo_root: &Path,
    listing: Result<TrackedTree, NotRun>,
    scope_values: &[String],
) -> Result<(TrackedTree, GateScope), GateRun> {
    let header = || vec![format!("==> {label}")];
    let tree = match listing {
        Ok(tree) if tree.file_count() == 0 => {
            let verdict = Verdict::did_not_run(
                format!("{label}: git listed no tracked file"),
                kind,
                NotRun::TargetMissing(repo_root.to_path_buf()),
            );
            return Err(GateRun::stopped(label, header(), verdict));
        }
        Ok(tree) => tree,
        Err(cause) => {
            let verdict = Verdict::did_not_run(
                format!("{label} could not list the tracked files"),
                kind,
                cause,
            );
            return Err(GateRun::stopped(label, header(), verdict));
        }
    };
    match GateScope::resolve(scope_values, repo_root, &tree) {
        Ok(scope) => Ok((tree, scope)),
        Err(refusal) => {
            let verdict = Verdict::did_not_run(
                format!("{label} scope `{}` {}", refusal.value, refusal.reason),
                kind,
                NotRun::TargetMissing(repo_root.join(&refusal.value)),
            );
            Err(GateRun::stopped(label, header(), verdict))
        }
    }
}

/// The verdict for a scope in which a gate found nothing to judge: an empty judgement is never a
/// clean one.
fn judged_nothing(label: &str, kind: Kind, repo_root: &Path, scope: &GateScope) -> Verdict {
    Verdict::did_not_run(
        format!("{label} judged nothing in {}", scope.describe()),
        kind,
        NotRun::TargetMissing(scope.anchor(repo_root)),
    )
}

/// A tracked file's text. Bytes that are not UTF-8 are replaced rather than refused, so a stray
/// byte is judged instead of turning the check into "did not run"; a file git lists but the disk
/// lacks is [`NotRun::TargetMissing`].
fn read_tracked(repo_root: &Path, path: &str) -> Result<String, NotRun> {
    let full = repo_root.join(path);
    match std::fs::read(&full) {
        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(NotRun::TargetMissing(full))
        }
        Err(source) => Err(NotRun::Unreadable { path: full, source }),
    }
}
