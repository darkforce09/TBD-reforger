//! The workspace laws: crate tiers, crate anatomy, the strangler rule, frontend layering and
//! Tailwind sources.
//!
//! **Role:** judges the workspace members of a checkout against the layout laws of
//! `documentation_v2/restructure/laws_and_gates.md` ("New laws") and renders each law as a
//! report a gate prints: [`crate_tiers`], [`crate_anatomy`], [`strangler`],
//! [`frontend_layering`] and [`tailwind_sources`].
//! **Position:** a library layer of `verification-core` over [`super::workspace_members`] and
//! [`super::cargo_manifest`]. `cargo xtask verify crate-tiers`, `crate-anatomy`, `strangler`,
//! `frontend-layering` and `tailwind-sources` print these reports; xtask passes in every path that
//! moves with the tree (the manifest sweep roots, the frontend layer table, the stylesheet).
//! **Signals & state:** none; each law reads the checkout and returns a report.
//! **Invariants:** a law that could not read an input reports exit 2 and "did not run", never a
//! pass; a finding is exit 1; a report's last line is `<LAW>: PASS` or `<LAW>: FAIL (…)`.

pub mod crate_anatomy;
mod crate_anatomy_sources;
mod crate_firewalls;
pub mod crate_layout;
pub mod crate_tiers;
pub mod frontend_layering;
mod rust_module_references;
pub mod strangler;
pub mod tailwind_sources;

use crate::verdict::NotRun;

/// What one workspace law found over a checkout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LawOutcome {
    /// One line naming what the law read, printed first.
    pub summary: String,
    /// Breaches of the law, one line each; any finding fails the law.
    pub findings: Vec<String>,
    /// Informational lines that do not fail the law.
    pub notes: Vec<String>,
}

/// The printed result of one workspace law.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLawReport {
    /// 0 the law held, 1 a finding, 2 an input could not be read.
    pub exit_code: u8,
    /// The lines the gate prints, the verdict line last.
    pub lines: Vec<String>,
}

impl WorkspaceLawReport {
    /// The report of `law` (its gate name, e.g. `crate-tiers`) over `outcome`.
    pub fn from_outcome(law: &str, outcome: Result<LawOutcome, NotRun>) -> Self {
        let verdict = law.to_uppercase();
        match outcome {
            Ok(outcome) => {
                let mut lines = vec![format!("==> {law} — {}", outcome.summary)];
                lines.extend(outcome.notes.iter().map(|note| format!("note: {note}")));
                lines.extend(
                    outcome
                        .findings
                        .iter()
                        .map(|finding| format!("FAIL: {finding}")),
                );
                let exit_code = if outcome.findings.is_empty() {
                    lines.push(format!("{verdict}: PASS"));
                    0
                } else {
                    lines.push(format!(
                        "{verdict}: FAIL ({} finding(s))",
                        outcome.findings.len()
                    ));
                    1
                };
                Self { exit_code, lines }
            }
            Err(not_run) => Self {
                exit_code: 2,
                lines: vec![
                    format!("==> {law} — did not run: {}", describe_not_run(&not_run)),
                    "      A law whose input could not be read is not a pass.".to_string(),
                    format!("{verdict}: FAIL (did not run)"),
                ],
            },
        }
    }
}

/// One line naming why a law could not read its input.
fn describe_not_run(not_run: &NotRun) -> String {
    match not_run {
        NotRun::TargetMissing(path) => format!("missing: {}", path.display()),
        NotRun::Unreadable { path, source } => {
            format!("unreadable: {}: {source}", path.display())
        }
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
#[path = "tests/fixture_workspace.rs"]
mod fixture_workspace;
