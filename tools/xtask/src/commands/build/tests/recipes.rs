//! Unit tests for [`crate::commands::build::recipes`] + [`crate::core::cargo_target_directory`] (SIZE split — see mk_target_dir).
//!
//! Attached to `mk_target_dir` because the RED arms it exists for (the pin marker, the worktree-
//! local reversal, the reclaim refusals) are that module's, and the recipe assertions reach across
//! with an explicit `crate::commands::build::recipes::*`.

use super::*;
use crate::commands::build::recipes::*;

/// The self-reference, pinned. The fixture is `include_str!` of this very file, so it is
/// DERIVED from [`PIN_SOURCE_MARKER`] so the two cannot drift.
#[test]
fn pin_marker_is_present_in_this_file() {
    let src = include_str!("../../../core/cargo_target_directory.rs");
    assert!(
        src.lines()
            .any(|l| l.contains(PIN_SOURCE_MARKER) && !l.contains("PIN_SOURCE_MARKER")),
        "the shared-pin formula `{PIN_SOURCE_MARKER}` left {PIN_SOURCE_REL}; \
         verify-cargo-target would evaporate"
    );
}

/// RED arm for §1: a tree whose target-directory source no longer computes the pin must FAIL.
#[test]
fn pin_marker_verdict_fails_when_the_formula_is_gone() {
    let dir = std::env::temp_dir().join(format!("t895-pin-{}", std::process::id()));
    let src = dir.join(PIN_SOURCE_REL);
    std::fs::create_dir_all(src.parent().unwrap()).unwrap();
    std::fs::write(&src, "fn primary_root() {}\n").unwrap();
    assert!(matches!(pin_marker_verdict(&dir), Verdict::Failed(_)));
    // …and a missing file is DidNotRun, never Held.
    std::fs::remove_file(&src).unwrap();
    assert!(matches!(pin_marker_verdict(&dir), Verdict::DidNotRun(..)));
    let _ = std::fs::remove_dir_all(&dir);
}

/// `?=`: an environment pin wins; otherwise the primary repo's `target/`.
#[test]
fn resolve_target_dir_follows_make_question_equals() {
    assert_eq!(resolve_target_dir(Some("/tmp/x")), "/tmp/x");
    // Empty is unset — make's `?=` treats a defined-but-empty variable as set, but the
    // Makefile's own `verify-cargo-target` rejected an empty result, so empty falls back here.
    assert_eq!(resolve_target_dir(Some("")), resolve_target_dir(None));
    assert_eq!(
        resolve_target_dir(None),
        primary_root().join("target").display().to_string()
    );
}

/// The invariant the `.cargo/config.toml` reversal would break: inside a linked worktree the
/// pin must be the PRIMARY repo's target, never this worktree's.
#[test]
fn pin_points_at_the_primary_repo_not_the_worktree() {
    let here = cwd_root();
    let primary = primary_root();
    if here == primary {
        return; // not a linked worktree; nothing to distinguish
    }
    assert_ne!(
        resolve_target_dir(None),
        here.join("target").display().to_string()
    );
}

/// §4 RED: the `.cargo/config.toml relative = true` reversal, and the three shapes that are
/// NOT it. This is the arm the Makefile could not express at all.
#[test]
fn worktree_local_pin_is_detected() {
    let wt = Path::new("/repo/.ai/artifacts/worktrees/T-895");
    let primary = Path::new("/repo");
    // The reversal: a linked worktree resolving to its own target/.
    assert!(pin_is_worktree_local(
        "/repo/.ai/artifacts/worktrees/T-895/target",
        wt,
        primary
    ));
    // Correct: the worktree resolves to the PRIMARY repo's target/.
    assert!(!pin_is_worktree_local("/repo/target", wt, primary));
    // In the primary checkout the two roots coincide, so `<here>/target` is the right answer
    // and must not be reported — the Makefile ran there, which is how the reversal hides.
    assert!(!pin_is_worktree_local("/repo/target", primary, primary));
    // A private dir is not the shared pin, but it is also not this check's business.
    assert!(!pin_is_worktree_local(
        "/repo/target/gate-check",
        wt,
        primary
    ));
}

/// §5 RED: a `rust-build` that set its own dir must be reported, with the offending line.
#[test]
fn private_target_dir_violation_bites() {
    assert_eq!(private_target_dir_violation(&rust_build()), None);
    let bad = vec![
        Step::new(&["cargo", "build", "--all-targets"])
            .cd(WEB)
            .env("CARGO_TARGET_DIR", "/tmp/private"),
    ];
    assert_eq!(
        private_target_dir_violation(&bad).as_deref(),
        Some("cd apps/api && CARGO_TARGET_DIR=/tmp/private cargo build --all-targets")
    );
}

/// `rust-build` inherits; `rust-api` keeps its private dir inside this checkout's `target/`.
/// §5 of the gate, as a unit test.
#[test]
fn only_rust_api_sets_a_private_target_dir() {
    assert!(private_target_dir_violation(&rust_build()).is_none());
    let api = rust_api();
    let v = api[0]
        .recipe_env("CARGO_TARGET_DIR")
        .expect("rust-api keeps target/dev-api");
    assert!(v.ends_with("/target/dev-api"), "{v}");
    // …and it is relative to this checkout, not the primary one: two roots, never one.
    assert_eq!(
        v,
        cwd_root()
            .join("target")
            .join("dev-api")
            .display()
            .to_string()
    );
    assert_eq!(v, dev_api_target_dir().display().to_string());
}

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
    use crate::commands::ci::task_runner::{Step as CiStep, TASKS};
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

/// `reclaim-target-ci` deletes `target/ci` and the retired root-level `target-ci`, and **leaves
/// the shared cache, its other purpose folders and a live slice's dir alone**.
#[test]
fn reclaim_never_touches_a_live_target_dir() {
    let root = std::env::temp_dir().join(format!("t895-reclaim-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let gone = ["target/ci", "target-ci"];
    let kept = [
        "target",
        "target/dev-api",
        "target/gate-check",
        "target-ctr",
        "target-T-454",
    ];
    for d in gone.iter().chain(kept.iter()) {
        std::fs::create_dir_all(root.join(d)).unwrap();
        std::fs::write(root.join(d).join("live"), "x").unwrap();
    }
    assert_eq!(reclaim_target_ci(&root).unwrap(), 0);
    for d in gone {
        assert!(!root.join(d).exists(), "{d} must be gone");
    }
    for d in kept {
        assert!(root.join(d).join("live").is_file(), "{d} must survive");
    }
    // Idempotent: a second run reports absence, rc 0.
    assert_eq!(reclaim_target_ci(&root).unwrap(), 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// Both `REFUSING:` guards. An empty root makes the relative paths `target/ci` and `target-ci`,
/// which fail the shape test; the collision test cannot fire for any root, and that is pinned so a
/// refactor that derives one path from the other has to confront this test.
#[test]
fn reclaim_refusals_are_preserved() {
    assert_eq!(reclaim_target_ci(Path::new("")).unwrap(), 1);
    for x in ["/a", "/a/b", "", "/"] {
        let p = Path::new(x);
        assert_ne!(p.join("target/ci"), p.join("target"));
        assert_ne!(p.join("target-ci"), p.join("target"));
    }
}

/// The ABI guard refuses a foreign stamp and is silent on its own.
#[test]
fn abi_guard_refuses_a_foreign_stamp() {
    let dir = std::env::temp_dir().join(format!("t895-abi-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // First use stamps and allows.
    assert!(abi_guard(&dir).is_ok());
    assert!(dir.join(".tbd-build-abi").is_file());
    assert!(abi_guard(&dir).is_ok());
    // A different ABI is refused, by name.
    std::fs::write(dir.join(".tbd-build-abi"), "glibc2.99-host\n").unwrap();
    let err = abi_guard(&dir).unwrap_err();
    assert!(err.contains("glibc2.99-host") && err.contains(&abi_id()));
    let _ = std::fs::remove_dir_all(&dir);
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
    use crate::commands::ci::task_runner::{Step as CiStep, TASKS};
    let row = TASKS
        .iter()
        .find(|t| t.name == "rust-test-it")
        .expect("the ci task table has a rust-test-it row");
    let [CiStep::Xtask { echo, run, .. }] = row.steps else {
        panic!("the rust-test-it row is one in-process xtask step");
    };
    assert!(std::ptr::fn_addr_eq(
        *run,
        crate::commands::db::operations::test_it::run_complete_suite as fn() -> Result<u8>
    ));
    assert_eq!(rust_ci_lines().last().map(String::as_str), Some(*echo));
}
