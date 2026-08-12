//! Unit tests for [`crate::mk_build`] + [`crate::mk_target_dir`] (SIZE split — see mk_target_dir).
//!
//! Attached to `mk_target_dir` because the RED arms it exists for (the pin marker, the worktree-
//! local reversal, the reclaim refusals) are that module's, and the recipe assertions reach across
//! with an explicit `crate::mk_build::*`.

use super::*;
use crate::mk_build::*;

/// The self-reference, pinned. The fixture is `include_str!` of this very file, so it is
/// DERIVED from [`PIN_SOURCE_MARKER`] and the two cannot drift — the T-440 lesson.
#[test]
fn pin_marker_is_present_in_this_file() {
    let src = include_str!("mk_target_dir.rs");
    assert!(
        src.lines()
            .any(|l| l.contains(PIN_SOURCE_MARKER) && !l.contains("PIN_SOURCE_MARKER")),
        "the shared-pin formula `{PIN_SOURCE_MARKER}` left {PIN_SOURCE_REL}; \
         verify-cargo-target would evaporate"
    );
}

/// RED arm for §1: a tree whose `mk_target_dir.rs` no longer computes the pin must FAIL, not pass.
#[test]
fn pin_marker_verdict_fails_when_the_formula_is_gone() {
    let dir = std::env::temp_dir().join(format!("t895-pin-{}", std::process::id()));
    let src = dir.join("xtask/src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("mk_target_dir.rs"), "fn primary_root() {}\n").unwrap();
    assert!(matches!(pin_marker_verdict(&dir), Verdict::Failed(_)));
    // …and a missing file is DidNotRun, never Held.
    std::fs::remove_file(src.join("mk_target_dir.rs")).unwrap();
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
        "/repo/target-gate-check",
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
        Some("cd apps/website/api && CARGO_TARGET_DIR=/tmp/private cargo build --all-targets")
    );
}

/// `rust-build` inherits; `rust-api` keeps its private dir. §5 of the gate, as a unit test.
#[test]
fn only_rust_api_sets_a_private_target_dir() {
    assert!(private_target_dir_violation(&rust_build()).is_none());
    let api = rust_api();
    let v = api[0]
        .recipe_env("CARGO_TARGET_DIR")
        .expect("rust-api keeps target-dev-api");
    assert!(v.ends_with(DEV_API_TARGET));
    // …and it is CURDIR-relative, not primary-relative: two roots, never one.
    assert_eq!(v, cwd_root().join(DEV_API_TARGET).display().to_string());
}

/// The echoed lines, pinned against the strings `make -n` printed on 2026-08-12.
#[test]
fn echo_matches_make() {
    assert_eq!(
        rust_build()[0].echo(),
        "cd apps/website/api && cargo build --all-targets"
    );
    assert_eq!(rust_fmt()[1].echo(), "cargo fmt --all --check");
    assert_eq!(
        rust_clippy()[0].echo(),
        "cd apps/website/api && cargo clippy --all-targets -- -D warnings"
    );
    assert_eq!(
        leptos()[0].echo(),
        "cd apps/website/frontend && trunk serve --release"
    );
    assert_eq!(
        ci_local_leptos()[3].echo(),
        "cd apps/website/frontend && trunk build --release"
    );
    assert_eq!(
        wasm_ci()[2].echo(),
        "cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings"
    );
    // The quoted psql argument: make echoed the recipe TEXT, quotes included.
    assert_eq!(
        rust_test_it()[1].echo(),
        "podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc \
         \"CREATE DATABASE rust_it;\""
    );
    assert_eq!(
        rust_api()[0].echo(),
        format!(
            "cd apps/website/api && CARGO_TARGET_DIR={}/{DEV_API_TARGET} cargo run --bin api",
            cwd_root().display()
        )
    );
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

/// `reclaim-target-ci` deletes `target-ci` and **leaves a live slice's dir alone**.
#[test]
fn reclaim_never_touches_a_live_target_dir() {
    let root = std::env::temp_dir().join(format!("t895-reclaim-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for d in ["target-ci", "target", "target-ctr", "target-dev-api"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
        std::fs::write(root.join(d).join("live"), "x").unwrap();
    }
    assert_eq!(reclaim_target_ci(&root).unwrap(), 0);
    assert!(!root.join("target-ci").exists(), "target-ci must be gone");
    for d in ["target", "target-ctr", "target-dev-api"] {
        assert!(root.join(d).join("live").is_file(), "{d} must survive");
    }
    // Idempotent: a second run reports absence, rc 0.
    assert_eq!(reclaim_target_ci(&root).unwrap(), 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// The two `REFUSING:` guards are carried over verbatim from a Makefile in which **neither was
/// reachable** — see [`crate::mk_target_dir::reclaim_target_ci`]. One of them becomes reachable in
/// Rust, and this is the arm that reaches it; without it the port would be shipping two lines of
/// text that nothing has ever executed.
#[test]
fn reclaim_refusals_are_preserved() {
    // Shape: an empty root makes the RELATIVE path `target-ci`, which is not `…/target-ci`.
    // `make` could not produce this — `$(TBD_REPO_ROOT)` empty yields the absolute `/target-ci`.
    assert_eq!(reclaim_target_ci(Path::new("")).unwrap(), 1);
    // Collision: still unreachable, and the assertion says WHY rather than asserting nothing.
    // `X/target-ci` and `X/target` differ for every X, so the branch is structurally dead here as
    // it was in the recipe. Pinned so that a future refactor which makes the two paths equal (a
    // `warm` derived from `ci`, say) has to confront this test.
    for x in ["/a", "/a/b", "", "/"] {
        let p = Path::new(x);
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
/// T-894/T-896 chaining seam silently swallow a target.
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
