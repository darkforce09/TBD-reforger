use super::*;

#[test]
fn tip_registry_passes_schema() {
    let root = worktree_root();
    let registry = load_registry(&root).expect("load tip registry");
    let errs = validate_registry_schema(&root, &registry);
    assert!(
        errs.is_empty(),
        "tip registry must PASS schema; got:\n{}",
        errs.join("\n")
    );
}

#[test]
fn perturbed_ticket_field_fails_schema() {
    let root = worktree_root();
    let mut registry = load_registry(&root).expect("load tip registry");
    let tickets = registry
        .get_mut("tickets")
        .and_then(|t| t.as_array_mut())
        .expect("tickets array");
    let first = tickets.first_mut().expect("at least one ticket");
    first
        .as_object_mut()
        .expect("ticket object")
        .remove("title");
    let errs = validate_registry_schema(&root, &registry);
    assert!(
        !errs.is_empty(),
        "removing required title must make schema check RED"
    );
    assert!(
        errs.iter().any(|e| e.contains("schema")),
        "errors should be schema-tagged: {errs:?}"
    );
}

/// The id pattern admits parents of three or more digits, so the first four-digit id is legal,
/// and still refuses a two-digit parent or a malformed suffix.
#[test]
fn schema_admits_ids_of_three_or_more_digits() {
    let root = worktree_root();
    let registry = load_registry(&root).expect("load tip registry");
    let errors_with_first_id = |id: &str| {
        let mut registry = registry.clone();
        registry
            .get_mut("tickets")
            .and_then(|t| t.as_array_mut())
            .and_then(|t| t.first_mut())
            .and_then(|t| t.as_object_mut())
            .expect("first ticket object")
            .insert("id".into(), json!(id));
        validate_registry_schema(&root, &registry)
    };
    for id in ["T-649", "T-1000", "T-1000.2", "T-12345.6.7"] {
        let errs = errors_with_first_id(id);
        assert!(errs.is_empty(), "{id} must pass: {}", errs.join("\n"));
    }
    for id in ["T-99", "T-99.1", "T-1000.", "T-1000a"] {
        assert!(
            !errors_with_first_id(id).is_empty(),
            "{id} must fail the id pattern"
        );
    }
}
