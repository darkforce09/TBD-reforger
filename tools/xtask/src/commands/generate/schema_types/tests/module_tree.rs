//! The module tree: every folder between the generated root and a schema module gets a `mod.rs`
//! declaring exactly its children, and a schema module never doubles as a folder.
use super::*;

fn unformatted(source: &str) -> Result<String> {
    Ok(source.to_string())
}

const TARGETS: [(&str, &str); 4] = [
    ("event-hub.schema.json", "operations/event_hub"),
    ("viewer/dataset.schema.json", "content/viewer/dataset"),
    ("viewer/resources.schema.json", "content/viewer/resources"),
    ("wiki-page.schema.json", "content/wiki_page"),
];

#[test]
fn every_folder_declares_exactly_its_children() {
    let files = render_tree_files(&TARGETS, &unformatted).unwrap();
    let keys: Vec<&str> = files.keys().map(String::as_str).collect();
    assert_eq!(
        keys,
        [
            "content/mod.rs",
            "content/viewer/mod.rs",
            "mod.rs",
            "operations/mod.rs"
        ]
    );
    let declared = |key: &str| -> Vec<String> {
        files[key]
            .lines()
            .filter_map(|line| line.strip_prefix("pub mod "))
            .map(|name| name.trim_end_matches(';').to_string())
            .collect()
    };
    assert_eq!(declared("mod.rs"), ["content", "operations"]);
    assert_eq!(declared("content/mod.rs"), ["viewer", "wiki_page"]);
    assert_eq!(declared("content/viewer/mod.rs"), ["dataset", "resources"]);
    assert_eq!(declared("operations/mod.rs"), ["event_hub"]);
}

#[test]
fn every_tree_file_names_the_command_and_documents_each_child() {
    let files = render_tree_files(&TARGETS, &unformatted).unwrap();
    for (key, source) in &files {
        assert!(
            source.starts_with(
                "// Code generated from JSON Schema using `cargo xtask schema codegen`"
            ),
            "{key}"
        );
        assert!(source.contains("cargo xtask ci schema-codegen"), "{key}");
        let lines: Vec<&str> = source.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if line.starts_with("pub mod ") {
                assert!(lines[index - 1].starts_with("/// "), "{key}: {line}");
            }
        }
    }
    assert!(
        files["content/viewer/mod.rs"].contains("`contracts/definitions/viewer/`"),
        "a folder of one schema subfolder names it"
    );
    assert!(
        files["content/mod.rs"]
            .contains("/// Types generated from `contracts/definitions/wiki-page.schema.json`.")
    );
}

#[test]
fn a_schema_module_never_doubles_as_a_folder() {
    let nested = [
        ("event-hub.schema.json", "operations/event_hub"),
        ("event-orbat.schema.json", "operations/event_hub/orbat"),
    ];
    let error = render_tree_files(&nested, &unformatted)
        .unwrap_err()
        .to_string();
    assert!(error.contains("would also hold"), "{error}");
    let shared = [
        ("event-hub.schema.json", "operations/event_hub"),
        ("event-orbat.schema.json", "operations/event_hub"),
    ];
    assert!(render_tree_files(&shared, &unformatted).is_err());
}
