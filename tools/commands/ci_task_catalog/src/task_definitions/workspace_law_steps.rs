//! Step list of the `verify-workspace-laws` row: the three enforced workspace laws.
//!
//! **Role:** Holds the three in-process steps of the `verify-workspace-laws` row of
//! [`super::TASKS`], one per enforced law of
//! [`repository_checks::architecture::workspace_laws`], each echoing its own command.
//! **Position:** Pulled into `task_definitions.rs` by `#[path]`; read only by that row, which
//! `ci-local` runs.
//! **Signals & state:** None; constant data.
//! **Invariants:** The steps run in the order the laws build on each other — membership and
//! tiers first, then anatomy and Tailwind sources — and each echo is the exact `cargo xtask verify`
//! command the `language-gates` job of `ci.yml` runs.

use super::Step;
use repository_checks::architecture::workspace_laws::{
    verify_crate_anatomy, verify_crate_tiers, verify_tailwind_sources,
};

/// The three enforced workspace laws, one step each.
pub(super) const WORKSPACE_LAW_STEPS: &[Step] = &[
    xt!("cargo xtask verify crate-tiers", false, || Ok(
        verify_crate_tiers()?
    )),
    xt!("cargo xtask verify crate-anatomy", false, || Ok(
        verify_crate_anatomy()?
    )),
    xt!("cargo xtask verify tailwind-sources", false, || Ok(
        verify_tailwind_sources()?
    )),
];
