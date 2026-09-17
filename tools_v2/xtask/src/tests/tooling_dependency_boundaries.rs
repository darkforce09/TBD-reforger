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
