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
    let root = tool_test_support::test_repo_root();
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
    let root = tool_test_support::test_repo_root();
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
            "staging-load",
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
        "tools/foundation/deploy_settings/Cargo.toml",
        "tools/foundation/tool_test_support/Cargo.toml",
        "tools/tickets/ticket_model/Cargo.toml",
        "tools/tickets/ticket_metrics/Cargo.toml",
        "tools/tickets/ticket_wave_lock/Cargo.toml",
        "tools/tickets/ticket_registry/Cargo.toml",
        "tools/checks/repository_checks/Cargo.toml",
        "tools/checks/mod_script_checks/Cargo.toml",
        // The crate that owns the repository paths the tools share, the modules that own the
        // paths only one tool spells, and the node package that sits outside every crate root
        // so no crate walk treats it as source.
        "tools/foundation/repository_layout/src/lib.rs",
        "tools/tickets/ticket_model/src/repository.rs",
        "tools/developer_tools/src/map_pipeline_layout.rs",
        "tools/enfusion/enfusion_script_index/src/script_index_layout.rs",
        "tools/browser_testing/browser_gate_suites/src/gate_layout.rs",
        "tools/enfusion_mcp_node_package/package.json",
    ] {
        assert!(root.join(relative).is_file(), "missing: {relative}");
    }
}

/// The folder of the tooling foundation crates.
const TOOL_FOUNDATION: &str = "tools/foundation";

/// Each `tools/foundation` crate depends, among the workspace crates, only on `tools/foundation`
/// crates of a lower declared tier — in every dependency table.
#[test]
fn foundation_crates_depend_only_on_lower_foundation_crates() {
    let root = tool_test_support::test_repo_root();
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

/// The folder of the ticket crates.
const TOOL_TICKETS: &str = "tools/tickets";

/// The `crates/foundation` crates a ticket crate may depend on.
const TICKET_CRATE_FOUNDATIONS: [&str; 3] = ["time_source", "content_digest", "newtype_ids"];

/// Each `tools/tickets` crate depends, among the workspace crates, only on `tools/foundation`
/// crates, on [`TICKET_CRATE_FOUNDATIONS`], and on `tools/tickets` crates of a lower declared
/// tier — in every dependency table.
#[test]
fn ticket_crates_depend_only_on_foundations_and_lower_ticket_crates() {
    let root = tool_test_support::test_repo_root();
    let members = read_workspace_members(&root).unwrap();
    let workspace_packages: Vec<&str> = members
        .iter()
        .map(|member| member.package_name.as_str())
        .collect();
    let in_folder = |folder: &str| -> HashMap<&str, u32> {
        members
            .iter()
            .filter(|member| member.path.starts_with(&format!("{folder}/")))
            .map(|member| {
                let tier = member
                    .manifest
                    .layout
                    .as_ref()
                    .and_then(|layout| layout.tier_number())
                    .unwrap_or_else(|| panic!("{}: no declared layout tier", member.path));
                (member.package_name.as_str(), tier)
            })
            .collect()
    };
    let foundation = in_folder(TOOL_FOUNDATION);
    let tickets = in_folder(TOOL_TICKETS);
    assert!(
        tickets.contains_key("ticket_model"),
        "no ticket_model under {TOOL_TICKETS}: {tickets:?}"
    );
    for member in &members {
        let Some(ceiling) = tickets.get(member.package_name.as_str()) else {
            continue;
        };
        for edge in &member.manifest.dependencies {
            let package = edge.package.as_str();
            if !workspace_packages.contains(&package) {
                continue;
            }
            let allowed = foundation.contains_key(package)
                || TICKET_CRATE_FOUNDATIONS.contains(&package)
                || tickets.get(package).is_some_and(|tier| tier < ceiling);
            assert!(
                allowed,
                "{}/Cargo.toml:{}: {} depends on {package}, which is neither a {TOOL_FOUNDATION} \
                 crate, one of {TICKET_CRATE_FOUNDATIONS:?}, nor a lower {TOOL_TICKETS} crate",
                member.path, edge.line_no, member.package_name
            );
        }
    }
}

#[test]
fn ticket_implementations_have_one_owner() {
    let root = tool_test_support::test_repo_root();
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
    for (adapter, owner) in [("ticket", "ticket_registry"), ("wave", "ticket_wave_lock")] {
        let source =
            fs::read_to_string(root.join(format!("tools/xtask/src/commands/{adapter}/mod.rs")))
                .unwrap();
        assert!(
            source.contains(&format!("{owner}::")),
            "the {adapter} adapter must delegate to {owner}"
        );
    }
}

/// The root folder of every tool crate.
const TOOLS_ROOT: &str = "tools";

/// The tool binaries that sit directly under [`TOOLS_ROOT`]; the folder walk must find both.
const TOOL_BINARIES: [&str; 2] = ["xtask", "developer_tools"];

/// The child folders of `parent`, sorted.
fn sorted_child_folders(parent: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = fs::read_dir(parent)
        .unwrap_or_else(|error| panic!("cannot list {}: {error}", parent.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    folders
}

/// The folder of every tool crate, found by folder: each `tools/<name>` holding a `Cargo.toml`
/// (the binaries of [`TOOL_BINARIES`]), then each `tools/<category>/<name>` holding one. No list
/// names a crate, so the structural rules below — line limits, no inline test modules, no
/// file-size exemptions — hold for every tool crate from the commit that creates it, with no
/// crate exempt from any of them.
fn tooling_crate_folders(root: &Path) -> Vec<PathBuf> {
    let mut binaries = Vec::new();
    let mut categorised = Vec::new();
    for top in sorted_child_folders(&root.join(TOOLS_ROOT)) {
        if top.join("Cargo.toml").is_file() {
            binaries.push(top);
            continue;
        }
        categorised.extend(
            sorted_child_folders(&top)
                .into_iter()
                .filter(|folder| folder.join("Cargo.toml").is_file()),
        );
    }
    for binary in TOOL_BINARIES {
        assert!(
            binaries.iter().any(|folder| folder.ends_with(binary)),
            "the folder walk found no {TOOLS_ROOT}/{binary}: {binaries:?}"
        );
    }
    for category in [TOOL_FOUNDATION, TOOL_TICKETS] {
        assert!(
            categorised
                .iter()
                .any(|folder| folder.starts_with(root.join(category))),
            "the folder walk found no crate under {category}"
        );
    }
    binaries.extend(categorised);
    binaries
}

/// The walk finds the binaries, every categorised crate this crate itself sits beside, and no
/// folder that holds no manifest.
#[test]
fn tooling_crate_folders_are_found_by_folder() {
    let root = tool_test_support::test_repo_root();
    let folders = tooling_crate_folders(&root);
    let relative: Vec<String> = folders
        .iter()
        .map(|folder| {
            folder
                .strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    for expected in [
        "tools/xtask",
        "tools/developer_tools",
        "tools/foundation/verification_core",
        "tools/tickets/ticket_model",
        "tools/checks/repository_checks",
        "tools/checks/mod_script_checks",
    ] {
        assert!(
            relative.iter().any(|path| path == expected),
            "missing {expected}: {relative:?}"
        );
    }
    assert!(
        !relative
            .iter()
            .any(|path| path == "tools/enfusion_mcp_node_package"),
        "a folder without a Cargo.toml is not a crate"
    );
    assert!(
        folders
            .iter()
            .all(|folder| folder.join("Cargo.toml").is_file())
    );
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
    if path.ends_with("browser_gate_suites/src/editor_smoke_tests.rs") {
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
    } else if path.starts_with("tools/browser_testing/browser_gate_suites/src/editor_smoke_tests")
        || path.ends_with("browser_gate_suites/src/editor_smoke_tests.rs")
    {
        450
    } else {
        500
    }
}

#[test]
fn tooling_source_files_stay_below_their_structural_limits() {
    let root = tool_test_support::test_repo_root();
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
    let root = tool_test_support::test_repo_root();
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
    let root = tool_test_support::test_repo_root();
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
            "tools/browser_testing/browser_gate_suites/src/editor_smoke_tests.rs",
            450,
        ),
        (
            "tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/scenario.rs",
            450,
        ),
        (
            "tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/tests/scenario.rs",
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
