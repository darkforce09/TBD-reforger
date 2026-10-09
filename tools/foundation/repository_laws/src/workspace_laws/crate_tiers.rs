//! The crate-tier law: the application boundary and the external-crate firewalls.
//!
//! **Role:** judges two rules over a checkout: no member depends on an application package, in
//! any table, and every application package is a member; and the external-crate firewalls hold
//! (`super::crate_firewalls`).
//! **Position:** `cargo xtask verify crate-tiers` prints [`check_crate_tiers`]; xtask passes the
//! [`CrateTierConfiguration`] (the application packages). Reads [`crate::workspace_members`].
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** the application rule reads every member's every table — normal, build, dev
//! and target-specific — under each edge's real package name, a crate's edge onto itself
//! excepted; an application package that is no member is a finding, never a pass. The firewalls
//! read the judged members ([`super::crate_layout::is_judged`]).

use std::path::Path;

use super::crate_layout::is_judged;
use super::{LawOutcome, WorkspaceLawReport, crate_firewalls};
use crate::workspace_members::{WorkspaceMember, read_workspace_members};
use verification_core::verdict::NotRun;

/// What the crate-tier law reads that moves with the tree; xtask passes it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrateTierConfiguration<'a> {
    /// The application packages, by package name: each must be a member, and no member depends
    /// on one in any table.
    pub application_packages: &'a [&'a str],
}

/// The crate-tier report over the checkout at `repo_root` under `configuration`.
pub fn check_crate_tiers(
    repo_root: &Path,
    configuration: &CrateTierConfiguration<'_>,
) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome("crate-tiers", crate_tier_outcome(repo_root, configuration))
}

/// The findings and notes of the crate-tier law; [`NotRun`] when an input could not be read.
pub fn crate_tier_outcome(
    repo_root: &Path,
    configuration: &CrateTierConfiguration<'_>,
) -> Result<LawOutcome, NotRun> {
    let members = read_workspace_members(repo_root)?;
    let judged: Vec<&WorkspaceMember> = members.iter().filter(|m| is_judged(m)).collect();
    let mut outcome = LawOutcome {
        summary: format!(
            "{} workspace member(s), {} judged",
            members.len(),
            judged.len()
        ),
        ..LawOutcome::default()
    };
    outcome.findings.extend(application_edge_findings(
        &members,
        configuration.application_packages,
    ));
    outcome.findings.extend(crate_firewalls::firewall_findings(
        repo_root, &members, &judged,
    )?);
    Ok(outcome)
}

/// The application rule over every member: no dependency edge, in any table, names an
/// application package (a crate's edge onto itself excepted), and every application package is a
/// member.
fn application_edge_findings(
    members: &[WorkspaceMember],
    application_packages: &[&str],
) -> Vec<String> {
    let mut findings: Vec<String> = application_packages
        .iter()
        .filter(|package| !members.iter().any(|m| m.package_name == **package))
        .map(|package| {
            format!(
                "application edge: application package `{package}` is no workspace member — the \
                 application list names members only"
            )
        })
        .collect();
    for member in members {
        for edge in &member.manifest.dependencies {
            if edge.package != member.package_name
                && application_packages.contains(&edge.package.as_str())
            {
                findings.push(format!(
                    "application edge: {}/Cargo.toml:{}: [{}] depends on the application {} — no \
                     member depends on an application package, in any table",
                    member.path, edge.line_no, edge.table, edge.package
                ));
            }
        }
    }
    findings
}

#[cfg(test)]
#[path = "tests/crate_tiers.rs"]
mod tests;
