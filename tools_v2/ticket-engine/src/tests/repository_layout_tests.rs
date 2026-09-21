use super::*;

/// The sparse-checkout sets are keyed by the target vocabulary a ticket's `[scope]` uses, so a
/// target the validator accepts always resolves to a set of paths. Divergence would make
/// `ticket sparse-paths` silently emit a checkout missing the tree the work happens in.
#[test]
fn every_ticket_target_has_a_sparse_checkout_set() {
    let keys: Vec<&str> = SPARSE_CHECKOUT_SETS.iter().map(|(name, _)| *name).collect();
    assert_eq!(keys, crate::validation::constants::VALID_TARGETS);
}

/// A root slice checks out the tooling tree, the registry, the artifacts it writes and the
/// documentation it edits — and nothing that stopped existing.
#[test]
fn the_root_sparse_set_carries_the_task_surface() {
    let (_, root_set) = SPARSE_CHECKOUT_SETS
        .iter()
        .find(|(name, _)| *name == "root")
        .expect("a root target");
    for expected in ["tools_v2", ".cargo", TICKETS_DIR, ARTIFACTS_DIR] {
        assert!(root_set.contains(&expected), "root set lacks {expected}");
    }
}

/// A ticket's plan path is derived, not stored: lowercase id with dots as underscores, under the
/// plans directory.
#[test]
fn a_plan_path_is_the_lowercased_id_under_the_plans_directory() {
    assert_eq!(
        documentation::plan_path("T-917.6"),
        "docs/plans/t-917_6_plan.md"
    );
    assert!(documentation::plan_path("T-090.4").starts_with(documentation::PLANS_DIR));
}

/// A handoff document belongs to the artifact tree, which a gate never reads as input.
#[test]
fn a_handoff_document_lands_in_the_artifact_tree() {
    let path = handoff_doc("t090_1");
    assert!(path.starts_with(ARTIFACTS_DIR), "{path}");
    assert!(path.ends_with("_claude_code_handoff.md"), "{path}");
}
