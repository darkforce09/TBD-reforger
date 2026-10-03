//! Executable ownership, documentation, and file-size rules for the ticket board's egui half.

use std::path::{Path, PathBuf};
use ticketboard_model::test_support::source_inspection::{
    dependencies, is_test, resolve_path, rust_sources, tokens,
};

const MODULES: &[&str] = &[
    "application",
    "core",
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
fn source_files_respect_the_size_limits_without_exemptions() {
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
fn module_roots_and_documentation_describe_the_entire_source_tree() {
    let root = source_root();
    assert!(root.parent().unwrap().join("README.md").is_file());
    for entry in std::fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert!(
            if path.is_dir() {
                MODULES.contains(&name) || name == "tests"
            } else {
                matches!(name, "main.rs" | "README.md")
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
        if *name == "application" {
            continue;
        }
        // A feature keeps only its views here; its models, services and events live in
        // `ticketboard_model`.
        for entry in std::fs::read_dir(&module).unwrap() {
            let path = entry.unwrap().path();
            let child = path.file_name().unwrap().to_str().unwrap();
            assert!(
                matches!(child, "mod.rs" | "README.md" | "ui"),
                "headless code belongs in ticketboard_model: {}",
                path.display()
            );
        }
    }
}

#[test]
fn dependency_boundaries_and_external_test_placement_are_enforced() {
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
        for dependency in dependencies(&code) {
            let dependency = resolve_path(relative, &dependency);
            let Some(target) = dependency.first().map(String::as_str) else {
                continue;
            };
            let violation = if feature == "core"
                && ((MODULES.contains(&target) && target != "core") || target == "ticket_model")
            {
                Some("core depends on a feature or ticket domain")
            } else if target == "ticketboard_model"
                && dependency.get(1).map(String::as_str) == Some("application_state")
                && feature != "application"
            {
                Some("feature depends on application state")
            } else if MODULES.contains(&feature)
                && feature != "application"
                && target == "application"
            {
                Some("feature depends on application internals")
            } else if feature != "application"
                && feature != target
                && MODULES.contains(&target)
                && dependency.iter().any(|part| part == "ui")
            {
                // Shared core widgets are the one rendering dependency features may reuse.
                (target != "core").then_some("feature imports another feature's UI")
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
