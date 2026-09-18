//! `cargo xtask mk <target>` — the Makefile's **build/test lane**, in Rust.
//!
//! T-853 Phase 3, slice T-895. Sixteen `make` targets move here byte-for-byte:
//! `rust-api rust-build rust-test rust-fmt rust-clippy rust-ci rust-sqlx-prepare wasm-ci
//! leptos leptos-debug leptos-build leptos-gates ci-local-leptos verify-cargo-target
//! print-cargo-target-dir reclaim-target-ci`. The Makefile itself is deleted by T-897; this slice
//! only has to make the equivalents exist and be provably identical.
//!
//! ── WHERE THE TARGET-DIR PIN LIVES ───────────────────────────────────────────────────────────
//!
//! In [`crate::core::cargo_target_directory`], with the two `make` targets that police it. That module is the one
//! to read before changing anything here: `CARGO_TARGET_DIR` is derived from `git rev-parse
//! --git-common-dir` so that every linked worktree shares the PRIMARY repo's warm `target/`
//! (T-253/T-322), and a `.cargo/config.toml` `[env]` with `relative = true` would silently reverse
//! that.
//!
//! Because the pin is a *value we compute*, it must be **injected into every child cargo**
//! ([`run_steps`]) rather than left to inheritance: `make` `export`ed it, so its children saw it,
//! and an `xtask` invoked without it in the environment must reproduce that. The one recipe-level
//! override is `rust-api`'s private `$(CURDIR)/target-dev-api` (T-322) — the *other* root, and the
//! reason `mk_target_dir` has two.
//!
//! ── OUTPUT IS A CONTRACT ─────────────────────────────────────────────────────────────────────
//!
//! `make` echoes each recipe line to **stdout** before running it, and the child's own output goes
//! wherever the child writes it. Acceptance for this slice is a stdout+stderr+rc diff against
//! `make`, so [`Step::echo`] reproduces those lines — and derives them **from the argv/cwd/env that
//! actually run**, so the label and the command cannot drift apart.
//!
//! Two deliberate divergences, both reported rather than papered over:
//!
//! 1. `make` collapses every failure to **rc 2** and prints `make: *** [Makefile:N: t] Error C` on
//!    stderr. Here the child's **raw** exit code is propagated and nothing extra is printed. That
//!    is [`verification_core::proc`]'s rule (`compile.sh --selftest` passes only on exactly 1), and a
//!    Makefile line number is not something a Makefile-less tree can honestly print.
//! 2. Composites (`rust-ci`, `leptos-gates`) call Rust functions instead of `$(MAKE) sub-target`,
//!    so make's `make[1]: Entering/Leaving directory` scaffolding and its `make <target>` echo are
//!    absent. The leaf lines under them are byte-identical.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::Result;
use verification_core::proc::Run;
use verification_core::{Kind, NotRun, Verdict};

use crate::core::cargo_target_directory::{
    DEV_API_TARGET, abi_guard, cwd_root, env_pin, primary_root, reclaim_target_ci,
    resolve_target_dir, verify_cargo_target,
};

// ── A RECIPE LINE ────────────────────────────────────────────────────────────────────────────

/// One line of a `make` recipe: where it runs, what it sets, what it execs.
///
/// `envs` are **recipe-level** assignments — the ones make echoed as part of the line, e.g.
/// `CARGO_TARGET_DIR=…/target-dev-api cargo run --bin api`. The inherited shared pin is NOT one of
/// these; it is injected by [`run_steps`] and was never echoed. That distinction is what
/// [`verify_cargo_target`] §5 checks, so it is structural rather than a convention.
pub(crate) struct Step {
    cwd: Option<String>,
    envs: Vec<(String, String)>,
    argv: Vec<String>,
    /// make's leading `-`: run it, ignore a non-zero status. Used by `rust-test-it`'s first DROP.
    ignore_error: bool,
}

impl Step {
    pub(crate) fn new(argv: &[&str]) -> Step {
        Step {
            cwd: None,
            envs: Vec::new(),
            argv: argv.iter().map(|s| (*s).to_string()).collect(),
            ignore_error: false,
        }
    }
    pub(crate) fn cd(mut self, dir: &str) -> Step {
        self.cwd = Some(dir.to_string());
        self
    }
    pub(crate) fn env(mut self, k: &str, v: &str) -> Step {
        self.envs.push((k.to_string(), v.to_string()));
        self
    }
    fn ignore_error(mut self) -> Step {
        self.ignore_error = true;
        self
    }

    /// The value of a **recipe-level** env assignment, if this line makes one.
    ///
    /// The accessor rather than a public field: `verify-cargo-target` §5 asks exactly this question
    /// and nothing else needs the vector. Keeping `envs` private is what stops a future caller from
    /// *appending* to a recipe from the outside, which is the shape that would let `rust-build`
    /// acquire a private target dir without the gate having anything to look at.
    pub(crate) fn recipe_env(&self, key: &str) -> Option<&str> {
        self.envs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The line `make` printed before running this — rendered FROM the fields that run.
    ///
    /// Hand-writing the label next to the argv is how a port drifts: the two are edited apart and
    /// the tool then narrates something it is not doing. Here there is only one source.
    ///
    /// `shell_word` re-quotes arguments containing whitespace because make echoed the recipe
    /// *text*, and every such argument in this lane was written quoted (the three `psql -qc "…"`
    /// calls). Nothing here contains a `"` or a `$`, so the naive rule is exact; a future argument
    /// that does would need real quoting, and `tests::echo_matches_make` would catch it.
    pub(crate) fn echo(&self) -> String {
        let mut s = String::new();
        if let Some(d) = &self.cwd {
            s.push_str(&format!("cd {d} && "));
        }
        for (k, v) in &self.envs {
            s.push_str(&format!("{k}={v} "));
        }
        s.push_str(
            &self
                .argv
                .iter()
                .map(|a| shell_word(a))
                .collect::<Vec<_>>()
                .join(" "),
        );
        s
    }
}

// ── THE RUNNER ───────────────────────────────────────────────────────────────────────────────

// ── THE RECIPES ──────────────────────────────────────────────────────────────────────────────

pub(crate) const WEB: &str = "apps/website/api_v2";
const FE: &str = "apps/website/frontend";

// ── DISPATCH ─────────────────────────────────────────────────────────────────────────────────

/// Every Makefile target this module answers to, in `make help` order.
pub(crate) const TARGETS: &[&str] = &[
    "print-cargo-target-dir",
    "verify-cargo-target",
    "reclaim-target-ci",
    "rust-api",
    "rust-build",
    "rust-test",
    "rust-fmt",
    "rust-clippy",
    "rust-sqlx-prepare",
    "rust-ci",
    "wasm-ci",
    "leptos",
    "leptos-debug",
    "leptos-build",
    "gate-doctor",
    "leptos-gates",
    "ci-local-leptos",
];

mod shell_word;
pub(crate) use shell_word::ci_local_leptos;
pub(crate) use shell_word::gate_doctor;
pub(crate) use shell_word::handles;
pub(crate) use shell_word::leptos;
pub(crate) use shell_word::leptos_build;
pub(crate) use shell_word::leptos_debug;
pub(crate) use shell_word::leptos_gates;
use shell_word::run_steps;
pub(crate) use shell_word::rust_api;
pub(crate) use shell_word::rust_build;
use shell_word::rust_ci;
pub(crate) use shell_word::rust_clippy;
pub(crate) use shell_word::rust_fmt;
pub(crate) use shell_word::rust_sqlx_prepare;
pub(crate) use shell_word::rust_test;
pub(crate) use shell_word::rust_test_it;
use shell_word::shell_word;
use shell_word::unknown_target;
pub(crate) use shell_word::wasm_ci;

mod execution;
pub(crate) use execution::run;
