//! The drag-move commit of the pointer-up handler, read from its source.
//!
//! **Role:** pins that the pointer-up handler commits a drag move of slots and vehicles through
//! the document's atomic mixed move, once, and never as two separate transactions.
//! **Position:** the native unit tests of `input`, mounted from `input/mod.rs`; the handler itself
//! is browser-only, so it is read as text through `frontend_test_support`'s live-code scrub.
//! **Signals & state:** none; pure text checks.
//! **Invariants:** comments, string literals and test modules are scrubbed before any token is
//! looked for, so a mention of a call cannot stand in for the call.

use frontend_test_support::class_r_scrub::live_code;

/// The `LG::Move` arms of `pointer_up.rs` that commit through `.move_entities_and_vehicles(`.
fn committing_move_arms(source: &str) -> Vec<&str> {
    source
        .split("LG::Move")
        .skip(1)
        .map(|arm| arm.split("LG::").next().unwrap_or(arm))
        .filter(|arm| arm.contains(".move_entities_and_vehicles("))
        .collect()
}

#[test]
fn pointer_up_move_arm_commits_through_the_atomic_mix_api() {
    let source = live_code(include_str!("../pointer_gestures/pointer_up.rs"));
    let move_arms = committing_move_arms(&source);
    assert!(
        move_arms.len() == 1,
        "expected exactly one LG::Move arm committing via `.move_entities_and_vehicles(` \
         (found {}) — the atomic mixed-move commit was forked, duplicated or deleted",
        move_arms.len()
    );
    let move_arm = move_arms[0];
    assert!(
        !move_arm.contains("core.move_entities("),
        "Move arm calls move_entities alone (two-txn defect)"
    );
    assert!(
        !move_arm.contains("editor_ops::move_vehicles"),
        "Move arm calls editor_ops::move_vehicles (second txn)"
    );
}
