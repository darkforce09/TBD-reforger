use super::*;

#[test]
fn chrome_map_is_deterministic_no_marker() {
    let scope: toml::Value = "[website.editor]\nchrome = [\"attr\", \"left\"]\n"
        .parse()
        .unwrap();
    let m = map_scope("T-x", &scope, &[]).unwrap();
    assert_eq!(m.domain, "website");
    assert_eq!(m.layer, "frontend");
    assert_eq!(m.component.as_deref(), Some("mission_creator"));
    assert_eq!(m.surface, vec!["attr_panel", "dock_left"]);
    assert!(!m.owns_inferred, "chrome map carries no marker");
}

#[test]
fn editor_owns_inference_dominant_component() {
    // Two mission_creator votes vs one shell vote → mission_creator, its surfaces only.
    let (c, s) = infer_editor(&owns(&[
        "apps/website/frontend/src/mission_editor.rs",
        "apps/website/frontend/src/editor_ops.rs",
        "apps/website/frontend/src/router.rs",
    ]));
    assert_eq!(c, "mission_creator");
    assert_eq!(s, vec!["map_canvas", "ops_undo"]);
    // Shell dominance flips the component and drops mission_creator surfaces.
    let (c, s) = infer_editor(&owns(&[
        "apps/website/frontend/src/router.rs",
        "apps/website/frontend/src/app_routes.rs",
        "apps/website/frontend/src/mission_editor.rs",
    ]));
    assert_eq!(c, "shell");
    assert_eq!(s, vec!["router"]);
    // No table match → mission_creator, surface-empty (the marker population).
    let (c, s) = infer_editor(&owns(&["apps/website/frontend/src/arsenal.rs"]));
    assert_eq!(c, "mission_creator");
    assert!(s.is_empty());
}

#[test]
fn chromeless_editor_is_owns_inferred_marked() {
    let scope: toml::Value = "[website.editor]\nchrome = []\n".parse().unwrap();
    let m = map_scope(
        "T-x",
        &scope,
        &owns(&["apps/website/frontend/src/attributes.rs"]),
    )
    .unwrap();
    assert_eq!(m.surface, vec!["attr_panel"]);
    assert!(m.owns_inferred, "owns-inference must be recorded");
}

#[test]
fn mod_feature_infers_from_enfusion_segments() {
    let scope: toml::Value = "[mod]\nlayers = [\"feature\"]\n".parse().unwrap();
    let m = map_scope(
        "T-x",
        &scope,
        &owns(&[
            "apps/mod/tbd-framework/Scripts/Game/TBD/Markers/TBD_MarkerData.c",
            "apps/mod/tbd-framework/Scripts/Game/TBD/Markers/TBD_MarkerClient.c",
            "apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_Api.c",
        ]),
    )
    .unwrap();
    assert_eq!(m.layer, "scripts");
    assert_eq!(m.component.as_deref(), Some("markers"));
    assert!(m.owns_inferred);
    // Unvoted paths (AI) fall back to scripts/None, still marked.
    let m = map_scope(
        "T-y",
        &scope,
        &owns(&["apps/mod/tbd-framework/Scripts/Game/TBD/AI/TBD_WaypointRuntime.c"]),
    )
    .unwrap();
    assert_eq!(m.layer, "scripts");
    assert_eq!(m.component, None);
    assert!(m.owns_inferred);
    // Explicit layers stay deterministic.
    let scope: toml::Value = "[mod]\nlayers = [\"backend\"]\n".parse().unwrap();
    let m = map_scope("T-z", &scope, &[]).unwrap();
    assert_eq!(
        (m.layer.as_str(), m.component.as_deref()),
        ("scripts", Some("backend"))
    );
    assert!(!m.owns_inferred);
}

#[test]
fn repo_xtask_component_prefix_rules() {
    assert_eq!(
        infer_xtask(&owns(&["xtask/src/wave_lock.rs", "xtask/src/wave/plan.rs"])),
        Some("wave".into())
    );
    assert_eq!(
        infer_xtask(&owns(&["xtask/src/cmds.rs", "xtask/src/tickets_store.rs"])),
        Some("tickets".into())
    );
    assert_eq!(
        infer_xtask(&owns(&["xtask/src/gate_mod_compile.rs"])),
        Some("gates".into())
    );
    assert_eq!(
        infer_xtask(&owns(&[
            "xtask/src/mk_ci_tasks.rs",
            "xtask/src/verify_ci_shell.rs"
        ])),
        Some("ci".into())
    );
    assert_eq!(infer_xtask(&owns(&["xtask/src/hostrun.rs"])), None);
    let scope: toml::Value = "[repo]\nlayers = [\"docs\"]\n".parse().unwrap();
    let m = map_scope("T-x", &scope, &[]).unwrap();
    assert_eq!(
        (m.domain, m.layer.as_str(), m.component),
        ("repo", "docs", None)
    );
    assert!(!m.owns_inferred, "repo/docs is a deterministic layer map");
}

#[test]
fn multi_layer_takes_first() {
    let scope: toml::Value = "[repo]\nlayers = [\"xtask\", \"tickets\"]\n"
        .parse()
        .unwrap();
    let m = map_scope(
        "T-916.2",
        &scope,
        &owns(&["crates/tbd-tickets", "xtask/src"]),
    )
    .unwrap();
    assert_eq!(m.layer, "xtask");
    assert!(m.owns_inferred);
}

#[test]
fn unmapped_shapes_refuse_naming_ticket() {
    let scope: toml::Value = "[galaxy]\nlayers = [\"far\"]\n".parse().unwrap();
    let err = map_scope("T-404", &scope, &[]).unwrap_err();
    assert!(format!("{err:#}").contains("T-404"), "{err:#}");
}
