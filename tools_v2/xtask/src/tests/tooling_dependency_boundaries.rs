use std::fs;
use toml::Value;

fn rejects_dependency(value: &Value, forbidden: &str) {
    let Some(table) = value.as_table() else {
        return;
    };
    for (key, value) in table {
        if matches!(
            key.as_str(),
            "dependencies" | "dev-dependencies" | "build-dependencies"
        ) {
            for (alias, specification) in value.as_table().expect("dependency table") {
                let package = specification
                    .get("package")
                    .and_then(Value::as_str)
                    .unwrap_or(alias);
                assert_ne!(package, forbidden, "forbidden dependency: {alias}");
            }
        } else {
            rejects_dependency(value, forbidden);
        }
    }
}

#[test]
fn tooling_dependency_direction_is_enforced() {
    let root = crate::root::test_repo_root();
    let read = |name: &str| -> Value {
        toml::from_str(
            &fs::read_to_string(root.join(format!("tools_v2/{name}/Cargo.toml"))).unwrap(),
        )
        .unwrap()
    };
    rejects_dependency(&read("xtask"), "website-map-engine");
    rejects_dependency(&read("xtask"), "website-graphics-engine");
    rejects_dependency(&read("developer-tools"), "xtask");
    assert_eq!(
        read("xtask")["dependencies"]["developer-tools"]["path"].as_str(),
        Some("../developer-tools")
    );
}

#[test]
fn heavy_package_has_one_owner_and_preserves_executable_names() {
    let root = crate::root::test_repo_root();
    let manifest: Value = toml::from_str(
        &fs::read_to_string(root.join("tools_v2/developer-tools/Cargo.toml")).unwrap(),
    )
    .unwrap();
    let mut names: Vec<_> = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .map(|bin| bin["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["capture", "enf", "gate", "map", "mcpd", "world"]);
    assert!(!root.join("tools/tbd-tools/Cargo.toml").exists());
    assert!(!root.join("tools_v2/xtask/src/map_blueprint").exists());
    assert!(
        !root
            .join("tools_v2/developer-tools/src/blueprint/pak.rs")
            .exists()
    );
    assert!(
        !root
            .join("tools_v2/developer-tools/src/world/pak.rs")
            .exists()
    );
}

#[test]
fn ticket_engine_has_no_workspace_dependencies() {
    let root = crate::root::test_repo_root();
    let workspace: Value =
        toml::from_str(&fs::read_to_string(root.join("Cargo.toml")).unwrap()).unwrap();
    let engine: Value = toml::from_str(
        &fs::read_to_string(root.join("tools_v2/ticket-engine/Cargo.toml")).unwrap(),
    )
    .unwrap();
    for member in workspace["workspace"]["members"].as_array().unwrap() {
        let manifest: Value = toml::from_str(
            &fs::read_to_string(root.join(member.as_str().unwrap()).join("Cargo.toml")).unwrap(),
        )
        .unwrap();
        rejects_dependency(&engine, manifest["package"]["name"].as_str().unwrap());
    }
}

#[test]
fn ticket_implementations_have_one_owner() {
    let root = crate::root::test_repo_root();
    for module in [
        "check",
        "cmds",
        "sync",
        "wave_lock",
        "metrics",
        "registry",
        "tickets_store",
        "phase2",
        "estimate_tokens",
        "backfill_stamps",
        "migrate_v2",
        "migrate_main_goal",
        "quarantine_walls",
        "vocab_check",
        "slice_collisions",
        "gap",
        "prompt",
    ] {
        assert!(
            !root
                .join(format!("tools_v2/xtask/src/{module}.rs"))
                .exists(),
            "duplicate ticket owner: {module}"
        );
    }
    for adapter in ["ticket", "wave"] {
        let source =
            fs::read_to_string(root.join(format!("tools_v2/xtask/src/commands/{adapter}/mod.rs")))
                .unwrap();
        assert!(
            source.contains("ticket_engine::"),
            "adapter must delegate to ticket-engine"
        );
    }
}
