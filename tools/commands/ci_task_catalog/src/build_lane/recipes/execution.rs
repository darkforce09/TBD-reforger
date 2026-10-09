//! `cargo xtask mk <target> [--dry-run]`: the dispatch from a target name to its recipe.
//!
//! **Role:** [`run`] answers one `mk` invocation: the target list, the three targets that compute
//! or delete, a dry run's printed lines, or the recipe's run.
//! **Position:** re-exported from [`super`] as the lane's entry; the xtask binary's `mk` verb calls
//! it with its raw arguments.
//! **Signals & state:** none held; the recipe children inherit stdio.
//! **Invariants:** every advertised target dispatches; an unknown target exits 2 with the list.

use super::*;

/// `cargo xtask mk <target> [--dry-run]`.
///
/// `--dry-run` is the analog of `make -n`: it prints the recipe lines without running them. It is
/// not a convenience — it is the only deterministic acceptance surface for the targets that never
/// terminate (`leptos`, `leptos-debug`) or that start a server (`rust-api`).
pub fn run(args: &[String]) -> Result<u8> {
    let dry = args.iter().any(|a| a == "--dry-run" || a == "-n");
    let target = args.iter().find(|a| !a.starts_with('-')).cloned();
    let Some(target) = target else {
        println!("usage: cargo xtask mk <target> [--dry-run]");
        for t in TARGETS {
            println!("  {t}");
        }
        return Ok(if args.iter().any(|a| a == "--list") {
            0
        } else {
            2
        });
    };

    // Asked and answered once, so the advertised list and the dispatch below cannot disagree —
    // and so `handles`, the chaining seam, is the same predicate callers get.
    if !handles(&target) {
        return unknown_target(&target);
    }

    // The three non-recipe targets: they compute or delete, they do not spawn a build.
    match target.as_str() {
        "print-cargo-target-dir" => {
            println!("{}", resolve_target_dir(env_pin().as_deref()));
            return Ok(0);
        }
        "reclaim-target-ci" => return reclaim_target_ci(&primary_root()),
        _ => {}
    }

    // `None` is reachable only if TARGETS advertises something with no recipe. Reported, never
    // panicked.
    if dry {
        let Some(lines) = recipe_lines(&target)? else {
            return unknown_target(&target);
        };
        for line in lines {
            println!("{line}");
        }
        return Ok(0);
    }
    if target == "rust-ci" {
        return rust_ci();
    }
    let Some(steps) = recipe_steps(&target)? else {
        return unknown_target(&target);
    };
    run_steps(&steps)
}

/// The lines a recipe target runs, in order, as `--dry-run` prints them; `None` for the three
/// targets that compute or delete instead of running a recipe, and for a name that is no target.
///
/// # Errors
/// A recipe that derives its lines from the workspace (`wasm-ci`, `ci-local-leptos`, and
/// `rust-ci` through `wasm-ci`) cannot read it.
pub(crate) fn recipe_lines(target: &str) -> Result<Option<Vec<String>>> {
    if target == "rust-ci" {
        return rust_ci_lines().map(Some);
    }
    Ok(recipe_steps(target)?.map(|steps| steps.iter().map(Step::echo).collect()))
}

/// The step list of every recipe target but the `rust-ci` composite, which [`rust_ci`] runs.
fn recipe_steps(target: &str) -> Result<Option<Vec<Step>>> {
    Ok(Some(match target {
        "rust-api" => rust_api(),
        "rust-build" => rust_build(&cwd_root())?,
        "rust-test" => rust_test(&cwd_root())?,
        "rust-fmt" => rust_fmt(),
        "rust-clippy" => rust_clippy(&cwd_root())?,
        "wasm-ci" => wasm_ci(&cwd_root())?,
        "leptos" => leptos(),
        "leptos-debug" => leptos_debug(),
        "leptos-build" => leptos_build(),
        "gate-doctor" => gate_doctor(),
        "leptos-gates" => leptos_gates(),
        "mortar-offline-gate" => mortar_offline_gate(),
        "ballistics-wasm-agreement" => ballistics_wasm_agreement(),
        "ci-local-leptos" => ci_local_leptos(&cwd_root())?,
        _ => return Ok(None),
    }))
}
