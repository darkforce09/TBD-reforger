use repository_laws::workspace_members::read_workspace_members;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use syn::parse::{ParseStream, Parser};
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
    let root = crate::core::repository_root::test_repo_root();
    let read = |name: &str| -> Value {
        toml::from_str(&fs::read_to_string(root.join(format!("tools/{name}/Cargo.toml"))).unwrap())
            .unwrap()
    };
    rejects_dependency(&read("xtask"), "map_engine");
    rejects_dependency(&read("xtask"), "graphics_engine");
    rejects_dependency(&read("developer_tools"), "xtask");
    assert_eq!(
        read("xtask")["dependencies"]["developer_tools"]["path"].as_str(),
        Some("../developer_tools")
    );
}

#[test]
fn the_tooling_tree_holds_its_executables_manifests_and_layout_modules() {
    let root = crate::core::repository_root::test_repo_root();
    let manifest: Value =
        toml::from_str(&fs::read_to_string(root.join("tools/developer_tools/Cargo.toml")).unwrap())
            .unwrap();
    let mut names: Vec<_> = manifest["bin"]
        .as_array()
        .unwrap()
        .iter()
        .map(|bin| bin["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "acknowledgement-dropping-relay",
            "capture",
            "enf",
            "gate",
            "map",
            "mcpd",
            "world"
        ]
    );
    for relative in [
        "tools/xtask/Cargo.toml",
        "tools/developer_tools/Cargo.toml",
        "tools/foundation/verification_core/Cargo.toml",
        "tools/foundation/process_runner/Cargo.toml",
        "tools/foundation/repository_laws/Cargo.toml",
        "tools/foundation/repository_layout/Cargo.toml",
        "tools/ticket_engine/Cargo.toml",
        // The crate that owns the repository paths the tools share, the three modules that own
        // the paths only one tool spells, and the node package that sits outside every crate root
        // so no crate walk treats it as source.
        "tools/foundation/repository_layout/src/lib.rs",
        "tools/ticket_engine/src/repository.rs",
        "tools/xtask/src/core/repository_layout.rs",
        "tools/developer_tools/src/repository_layout.rs",
        "tools/enfusion_mcp_node_package/package.json",
    ] {
        assert!(root.join(relative).is_file(), "missing: {relative}");
    }
}

/// The folder of the tooling foundation crates.
const TOOL_FOUNDATION: &str = "tools/foundation";

/// Each `tools/foundation` crate depends, among the workspace crates, only on `tools/foundation`
/// crates of a lower declared tier, and `ticket_engine` only on `tools/foundation` crates — in
/// every dependency table.
#[test]
fn foundation_crates_depend_only_on_lower_foundation_crates() {
    let root = crate::core::repository_root::test_repo_root();
    let members = read_workspace_members(&root).unwrap();
    let workspace_packages: Vec<&str> = members
        .iter()
        .map(|member| member.package_name.as_str())
        .collect();
    let foundation: HashMap<&str, u32> = members
        .iter()
        .filter(|member| member.path.starts_with(&format!("{TOOL_FOUNDATION}/")))
        .map(|member| {
            let tier = member
                .manifest
                .layout
                .as_ref()
                .and_then(|layout| layout.tier_number())
                .unwrap_or_else(|| panic!("{}: no declared layout tier", member.path));
            (member.package_name.as_str(), tier)
        })
        .collect();
    assert!(
        foundation.contains_key("verification_core"),
        "no crate under {TOOL_FOUNDATION}: {foundation:?}"
    );
    for member in &members {
        let ceiling = match foundation.get(member.package_name.as_str()) {
            Some(tier) => *tier,
            None if member.package_name == "ticket_engine" => u32::MAX,
            None => continue,
        };
        for edge in &member.manifest.dependencies {
            if !workspace_packages.contains(&edge.package.as_str()) {
                continue;
            }
            let allowed = foundation
                .get(edge.package.as_str())
                .is_some_and(|tier| *tier < ceiling);
            assert!(
                allowed,
                "{}/Cargo.toml:{}: {} depends on {}, which is not a lower {TOOL_FOUNDATION} crate",
                member.path, edge.line_no, member.package_name, edge.package
            );
        }
    }
}

#[test]
fn ticket_implementations_have_one_owner() {
    let root = crate::core::repository_root::test_repo_root();
    for module in [
        "check",
        "cmds",
        "sync",
        "wave_lock",
        "metrics",
        "registry",
        "tickets_store",
        "phase2",
        "vocab_check",
        "slice_collisions",
        "gap",
        "prompt",
    ] {
        assert!(
            !root.join(format!("tools/xtask/src/{module}.rs")).exists(),
            "duplicate ticket owner: {module}"
        );
    }
    for adapter in ["ticket", "wave"] {
        let source =
            fs::read_to_string(root.join(format!("tools/xtask/src/commands/{adapter}/mod.rs")))
                .unwrap();
        assert!(
            source.contains("ticket_engine::"),
            "adapter must delegate to ticket_engine"
        );
    }
}

/// The tooling crates directly under `tools`; every crate under `tools/foundation` joins them from
/// the folder. The structural rules below — line limits, no inline test modules, no file-size
/// exemptions — hold for all of them, with no crate exempt from any of them.
const TOOLING_CRATES: [&str; 3] = ["xtask", "developer_tools", "ticket_engine"];

/// The folder of every tooling crate: [`TOOLING_CRATES`], then each `tools/foundation` folder
/// holding a `Cargo.toml`.
fn tooling_crate_folders(root: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = TOOLING_CRATES
        .iter()
        .map(|name| root.join("tools").join(name))
        .collect();
    let foundation = root.join(TOOL_FOUNDATION);
    let mut found: Vec<PathBuf> = fs::read_dir(&foundation)
        .unwrap_or_else(|error| panic!("cannot list {}: {error}", foundation.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|folder| folder.join("Cargo.toml").is_file())
        .collect();
    assert!(!found.is_empty(), "no crate under {}", foundation.display());
    found.sort();
    folders.extend(found);
    folders
}

fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for folder in tooling_crate_folders(root) {
        let source = folder.join("src");
        assert!(source.is_dir(), "missing source tree: {}", source.display());
        let before = paths.len();
        for entry in walkdir::WalkDir::new(&source) {
            let entry = entry.unwrap_or_else(|error| panic!("source walk failed: {error}"));
            assert!(
                !entry.file_type().is_symlink(),
                "source walk cannot skip symlinks: {}",
                entry.path().display()
            );
            if entry.file_type().is_file()
                && entry.path().extension().is_some_and(|ext| ext == "rs")
            {
                paths.push(entry.into_path());
            }
        }
        assert!(
            paths.len() > before,
            "empty source tree: {}",
            source.display()
        );
    }
    paths.sort();
    paths
}

fn separate_test_file(path: &Path) -> bool {
    if path.ends_with("browser_testing/editor_smoke_tests.rs") {
        return false;
    }
    path.components().any(|part| part.as_os_str() == "tests")
        || path.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name == "tests.rs" || name.ends_with("_tests.rs")
        })
}

fn exclusive_line_limit(path: &Path) -> usize {
    if separate_test_file(path) {
        1000
    } else if path == Path::new("tools/xtask/src/main.rs") {
        150
    } else if path.starts_with("tools/developer_tools/src/bin") {
        250
    } else if path.starts_with("tools/developer_tools/src/browser_testing/editor_smoke_tests")
        || path.ends_with("browser_testing/editor_smoke_tests.rs")
    {
        450
    } else {
        500
    }
}

#[test]
fn tooling_source_files_stay_below_their_structural_limits() {
    let root = crate::core::repository_root::test_repo_root();
    let mut violations = Vec::new();
    for path in rust_sources(&root) {
        let relative = path.strip_prefix(&root).unwrap();
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let lines = source.lines().count();
        let limit = exclusive_line_limit(relative);
        if lines >= limit {
            violations.push(format!(
                "{}: {lines} lines, must be <{limit}",
                relative.display()
            ));
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

fn condition_enables_tests(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) if list.path.is_ident("not") => false,
        syn::Meta::List(list) => {
            let nested = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
                .parse2(list.tokens.clone())
                .expect("valid cfg condition");
            nested.iter().any(condition_enables_tests)
        }
        syn::Meta::NameValue(_) => false,
    }
}

fn attribute_enables_tests(meta: &syn::Meta) -> bool {
    let syn::Meta::List(list) = meta else {
        return false;
    };
    if list.path.is_ident("cfg") {
        condition_enables_tests(meta)
    } else if list.path.is_ident("cfg_attr") {
        let nested = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())
            .expect("valid cfg_attr attribute");
        nested.iter().skip(1).any(attribute_enables_tests)
    } else {
        false
    }
}

// Walk token groups as well as modules so locally declared modules inside functions
// and macro bodies receive the same check. Rust parsing excludes comments and strings.
fn inline_test_modules(input: ParseStream<'_>) -> syn::Result<Vec<String>> {
    let mut found = Vec::new();
    while !input.is_empty() {
        if let Ok(module) = input.fork().parse::<syn::ItemMod>()
            && module.content.is_some()
            && module
                .attrs
                .iter()
                .any(|attribute| attribute_enables_tests(&attribute.meta))
        {
            found.push(module.ident.to_string());
        }
        if input.peek(syn::token::Brace) {
            let inner;
            syn::braced!(inner in input);
            found.extend(inline_test_modules(&inner)?);
        } else if input.peek(syn::token::Paren) {
            let inner;
            syn::parenthesized!(inner in input);
            found.extend(inline_test_modules(&inner)?);
        } else if input.peek(syn::token::Bracket) {
            let inner;
            syn::bracketed!(inner in input);
            found.extend(inline_test_modules(&inner)?);
        } else {
            input.step(|cursor| {
                cursor
                    .token_tree()
                    .map(|(_, next)| ((), next))
                    .ok_or_else(|| cursor.error("expected Rust token"))
            })?;
        }
    }
    found.sort();
    found.dedup();
    Ok(found)
}

#[test]
fn tooling_test_modules_live_in_separate_files() {
    let root = crate::core::repository_root::test_repo_root();
    let mut violations = Vec::new();
    for path in rust_sources(&root) {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        syn::parse_file(&source)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
        let modules = inline_test_modules
            .parse_str(&source)
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", path.display()));
        if !modules.is_empty() {
            violations.push(format!("{}: {}", path.display(), modules.join(", ")));
        }
    }
    assert!(
        violations.is_empty(),
        "inline test modules:\n{}",
        violations.join("\n")
    );
}

#[test]
fn file_size_allowlist_is_permanently_retired() {
    let root = crate::core::repository_root::test_repo_root();
    assert!(
        !root.join(".coding-standards-allowlist.yaml").exists(),
        ".coding-standards-allowlist.yaml must not exist — file-length allowlisting is permanently retired"
    );
}

#[test]
fn structural_limits_distinguish_scenarios_and_separate_tests() {
    for (path, limit) in [
        ("tools/xtask/src/main.rs", 150),
        ("tools/developer_tools/src/bin/world.rs", 250),
        (
            "tools/developer_tools/src/browser_testing/editor_smoke_tests.rs",
            450,
        ),
        (
            "tools/developer_tools/src/browser_testing/editor_smoke_tests/scenario.rs",
            450,
        ),
        (
            "tools/developer_tools/src/browser_testing/editor_smoke_tests/tests/scenario.rs",
            1000,
        ),
        ("tools/xtask/src/core/runner_tests.rs", 1000),
        ("tools/xtask/src/core/tests.rs", 1000),
        ("tools/xtask/src/core/runner.rs", 500),
    ] {
        assert_eq!(exclusive_line_limit(Path::new(path)), limit, "{path}");
    }
}

#[test]
fn inline_module_detection_handles_nested_syntax_without_matching_source_strings() {
    let source = r##"
        mod production {
            #[cfg(any(target_arch = "wasm32", test))]
            mod forbidden {}
            fn helper() { #[cfg(test)] mod local_tests {} }
        }
        #[cfg(test)] #[path = "tests/external.rs"] mod external;
        #[cfg(not(test))] mod production_only {}
        #[cfg_attr(test, allow(dead_code))] mod conditional_lint {}
        #[cfg_attr(feature = "extra", cfg(test))] mod conditional_tests {}
        const SOURCE: &str = "#[cfg(test)] mod string_content {}";
        // #[cfg(test)] mod comment_content {}
    "##;
    assert_eq!(
        inline_test_modules.parse_str(source).unwrap(),
        ["conditional_tests", "forbidden", "local_tests"]
    );
}
