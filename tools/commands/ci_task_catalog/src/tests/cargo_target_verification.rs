//! Unit tests for [`crate::cargo_target_verification`] and the pin it checks
//! ([`crate::cargo_target_pin`]): the source marker, the worktree-local reversal,
//! the private-directory rule over the recipes, the reclaim refusals and the glibc stamp guard.

use super::*;
use crate::build_lane::recipes::{Step, WEB, rust_api, rust_build};
use crate::cargo_target_pin::*;

/// The source pin, held. The fixture is `include_str!` of the pin's own file, and the needle is
/// [`PIN_SOURCE_MARKER`], so the two cannot drift.
#[test]
fn pin_marker_is_present_in_this_file() {
    let src = include_str!("../cargo_target_pin.rs");
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
    assert_eq!(
        private_target_dir_violation(
            &rust_build(&tool_test_support::test_repo_root())
                .expect("the rust-build recipe derives")
        ),
        None
    );
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
    assert!(
        private_target_dir_violation(
            &rust_build(&tool_test_support::test_repo_root())
                .expect("the rust-build recipe derives")
        )
        .is_none()
    );
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

/// The development API builds into this checkout's own `target/dev-api`, never a root-level
/// sibling of `target/` and never the primary checkout's folder from inside a worktree.
#[test]
fn dev_api_target_dir_is_per_checkout_under_target() {
    let dir = dev_api_target_dir();
    assert_eq!(dir, cwd_root().join("target").join("dev-api"));
    assert!(
        dir.display().to_string().ends_with("/target/dev-api"),
        "{}",
        dir.display()
    );
}
