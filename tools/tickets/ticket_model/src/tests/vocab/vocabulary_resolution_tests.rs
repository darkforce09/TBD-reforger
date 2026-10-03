use super::*;

#[test]
fn legality_walks_the_tree() {
    let v = ScopeVocab::parse(MINI).unwrap();
    v.check_scope(
        &crate::TicketId::from("T-1"),
        &scope(Domain::Repo, "docs", None, &[]),
    )
    .expect("component-free layer");
    v.check_scope(
        &crate::TicketId::from("T-1"),
        &scope(
            Domain::Website,
            "frontend",
            Some("mission_creator"),
            &["toolbelt"],
        ),
    )
    .expect("known surface");

    let err = v
        .check_scope(
            &crate::TicketId::from("T-2"),
            &scope(Domain::Repo, "nope", None, &[]),
        )
        .unwrap_err();
    assert!(err.contains("T-2") && err.contains("repo.nope"), "{err}");
    let err = v
        .check_scope(
            &crate::TicketId::from("T-3"),
            &scope(Domain::Website, "frontend", Some("ghost"), &[]),
        )
        .unwrap_err();
    assert!(
        err.contains("T-3") && err.contains("website.frontend.ghost"),
        "{err}"
    );
    let err = v
        .check_scope(
            &crate::TicketId::from("T-4"),
            &scope(
                Domain::Website,
                "frontend",
                Some("mission_creator"),
                &["dock_left"],
            ),
        )
        .unwrap_err();
    assert!(
        err.contains("T-4") && err.contains("\"dock_left\""),
        "{err}"
    );
    let err = v
        .check_scope(
            &crate::TicketId::from("T-5"),
            &scope(Domain::Engine, "core", None, &[]),
        )
        .unwrap_err();
    assert!(err.contains("domain \"engine\""), "{err}");
}

#[test]
fn missing_file_refuses_naming_path() {
    let dir = std::env::temp_dir().join(format!("t917-vocab-lib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(repository_layout::TICKETS_DIR)).unwrap();
    let err = ScopeVocab::load(&dir).unwrap_err();
    assert!(err.contains("scope-vocab.toml"), "{err}");
    std::fs::write(dir.join(SCOPE_VOCAB), MINI).unwrap();
    let v = ScopeVocab::load(&dir).expect("present file loads");
    assert_eq!(
        v.surfaces_of("website", "frontend", "mission_creator"),
        Some(&["map_canvas".to_string(), "toolbelt".to_string()][..])
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// `domains` exposes the parsed tree as written: domains, layers and components in name order,
/// a component-free layer as an empty map, and each component's surfaces in file order.
#[test]
fn domains_expose_the_tree_in_name_order() {
    let v = ScopeVocab::parse(MINI).unwrap();
    let tree = v.domains();
    assert_eq!(tree.keys().collect::<Vec<_>>(), ["repo", "website"]);
    assert!(tree["repo"]["docs"].is_empty(), "component-free layer");
    assert_eq!(
        tree["website"]["frontend"]["mission_creator"],
        ["map_canvas", "toolbelt"]
    );
    assert!(ScopeVocab::default().domains().is_empty());
}
