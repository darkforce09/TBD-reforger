//! Unit tests for [`crate::build_lane::recipes`]: the echoed recipe lines, the CI-row parity,
//! the dispatch table and the container-runtime lines. The target-directory checks the recipes
//! dispatch to are tested beside them, in `crate::cargo_target_verification`.

use super::*;

/// The echoed lines, pinned against the strings `make -n` printed on 2026-08-12.
#[test]
fn echo_matches_make() {
    assert_eq!(
        rust_build()[0].echo(),
        "cd apps/api && cargo build --all-targets"
    );
    assert_eq!(rust_fmt()[1].echo(), "cargo fmt --all --check");
    assert_eq!(
        rust_clippy()[0].echo(),
        "cd apps/api && cargo clippy --all-targets -- -D warnings"
    );
    assert_eq!(
        leptos()[0].echo(),
        "cd apps/frontend && trunk serve --release"
    );
    assert_eq!(
        ci_local_leptos()[1].echo(),
        "cargo clippy -p frontend --target wasm32-unknown-unknown --all-targets -- -D warnings"
    );
    assert_eq!(
        ci_local_leptos()[2].echo(),
        "cargo clippy -p frontend --all-targets --locked -- -D warnings"
    );
    assert_eq!(
        ci_local_leptos()[4].echo(),
        "cd apps/frontend && trunk build --release"
    );
    assert_eq!(
        wasm_ci()[2].echo(),
        "cargo clippy -p map_engine -p graphics_engine \
         -p offline_service_worker --target wasm32-unknown-unknown -- -D warnings"
    );
    // An argument holding whitespace: make echoed the recipe TEXT, quotes included.
    assert_eq!(
        Step::new(&["psql", "-qc", "CREATE DATABASE rust_it;"]).echo(),
        "psql -qc \"CREATE DATABASE rust_it;\""
    );
    assert_eq!(
        rust_ci_lines().last().map(String::as_str),
        Some("cargo xtask db test-it")
    );
    assert_eq!(
        rust_api()[0].echo(),
        format!(
            "cd apps/api && CARGO_TARGET_DIR={}/target/dev-api cargo run --bin api",
            cwd_root().display()
        )
    );
}

/// The offline service worker crate is formatted, linted (host and wasm32) and tested by
/// `wasm-ci`: a browser crate the lane does not name is never gated.
#[test]
fn wasm_ci_gates_the_offline_service_worker_in_every_step() {
    let lines: Vec<String> = wasm_ci().iter().map(|s| s.echo()).collect();
    for prefix in [
        "cargo fmt --check",
        "cargo clippy",
        "cargo test -p offline_service_worker",
    ] {
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with(prefix) && l.contains("-p offline_service_worker")),
            "no `{prefix}` step names offline_service_worker: {lines:?}"
        );
    }
    let clippy: Vec<&String> = lines
        .iter()
        .filter(|l| l.starts_with("cargo clippy"))
        .collect();
    assert_eq!(clippy.len(), 2, "{lines:?}");
    assert!(
        clippy
            .iter()
            .all(|l| l.contains("-p offline_service_worker")),
        "{clippy:?}"
    );
    assert!(
        clippy
            .iter()
            .any(|l| l.contains("--target wasm32-unknown-unknown"))
    );
}

/// `mk wasm-ci` and the `wasm-ci` row of `cargo xtask ci` are the same lane spelled twice; the two
/// spellings run the same lines in the same order.
#[test]
fn wasm_ci_recipe_and_ci_task_row_run_the_same_lines() {
    use crate::task_runner::{Step as CiStep, TASKS};
    let row = TASKS
        .iter()
        .find(|t| t.name == "wasm-ci")
        .expect("the ci task table has a wasm-ci row");
    let ci_lines: Vec<String> = row
        .steps
        .iter()
        .map(|step| match step {
            CiStep::Cmd { line, .. } => (*line).to_string(),
            _ => panic!("the wasm-ci row runs only command lines"),
        })
        .collect();
    let mk_lines: Vec<String> = wasm_ci().iter().map(|s| s.echo()).collect();
    assert_eq!(ci_lines, mk_lines);
}

/// `mortar-offline-gate` builds the release app once, then runs `gate mortar-offline` on it.
#[test]
fn mortar_offline_gate_builds_then_runs_the_offline_gate() {
    let lines: Vec<String> = mortar_offline_gate().iter().map(|s| s.echo()).collect();
    assert_eq!(
        lines,
        vec![
            "cd apps/frontend && trunk build --release".to_string(),
            "cargo run -q -p developer_tools --bin gate -- mortar-offline".to_string(),
        ]
    );
    assert!(TARGETS.contains(&"mortar-offline-gate"));
}

/// `ballistics-wasm-agreement` builds the release app once, then runs `gate ballistics-agreement`.
#[test]
fn ballistics_wasm_agreement_builds_then_runs_the_agreement_gate() {
    let lines: Vec<String> = ballistics_wasm_agreement()
        .iter()
        .map(|s| s.echo())
        .collect();
    assert_eq!(
        lines,
        vec![
            "cd apps/frontend && trunk build --release".to_string(),
            "cargo run -q -p developer_tools --bin gate -- ballistics-agreement".to_string(),
        ]
    );
    assert!(TARGETS.contains(&"ballistics-wasm-agreement"));
}

/// `leptos-gates` runs `trunk build --release` ONCE — make builds a prerequisite once per run.
#[test]
fn leptos_gates_does_not_double_build() {
    let n = leptos_gates()
        .iter()
        .filter(|s| s.echo().contains("trunk build --release"))
        .count();
    assert_eq!(n, 1);
    assert_eq!(leptos_gates().len(), 4);
}

/// `handles` and the dispatch table agree — an entry that dispatches nowhere would make the
/// The chaining seam must not silently swallow a target.
#[test]
fn every_advertised_target_dispatches() {
    for t in TARGETS {
        assert!(handles(t));
    }
    assert!(!handles("db-up"));
    // Only the RECIPE targets: `--dry-run` is a no-op for the three that compute or delete,
    // and `reclaim-target-ci` would run for real against the primary repo from a unit test.
    for t in TARGETS {
        if matches!(
            *t,
            "print-cargo-target-dir" | "verify-cargo-target" | "reclaim-target-ci"
        ) {
            continue;
        }
        let rc = run(&[t.to_string(), "--dry-run".into()]).unwrap();
        assert_eq!(rc, 0, "{t} --dry-run");
    }
    assert_eq!(run(&["no-such-target".to_string()]).unwrap(), 2);
}

/// The container runtimes a recipe line never runs itself.
const CONTAINER_RUNTIMES: &[&str] = &["podman", "docker", "podman-compose", "docker-compose"];

/// Every `<target>: <line>` whose line names a container runtime as a word, wherever in the line
/// it stands: first, after `cd … &&`, or inside a pipeline.
fn bare_container_runtime_lines(targets: &[&str]) -> Vec<String> {
    let mut offences = Vec::new();
    for target in targets {
        for line in recipe_lines(target).unwrap_or_default() {
            let named = line
                .split(|c: char| c.is_whitespace() || ";|&()<>\"'`\\".contains(c))
                .any(|word| CONTAINER_RUNTIMES.contains(&word));
            if named {
                offences.push(format!("{target}: {line}"));
            }
        }
    }
    offences
}

/// The database lane resolves the container runtime (`TBD_CONTAINER_RUNTIME`, `podman`, `docker`,
/// or either through the distrobox bridge). A recipe line that names a runtime itself fails
/// wherever the runtime is reachable only through the bridge, so every recipe reaches the database
/// through the database lane instead.
#[test]
fn no_recipe_line_names_a_bare_container_runtime() {
    for target in TARGETS {
        let computed = matches!(
            *target,
            "print-cargo-target-dir" | "verify-cargo-target" | "reclaim-target-ci"
        );
        assert_eq!(
            recipe_lines(target).is_some(),
            !computed,
            "{target}: every recipe target, and only those, lists its lines"
        );
    }
    let offences = bare_container_runtime_lines(TARGETS);
    assert!(
        offences.is_empty(),
        "these recipe lines run a container runtime themselves instead of the database lane:\n{}",
        offences.join("\n")
    );
}

/// `rust-ci`'s integration-test step and the `rust-test-it` row of `cargo xtask ci` are one
/// command spelled twice: the same line, and the ci row runs the database lane's own function.
#[test]
fn rust_ci_and_the_ci_rust_test_it_row_run_the_database_lane() {
    use crate::task_runner::{Step as CiStep, TASKS};
    let row = TASKS
        .iter()
        .find(|t| t.name == "rust-test-it")
        .expect("the ci task table has a rust-test-it row");
    let [CiStep::Xtask { echo, run, .. }] = row.steps else {
        panic!("the rust-test-it row is one in-process xtask step");
    };
    assert!(std::ptr::fn_addr_eq(
        *run,
        crate::task_runner::run_database_test_suite as fn() -> Result<u8>
    ));
    assert_eq!(rust_ci_lines().last().map(String::as_str), Some(*echo));
}
