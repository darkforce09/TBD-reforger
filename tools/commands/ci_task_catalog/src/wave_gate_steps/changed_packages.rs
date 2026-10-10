//! `cargo xtask mk changed-packages [--range R]`: what a range changed, as JSON for the runner.
//!
//! **Role:** lists the committed and working-tree changes of a range, the changed Rust files, the
//! packages that own them (an orphan fragment counts for every package that `include!`s it), and
//! the two scopes the gates key steps off: `wasm_scope` (the committed diff reaches a crate the
//! single-page app compiles — the trunk build) and `frontend_tests` (committed or working-tree
//! changes reach that scope or one of its `include_str!` inputs — the slice gate's frontend tests).
//! **Position:** named by `[gate] changed_packages` in `ticket_manager_execution.toml`; the ticket
//! manager's runner reads `scopes.<name>.touched` for steps with `when_scope`.
//! **Signals & state:** none; reads git and the checkout.
//! **Invariants:** a porcelain read that fails exits non-zero (never an empty change list); each
//! scope carries the detail a SKIP line prints, so an untouched scope names what it decided against.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::json;

use super::changed::{
    DEFAULT_BASE, frontend_include_input_touched, include_consumer_package_dirs,
    owning_package_dir, wasm_scope_prefixes, wasm_scope_touched,
};
use super::step_context::{Ctx, git_porcelain_paths, git_stdout_lossy};

/// Print the JSON for `range` (default `main...HEAD`); 0 printed, 1 the change list is unreadable.
pub(crate) fn print_changed_packages(ctx: &Ctx, range: &str) -> i32 {
    let range = if range.is_empty() {
        DEFAULT_BASE
    } else {
        range
    };
    let working_tree: Vec<String> = match git_porcelain_paths() {
        Ok(v) => v.into_iter().filter(|p| !p.is_empty()).collect(),
        Err(rc) => return rc.max(1),
    };
    let committed: Vec<String> = git_stdout_lossy(&["diff", "--name-only", range])
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let all: BTreeSet<&str> = committed
        .iter()
        .chain(working_tree.iter())
        .map(String::as_str)
        .collect();
    let rust_files: Vec<&str> = all.iter().copied().filter(|p| p.ends_with(".rs")).collect();
    let mut package_dirs: BTreeSet<String> = BTreeSet::new();
    for f in &rust_files {
        match owning_package_dir(f) {
            Some(d) => {
                package_dirs.insert(d);
            }
            None => package_dirs.extend(include_consumer_package_dirs(f)),
        }
    }
    let packages: Vec<String> = package_dirs
        .iter()
        .filter_map(|d| {
            repository_laws::cargo_manifest::read_manifest(&Path::new(d).join("Cargo.toml"))
                .ok()
                .and_then(|m| m.package_name)
        })
        .collect();
    let prefixes = wasm_scope_prefixes(&ctx.root).join(" ");
    let wasm = wasm_scope_touched(&ctx.root, committed.iter().map(String::as_str));
    let frontend = wasm_scope_touched(&ctx.root, all.iter().copied())
        || frontend_include_input_touched(&ctx.root, all.iter().copied());
    let report = json!({
        "format": "reforger.changed-packages/1",
        "range": range,
        "committed_files": committed,
        "working_tree_files": working_tree,
        "rust_files": rust_files,
        "package_dirs": package_dirs,
        "packages": packages,
        "scopes": {
            "wasm_scope": {
                "touched": wasm,
                "detail": if wasm {
                    "the range changes what the single-page app compiles".to_string()
                } else {
                    format!("nothing the SPA compiles changed in {range}: {prefixes}")
                },
            },
            "frontend_tests": {
                "touched": frontend,
                "detail": if frontend {
                    "the change reaches the frontend family or its include inputs".to_string()
                } else {
                    format!("frontend untouched — scope: {prefixes} (+ their include_str! inputs)")
                },
            },
        },
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    0
}
