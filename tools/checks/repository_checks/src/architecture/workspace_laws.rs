//! The workspace-law gates: `cargo xtask verify crate-tiers`, `crate-anatomy`,
//! `test-file-reachability`, `frontend-layering` and `tailwind-sources`.
//!
//! **Role:** prints the report of each law of
//! [`repository_laws::workspace_laws`] over a checkout and exits with its
//! code; the `verify-workspace-laws` task row runs the five in order.
//! **Position:** called by `tools/xtask/src/commands/verify/dispatch.rs` and the
//! `verify-workspace-laws` row of the `ci` task table; the paths the laws read come from
//! `super::workspace_law_locations`, the rules from `verification_core`.
//! **Signals & state:** none; one read of the checkout per law, one print.
//! **Invariants:** the output is the library report line for line, and the exit code is the
//! report's: 0 the law held, 1 a finding, 2 an input that could not be read.

use std::path::Path;

use crate::Result;
use repository_laws::workspace_laws::WorkspaceLawReport;
use repository_laws::workspace_laws::crate_anatomy::check_crate_anatomy;
use repository_laws::workspace_laws::crate_tiers::check_crate_tiers;
use repository_laws::workspace_laws::frontend_layering::check_frontend_layering;
use repository_laws::workspace_laws::tailwind_sources::check_tailwind_sources;
use repository_laws::workspace_laws::test_file_reachability::check_test_file_reachability;

use super::workspace_law_locations::{FRONTEND_LAYERS, MANIFEST_SWEEP_ROOTS, TAILWIND_STYLESHEET};
use repository_root::find_repository_root;

#[cfg(test)]
#[path = "tests/workspace_laws.rs"]
mod tests;

/// One workspace law, as `cargo xtask verify <law>` names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceLaw {
    /// `verify crate-tiers`.
    CrateTiers,
    /// `verify crate-anatomy`.
    CrateAnatomy,
    /// `verify test-file-reachability`.
    TestFileReachability,
    /// `verify frontend-layering`.
    FrontendLayering,
    /// `verify tailwind-sources`.
    TailwindSources,
}

/// The report of `law` over the checkout at `repo_root`.
pub fn workspace_law_report(law: WorkspaceLaw, repo_root: &Path) -> WorkspaceLawReport {
    match law {
        WorkspaceLaw::CrateTiers => check_crate_tiers(repo_root, MANIFEST_SWEEP_ROOTS),
        WorkspaceLaw::CrateAnatomy => check_crate_anatomy(repo_root),
        WorkspaceLaw::TestFileReachability => check_test_file_reachability(repo_root),
        WorkspaceLaw::FrontendLayering => check_frontend_layering(repo_root, FRONTEND_LAYERS),
        WorkspaceLaw::TailwindSources => check_tailwind_sources(repo_root, TAILWIND_STYLESHEET),
    }
}

/// Print the report of `law` for the checkout at `repo_root` and return its exit code.
pub fn verify_workspace_law(law: WorkspaceLaw, repo_root: &Path) -> Result<u8> {
    let report = workspace_law_report(law, repo_root);
    for line in &report.lines {
        println!("{line}");
    }
    Ok(report.exit_code)
}

/// `cargo xtask verify crate-tiers` over the checkout this command runs in.
pub fn verify_crate_tiers() -> Result<u8> {
    verify_here(WorkspaceLaw::CrateTiers)
}

/// `cargo xtask verify crate-anatomy` over the checkout this command runs in.
pub fn verify_crate_anatomy() -> Result<u8> {
    verify_here(WorkspaceLaw::CrateAnatomy)
}

/// `cargo xtask verify test-file-reachability` over the checkout this command runs in.
pub fn verify_test_file_reachability() -> Result<u8> {
    verify_here(WorkspaceLaw::TestFileReachability)
}

/// `cargo xtask verify frontend-layering` over the checkout this command runs in.
pub fn verify_frontend_layering() -> Result<u8> {
    verify_here(WorkspaceLaw::FrontendLayering)
}

/// `cargo xtask verify tailwind-sources` over the checkout this command runs in.
pub fn verify_tailwind_sources() -> Result<u8> {
    verify_here(WorkspaceLaw::TailwindSources)
}

/// [`verify_workspace_law`] over the checkout this command runs in.
fn verify_here(law: WorkspaceLaw) -> Result<u8> {
    verify_workspace_law(law, &find_repository_root()?)
}
