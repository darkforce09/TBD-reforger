//! The repository-specific halves of the ticket manager's slice and wave gates, as `cargo xtask mk`
//! helper commands.
//!
//! **Role:** the ticket manager's generic runner (`ttm wave gate`, configured by
//! `ticket_manager_execution.toml`) runs these as gate steps: the changed-package scopes
//! (`mk changed-packages`), the fingerprint touches, the change-scoped format check, the clippy
//! lanes and workspace test steps derived from the manifest, the gate database and its API tests,
//! the persistent migration step, the trunk build, the schema sub-gates (`mk gate-step <name>`),
//! the API freshness preflight probe (`mk preflight-api-freshness`) and the run lane's glibc stamp
//! guard (`mk target-abi-guard <folder>`).
//! **Position:** dispatched from `build_lane::recipes::run` ahead of the recipe table; reads the
//! environment the runner exports (`TTM_ROOT`, `TTM_MAIN_ROOT`, `TTM_GATE_RANGE`, `TTM_SLICE`,
//! `TTM_WAVE`, `TTM_GATE_LOCK`, `CARGO_TARGET_DIR`) and falls back to git when run by hand.
//! **Signals & state:** each step prints its own output and returns its exit code; the runner
//! captures the output and shows it only on failure.
//! **Invariants:** a step that could not examine its input is red, never green: an unreadable
//! workspace, an unreadable change list, a missing gate wave number and a gate lock no gate holds
//! (for the destructive database steps) each refuse; the step names here are the vocabulary the
//! configuration file uses, and an unknown name exits 2 with the list.

use std::path::Path;

mod api_freshness;
mod changed;
mod changed_packages;
mod clippy_lanes;
mod gate_database;
mod host;
mod migration_persistence;
mod schema_step;
mod step_context;
mod touch;
mod trunk_build;

pub(crate) use step_context::flush;

/// `println!` for a step (the runner captures stdout and stderr together).
macro_rules! wprintln {
    () => { println!() };
    ($($arg:tt)*) => { println!($($arg)*) };
}

/// `print!` for a step.
macro_rules! wprint {
    ($($arg:tt)*) => { print!($($arg)*) };
}

/// `eprintln!` for a step, stdout flushed first.
macro_rules! werr {
    ($($arg:tt)*) => {{
        $crate::wave_gate_steps::flush();
        eprintln!($($arg)*)
    }};
}

pub(crate) use {werr, wprint, wprintln};

/// The `cargo xtask mk` helper commands this module answers.
pub const HELPER_TARGETS: &[&str] = &[
    "changed-packages",
    "gate-step",
    "preflight-api-freshness",
    "target-abi-guard",
];

/// The step names `mk gate-step` accepts.
pub const GATE_STEPS: &[&str] = &[
    "touch-changed",
    "touch-workspace",
    "fmt-changed",
    "frontend-tests-changed",
    "clippy-native",
    "clippy-wasm32",
    "clippy-frontend",
    "clippy-frontend-native",
    "clippy-tools",
    "gate-database",
    "migrate-persist",
    "test-api",
    "test-frontend",
    "test-workspace-members",
    "trunk-build",
    "schema",
];

/// The value after `flag`, else the environment variable `var`, else empty.
fn option(args: &[String], flag: &str, var: &str) -> String {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
        .or_else(|| std::env::var(var).ok())
        .unwrap_or_default()
}

/// `cargo xtask mk <helper> …`: `Some(exit code)` when `args` names a helper, `None` otherwise.
pub fn dispatch(args: &[String]) -> Option<u8> {
    let name = args.first()?;
    if !HELPER_TARGETS.contains(&name.as_str()) {
        return None;
    }
    if name == "target-abi-guard" {
        let Some(folder) = args.get(1) else {
            eprintln!("usage: cargo xtask mk target-abi-guard <folder>");
            return Some(2);
        };
        return Some(
            match crate::cargo_target_pin::abi_guard(Path::new(folder)) {
                Ok(()) => 0,
                Err(why) => {
                    eprintln!("{why}");
                    1
                }
            },
        );
    }
    let ctx = match step_context::Ctx::from_environment() {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("mk {name}: cannot enter the checkout: {e}");
            return Some(1);
        }
    };
    let rest = &args[1..];
    let rc = match name.as_str() {
        "changed-packages" => changed_packages::print_changed_packages(
            &ctx,
            &option(rest, "--range", "TTM_GATE_RANGE"),
        ),
        "preflight-api-freshness" => api_freshness::api_freshness(&ctx),
        _ => run_gate_step(&ctx, rest),
    };
    flush();
    Some(rc.clamp(0, 255) as u8)
}

fn run_gate_step(ctx: &step_context::Ctx, args: &[String]) -> i32 {
    let step = args.first().map(String::as_str).unwrap_or("");
    let range = option(args, "--range", "TTM_GATE_RANGE");
    match step {
        "touch-changed" => touch::touch_changed(&range),
        "touch-workspace" => touch::touch_workspace(ctx),
        "fmt-changed" => changed::fmt_changed(ctx, &range),
        "frontend-tests-changed" => {
            let slice = option(args, "--slice", "TTM_SLICE");
            if slice.is_empty() {
                eprintln!(
                    "gate-step frontend-tests-changed: --slice <slice> (or TTM_SLICE) is required"
                );
                return 2;
            }
            changed::frontend_tests_changed(ctx, &range, &slice)
        }
        "clippy-native" => clippy_lanes::clippy_native(ctx),
        "clippy-wasm32" => clippy_lanes::clippy_wasm32(ctx),
        "clippy-frontend" => clippy_lanes::clippy_frontend(ctx, true),
        "clippy-frontend-native" => clippy_lanes::clippy_frontend(ctx, false),
        "clippy-tools" => clippy_lanes::clippy_tools(ctx),
        "gate-database" => gate_database::ensure_gate_db(ctx, &step_context::GateState::probe()),
        "migrate-persist" => {
            let mode = args.get(1).map(String::as_str).unwrap_or("audit");
            i32::from(migration_persistence::gate_db_migrate_persist(
                ctx,
                &step_context::GateState::probe(),
                mode,
            ))
        }
        "test-api" => match gate_database::export_gate_database_url(ctx) {
            0 => gate_database::gate_test_api(ctx),
            rc => rc,
        },
        "test-frontend" => clippy_lanes::test_frontend(ctx),
        "test-workspace-members" => clippy_lanes::test_workspace_members(ctx),
        "trunk-build" => trunk_build::gate_trunk_build(ctx),
        "schema" => schema_step::gate_schema(ctx),
        other => {
            eprintln!("mk gate-step: unknown step '{other}'");
            eprintln!("    known: {}", GATE_STEPS.join(" "));
            2
        }
    }
}
