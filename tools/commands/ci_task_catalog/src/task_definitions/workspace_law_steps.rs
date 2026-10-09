//! Step list of the `verify-workspace-laws` row: the five workspace laws.
//!
//! **Role:** Holds the five in-process steps of the `verify-workspace-laws` row of
//! [`super::TASKS`], one per law of
//! [`repository_checks::architecture::workspace_laws`], each echoing its own command.
//! **Position:** Pulled into `task_definitions.rs` by `#[path]`; read only by that row, which
//! `ci-local` runs right after `verify-ci-shell`.
//! **Signals & state:** None; constant data.
//! **Invariants:** The steps run in the order the laws build on each other — membership and
//! tiers first, then anatomy, test-file reachability, frontend layering and Tailwind sources — and
//! each echo is the exact `cargo xtask verify` command the `language-gates` job of `ci.yml` runs.

use super::Step;
use repository_checks::architecture::workspace_laws::{
    verify_crate_anatomy, verify_crate_tiers, verify_frontend_layering, verify_tailwind_sources,
    verify_test_file_reachability,
};

/// The five workspace laws, one step each.
pub(super) const WORKSPACE_LAW_STEPS: &[Step] = &[
    xt!("cargo xtask verify crate-tiers", false, || Ok(
        verify_crate_tiers()?
    )),
    xt!("cargo xtask verify crate-anatomy", false, || Ok(
        verify_crate_anatomy()?
    )),
    xt!("cargo xtask verify test-file-reachability", false, || Ok(
        verify_test_file_reachability()?
    )),
    xt!("cargo xtask verify frontend-layering", false, || Ok(
        verify_frontend_layering()?
    )),
    xt!("cargo xtask verify tailwind-sources", false, || Ok(
        verify_tailwind_sources()?
    )),
];
