//! Executable ownership, documentation and file-size rules for the ticketboard model crate.

use crate::test_support::source_inspection::{
    dependencies, is_test, resolve_path, rust_sources, tokens,
};
use std::path::{Path, PathBuf};

const MODULES: &[&str] = &[
    "application_state",
    "core",
    "ticket_registry",
    "ticket_browser",
    "ticket_actions",
    "wave_plan",
    "execution_metrics",
    "document_viewer",
    "repository_status",
];

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

#[test]
fn model_sources_respect_the_size_limits_without_exemptions() {
    let sources = rust_sources(&source_root());
    assert!(
        !sources.is_empty(),
        "the source scan must not pass vacuously"
    );
    let mut failures = Vec::new();
    for (path, text) in sources {
        let maximum = if is_test(&path) { 1000 } else { 499 };
        let actual = text.lines().count();
        if actual > maximum {
            failures.push(format!(
                "{}: {actual} lines, maximum {maximum}",
                path.display()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn model_module_roots_and_documentation_describe_the_entire_source_tree() {
    let root = source_root();
    assert!(root.parent().unwrap().join("README.md").is_file());
    for entry in std::fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(
            if path.is_dir() {
                MODULES.contains(&name) || name == "tests"
            } else {
                matches!(name, "lib.rs" | "prelude.rs" | "error.rs" | "README.md")
            },
            "unexpected flat source or module: {}",
            path.display()
        );
    }
    for name in MODULES {
        let module = root.join(name);
        assert!(
            module.join("mod.rs").is_file(),
            "missing module root: {name}"
        );
        assert!(
            module.join("README.md").is_file(),
            "missing module README: {name}"
        );
    }
}

#[test]
fn model_dependency_boundaries_and_external_test_placement_are_enforced() {
    let root = source_root();
    let mut failures = Vec::new();
    for (path, text) in rust_sources(&root) {
        if is_test(&path) {
            continue;
        }
        let relative = path.strip_prefix(&root).unwrap();
        let feature = relative
            .components()
            .next()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap();
        let code = tokens(&text);
        if code
            .windows(3)
            .any(|window| window[0] == "mod" && window[2] == "{")
            || code
                .windows(3)
                .any(|window| window[0] == "[" && window[1] == "test" && window[2] == "]")
        {
            failures.push(format!(
                "{}: modules and unit tests live in separate files",
                relative.display()
            ));
        }
        if code.iter().any(|token| {
            ["egui", "eframe", "egui_extras", "egui_commonmark", "rfd"].contains(&token.as_str())
        }) {
            failures.push(format!(
                "{}: the model crate depends on rendering",
                relative.display()
            ));
        }
        for dependency in dependencies(&code) {
            let dependency = resolve_path(relative, &dependency);
            let Some(target) = dependency.first().map(String::as_str) else {
                continue;
            };
            let violation = if feature == "core"
                && ((MODULES.contains(&target) && target != "core") || target == "ticket_model")
            {
                Some("core depends on a feature or ticket domain")
            } else if MODULES.contains(&feature)
                && feature != "application_state"
                && target == "application_state"
            {
                Some("feature depends on application state")
            } else if feature == "ticket_registry"
                && MODULES.contains(&target)
                && !["ticket_registry", "core"].contains(&target)
            {
                Some("registry depends on a consuming feature")
            } else {
                None
            };
            if let Some(reason) = violation {
                failures.push(format!(
                    "{}: {reason}: {}",
                    relative.display(),
                    dependency.join("::")
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn source_inspection_handles_grouped_imports_aliases_and_ignored_prose() {
    let source = r###"
        // use crate::application::State;
        /* outer /* use crate::application; */ comment */
        const NOTE: &str = r#"use crate::application;"#;
        use crate::{ticket_browser::{models::View, ui as forbidden_ui}, core::process};
        use super::super::services::load;
        fn action() { crate::document_viewer::models::State::Closed; }
    "###;
    let code = tokens(source);
    let paths = dependencies(&code);
    assert!(!paths.iter().flatten().any(|part| part == "application"));
    assert!(paths.contains(&vec!["crate".into(), "ticket_browser".into(), "ui".into()]));
    assert!(paths.contains(&vec!["crate".into(), "core".into(), "process".into()]));
    assert_eq!(
        resolve_path(
            Path::new("ticket_browser/ui/cards.rs"),
            &[
                "super".into(),
                "super".into(),
                "services".into(),
                "load".into()
            ]
        ),
        ["ticket_browser", "services", "load"]
    );
}
