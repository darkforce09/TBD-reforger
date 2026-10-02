//! Step list of the `verify-workspace-laws` row: the five workspace laws.
//!
//! **Role:** Holds the five in-process steps of the `verify-workspace-laws` row of
//! [`super::TASKS`], one per law of
//! [`crate::verifications::architecture::workspace_laws`], each echoing its own command.
//! **Position:** Pulled into `task_definitions.rs` by `#[path]`; read only by that row, which
//! `ci-local` runs right after `verify-engine-layers`.
//! **Signals & state:** None; constant data.
//! **Invariants:** The steps run in the order the laws build on each other — membership and
//! tiers first, then anatomy, the strangler rule, frontend layering and Tailwind sources — and
//! each echo is the exact `cargo xtask verify` command the `language-gates` job of `ci.yml` runs.

use super::Step;
use crate::verifications::architecture::workspace_laws::{
    verify_crate_anatomy, verify_crate_tiers, verify_frontend_layering, verify_strangler,
    verify_tailwind_sources,
};

/// The five workspace laws, one step each.
pub(super) const WORKSPACE_LAW_STEPS: &[Step] = &[
    xt!("cargo xtask verify crate-tiers", false, verify_crate_tiers),
    xt!(
        "cargo xtask verify crate-anatomy",
        false,
        verify_crate_anatomy
    ),
    xt!("cargo xtask verify strangler", false, verify_strangler),
    xt!(
        "cargo xtask verify frontend-layering",
        false,
        verify_frontend_layering
    ),
    xt!(
        "cargo xtask verify tailwind-sources",
        false,
        verify_tailwind_sources
    ),
];
