//! The pinned environment of the headless browser gates.
//!
//! **Role:** the repository-relative path of the committed pin file naming the Chrome for Testing
//! build, the toolchain and the resources the browser gates expect.
//! **Position:** the `ci-chrome` task of `xtask ci` installs the pinned browser from it; the
//! `gate doctor` of `browser_gate_suites` compares the resolved browser and toolchain against it.
//! **Signals & state:** none; a constant.
//! **Invariants:** a committed JSON file, relative to a checkout root.

/// The committed pin file of the browser gates: the Chrome for Testing version, the toolchain and
/// the resource floors `gate doctor` checks.
pub const BROWSER_GATE_ENVIRONMENT: &str =
    "tools/browser_testing/browser_gate_suites/gate-env.json";

#[cfg(test)]
#[path = "tests/browser_gate_environment_tests.rs"]
mod tests;
