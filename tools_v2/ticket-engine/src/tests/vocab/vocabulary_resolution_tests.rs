use super::*;

#[test]
fn legality_walks_the_tree() {
    let v = ScopeVocab::parse(MINI).unwrap();
    v.check_scope("T-1", &scope(Domain::Repo, "docs", None, &[]))
        .expect("component-free layer");
    v.check_scope(
        "T-1",
        &scope(
            Domain::Website,
            "frontend",
            Some("mission_creator"),
            &["toolbelt"],
        ),
    )
    .expect("known surface");

    let err = v
        .check_scope("T-2", &scope(Domain::Repo, "nope", None, &[]))
        .unwrap_err();
    assert!(err.contains("T-2") && err.contains("repo.nope"), "{err}");
    let err = v
        .check_scope(
            "T-3",
            &scope(Domain::Website, "frontend", Some("ghost"), &[]),
        )
        .unwrap_err();
    assert!(
        err.contains("T-3") && err.contains("website.frontend.ghost"),
        "{err}"
    );
    let err = v
        .check_scope(
            "T-4",
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
        .check_scope("T-5", &scope(Domain::Engine, "core", None, &[]))
        .unwrap_err();
    assert!(err.contains("domain \"engine\""), "{err}");
}

#[test]
fn missing_file_refuses_naming_path() {
    let dir = std::env::temp_dir().join(format!("t917-vocab-lib-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".ai/tickets")).unwrap();
    let err = ScopeVocab::load(&dir).unwrap_err();
    assert!(err.contains("scope-vocab.toml"), "{err}");
    std::fs::write(dir.join(VOCAB_REL), MINI).unwrap();
    let v = ScopeVocab::load(&dir).expect("present file loads");
    assert_eq!(
        v.surfaces_of("website", "frontend", "mission_creator"),
        Some(&["map_canvas".to_string(), "toolbelt".to_string()][..])
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
