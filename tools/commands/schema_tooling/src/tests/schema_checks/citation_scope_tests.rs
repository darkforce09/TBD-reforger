use super::*;
use serde_json::json;

fn fixture_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("t611-citations-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("fixture root");
    dir
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, body).expect("write");
}

/// A schema dir with one real schema carrying one real `$defs`.
fn schema_dir(root: &Path) -> PathBuf {
    let dir = root.join("schema");
    fs::create_dir_all(&dir).expect("schema dir");
    fs::write(
        dir.join("good.schema.json"),
        serde_json::to_string(&json!({"$defs": {"item": {"type": "object"}}})).unwrap(),
    )
    .expect("write schema");
    dir
}

/// The red proof, as a test: a dangling `@contract` in `apps/**/*.rs` and a bad pointer in
/// `tools/**/*.rs` are both caught; the existing tools tree is scanned too.
#[test]
fn rust_under_apps_and_tooling_is_scanned_and_can_fail() {
    let root = fixture_dir("apps-tools-rs");
    let schemas = schema_dir(&root);
    workspace(
        &root,
        &[
            "crates/api/api_server",
            "tools/developer_tools",
            "tools/tickets/ticket_model",
        ],
    );
    write(
        &root,
        "crates/api/api_server/src/handlers/x.rs",
        concat!("//! @contract", " nope.schema.json#/\n"),
    );
    write(
        &root,
        "tools/tickets/ticket_model/src/types.rs",
        concat!("// @contract", " good.schema.json#/$defs/absent\n"),
    );
    write(
        &root,
        "apps/mod/tbd-framework/Scripts/z.c",
        concat!("//! @contract", " good.schema.json#/\n"),
    );
    write(
        &root,
        "crates/frontend/shell/frontend_application/w.ts",
        concat!(" * @contract", " good.schema.json#/$defs/item\n"),
    );

    write(
        &root,
        "tools/developer_tools/src/lib.rs",
        concat!("// @contract", " good.schema.json#/\n"),
    );

    let scan = scan_citations(&root, &schemas).expect("scan");
    assert_eq!(scan.citations, 5, "one tag per fixture file");
    assert_eq!(scan.per_ext["rs"], 3, "rs must be scanned (T-611)");
    assert_eq!(scan.per_ext["c"], 1);
    assert_eq!(scan.per_ext["ts"], 1);
    assert_eq!(scan.scope_errors, Vec::<String>::new());
    assert_eq!(scan.problems.len(), 2, "got: {:?}", scan.problems);
    assert!(
        scan.problems
            .iter()
            .any(|p| p.contains("crates/api/api_server/src/handlers/x.rs")
                && p.contains("not found")),
        "missing-schema in apps/**/*.rs must fail: {:?}",
        scan.problems
    );
    assert!(
        scan.problems
            .iter()
            .any(|p| p.contains("tools/tickets/ticket_model/src/types.rs") && p.contains("pointer")),
        "bad pointer in tools/**/*.rs must fail: {:?}",
        scan.problems
    );
    let _ = fs::remove_dir_all(&root);
}

/// A scan root that is not there is not "nothing to report" — it is no verdict.
#[test]
fn missing_scan_root_is_a_scope_failure_not_a_pass() {
    let root = fixture_dir("missing-root");
    let schemas = schema_dir(&root);
    workspace(&root, &["crates/api/api_server"]);
    write(
        &root,
        "crates/api/api_server/src/a.rs",
        concat!("//! @contract", " good.schema.json#/\n"),
    );
    // `apps/` itself exists (it holds the mod in the real tree), so the one missing root is
    // `apps/mod/`.
    fs::create_dir_all(root.join("apps")).expect("apps folder");

    let scan = scan_citations(&root, &schemas).expect("scan");
    assert!(scan.problems.is_empty(), "the one citation resolves");
    assert_eq!(scan.scope_errors.len(), 1, "apps/mod/ absent");
    assert!(
        scan.scope_errors
            .iter()
            .any(|e| e.starts_with("scan root apps/mod/"))
    );
    let _ = fs::remove_dir_all(&root);
}

/// Zero citations means the matcher or the config broke, not that everything resolves.
#[test]
fn empty_corpus_is_a_scope_failure_not_a_pass() {
    let root = fixture_dir("empty-corpus");
    let schemas = schema_dir(&root);
    workspace(&root, &["crates/api/api_server", "tools/xtask"]);
    for code_root in NON_WORKSPACE_CODE_ROOTS {
        fs::create_dir_all(root.join(code_root)).expect("root");
    }
    write(&root, "crates/api/api_server/src/a.rs", "// no tags here\n");

    let scan = scan_citations(&root, &schemas).expect("scan");
    assert_eq!(scan.citations, 0);
    assert_eq!(scan.scope_errors.len(), 1, "got: {:?}", scan.scope_errors);
    assert!(scan.scope_errors[0].contains("0 @contract citation(s)"));
    let _ = fs::remove_dir_all(&root);
}

/// The printed scope sentence is generated from the extensions and the roots the scan walks, so
/// it cannot drift from the walker; over the live checkout those roots hold the top-level folder
/// of every workspace member, `apps/`, `crates/` (the frontend crates' `@contract` tags among
/// them) and `tools/` among them.
#[test]
fn scope_line_is_generated_from_the_walked_roots() {
    let root = tool_test_support::test_repo_root();
    let roots = scan_roots(&root).expect("the live workspace reads");
    let scope = citation_scope(&roots);
    for e in CODE_EXTS {
        assert!(scope.contains(&format!(".{e}")), "{scope} omits .{e}");
    }
    for r in &roots {
        assert!(scope.contains(&format!("{r}/")), "{scope} omits {r}/");
    }
    let members = read_workspace_members(&root).expect("the live workspace reads");
    for member in &members {
        let top_level_folder = member.path.split('/').next().unwrap_or_default();
        assert!(
            roots.iter().any(|r| r == top_level_folder),
            "member {} lies outside the scan roots {roots:?}",
            member.path
        );
    }
    assert!(CODE_EXTS.contains(&"rs"), "T-611: rs must stay scanned");
    for r in ["apps", "crates", "tools"] {
        assert!(roots.iter().any(|root| root == r), "{r}/ must stay scanned");
    }
}

/// A root `Cargo.toml` whose workspace names `members`, and a package manifest in each member.
fn workspace(root: &Path, members: &[&str]) {
    let list: Vec<String> = members.iter().map(|m| format!("\"{m}\"")).collect();
    write(
        root,
        "Cargo.toml",
        &format!("[workspace]\nmembers = [{}]\n", list.join(", ")),
    );
    for member in members {
        let name = member.rsplit('/').next().expect("a folder name");
        write(
            root,
            &format!("{member}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\n"),
        );
    }
}

/// Every workspace member's source is scanned, whatever top-level folder holds it: a dangling
/// citation in a member under `crates/` or `engines/` fails the scan as one under `apps/` does.
#[test]
fn every_workspace_members_top_level_folder_is_scanned_and_can_fail() {
    let root = fixture_dir("workspace-members");
    let schemas = schema_dir(&root);
    workspace(
        &root,
        &[
            "crates/api/api_server",
            "crates/foundation/guard",
            "engines/renderer",
            "tools/xtask",
        ],
    );
    write(
        &root,
        "apps/mod/tbd-framework/Scripts/z.c",
        concat!("//! @contract", " good.schema.json#/\n"),
    );
    write(
        &root,
        "crates/foundation/guard/src/lib.rs",
        concat!("//! @contract", " nope.schema.json#/\n"),
    );
    write(
        &root,
        "engines/renderer/src/lib.rs",
        concat!("// @contract", " good.schema.json#/$defs/absent\n"),
    );

    let scan = scan_citations(&root, &schemas).expect("scan");
    assert_eq!(scan.scope_errors, Vec::<String>::new());
    assert_eq!(scan.citations, 3, "one tag per fixture file");
    assert_eq!(scan.problems.len(), 2, "got: {:?}", scan.problems);
    assert!(
        scan.problems
            .iter()
            .any(|p| p.contains("crates/foundation/guard/src/lib.rs") && p.contains("not found")),
        "a missing schema cited under crates/ must fail: {:?}",
        scan.problems
    );
    assert!(
        scan.problems
            .iter()
            .any(|p| p.contains("engines/renderer/src/lib.rs") && p.contains("pointer")),
        "a bad pointer cited under engines/ must fail: {:?}",
        scan.problems
    );
    let _ = fs::remove_dir_all(&root);
}

/// A checkout whose workspace cannot be read gives the scan no roots to derive: that is no
/// verdict, never a pass over the folders it happened to find.
#[test]
fn an_unreadable_workspace_is_a_scope_failure_not_a_pass() {
    let root = fixture_dir("unreadable-workspace");
    let schemas = schema_dir(&root);
    write(
        &root,
        "apps/mod/z.c",
        concat!("//! @contract", " good.schema.json#/\n"),
    );
    write(
        &root,
        "tools/xtask/src/main.rs",
        concat!("// @contract", " good.schema.json#/\n"),
    );

    let scan = scan_citations(&root, &schemas).expect("scan");
    assert!(
        scan.scope_errors
            .iter()
            .any(|e| e.contains("Cargo.toml") && e.contains("workspace")),
        "got: {:?}",
        scan.scope_errors
    );
    let _ = fs::remove_dir_all(&root);
}
