//! LANG-2 is the same `TrackedLanguageBan` table as LANG-1.
//!
//! `cargo xtask verify no-python` stays as a CI alias so job names in `ci.yml` / `mk_ci_tasks`
//! / the wave gate do not break. It does **not** keep a second ratchet that can disagree with
//! `verify no-shell`. Inventories are gone.
//!
//! The ban is fail-closed: a search tool that cannot run is a FAIL, never a clean tree. It folds
//! the `.py` and `python3` command-position ban into
//! `tools_v2/xtask/src/verifications/language_bans/shell_scripts.rs`.

use anyhow::Result;

/// Entry for `xtask verify no-python`. Same walk as [`crate::verifications::language_bans::shell_scripts::verify_no_shell`].
pub fn verify_no_python() -> Result<u8> {
    crate::verifications::language_bans::shell_scripts::verify_no_python()
}

/// Testable / fixture entry that does not walk for the repo root.
#[cfg(test)]
pub fn run_with_root(root: &std::path::Path) -> Result<u8> {
    crate::verifications::language_bans::shell_scripts::run_with_root(root)
}

#[cfg(test)]
#[path = "tests/python_scripts/tests.rs"]
mod tests;
