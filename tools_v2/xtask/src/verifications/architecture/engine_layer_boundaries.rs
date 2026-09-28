//! Engine-layer walls — the `cargo xtask verify engine-layers` gate.
//!
//! **Role:** prints the report of
//! [`verification_core::repository_laws::engine_layers::check_engine_layers`] — the eight
//! engine-layer rules of `documentation_v2/standards/engine_boundary_rules.md` §5 — and exits
//! with its code.
//! **Position:** called by `tools_v2/xtask/src/commands/verify/dispatch.rs` and the
//! `verify-engine-layers` row of the `ci` task table; the rules, matchers, pins and report text
//! live in `verification_core::repository_laws::engine_layers`.
//! **Signals & state:** none; one read of the checkout, one print.
//! **Invariants:** the output is the library report line for line, and the exit code is the
//! report's: 0 every rule held, 1 a breach or an empty root, 2 an input that could not be read.

use std::path::Path;

use anyhow::Result;
use verification_core::repository_laws::engine_layers::check_engine_layers;

#[cfg(test)]
#[path = "tests/engine_layer_boundaries.rs"]
mod tests;

/// Print the engine-layer report for the checkout at `repo_root` and return its exit code.
pub fn verify_engine_layers(repo_root: &Path) -> Result<u8> {
    let report = check_engine_layers(repo_root);
    for line in &report.lines {
        println!("{line}");
    }
    Ok(report.exit_code)
}
