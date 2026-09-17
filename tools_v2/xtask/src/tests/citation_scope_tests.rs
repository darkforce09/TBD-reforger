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
/// `tools_v2/**/*.rs` are both caught; the existing tools tree is scanned too.
#[test]
fn rust_under_apps_and_tooling_is_scanned_and_can_fail() {
    let root = fixture_dir("apps-tools-rs");
    let schemas = schema_dir(&root);
    write(
        &root,
        "apps/website/api/src/handlers/x.rs",
        concat!("//! @contract", " nope.schema.json#/\n"),
    );
    write(
        &root,
        "tools_v2/ticket-engine/src/types.rs",
        concat!("// @contract", " good.schema.json#/$defs/absent\n"),
    );
    write(
        &root,
        "apps/mod/tbd-framework/Scripts/z.c",
        concat!("//! @contract", " good.schema.json#/\n"),
    );
    write(
        &root,
        "packages/tbd-schema/w.ts",
        concat!(" * @contract", " good.schema.json#/$defs/item\n"),
    );

    write(
        &root,
        "tools_v2/developer-tools/src/lib.rs",
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
            .any(|p| p.contains("apps/website/api/src/handlers/x.rs") && p.contains("not found")),
        "missing-schema in apps/**/*.rs must fail: {:?}",
        scan.problems
    );
    assert!(
        scan.problems
            .iter()
            .any(|p| p.contains("tools_v2/ticket-engine/src/types.rs") && p.contains("pointer")),
        "bad pointer in tools_v2/**/*.rs must fail: {:?}",
        scan.problems
    );
    let _ = fs::remove_dir_all(&root);
}

/// A scan root that is not there is not "nothing to report" — it is no verdict.
#[test]
fn missing_scan_root_is_a_scope_failure_not_a_pass() {
    let root = fixture_dir("missing-root");
    let schemas = schema_dir(&root);
    write(
        &root,
        "apps/a.rs",
        concat!("//! @contract", " good.schema.json#/\n"),
    );

    let scan = scan_citations(&root, &schemas).expect("scan");
    assert!(scan.problems.is_empty(), "the one citation resolves");
    assert_eq!(scan.scope_errors.len(), 2, "tools_v2/ and packages/ absent");
    assert!(
        scan.scope_errors
            .iter()
            .any(|e| e.starts_with("scan root tools_v2/"))
    );
    assert!(
        scan.scope_errors
            .iter()
            .any(|e| e.starts_with("scan root packages/"))
    );
    let _ = fs::remove_dir_all(&root);
}

/// Zero citations means the matcher or the config broke, not that everything resolves.
#[test]
fn empty_corpus_is_a_scope_failure_not_a_pass() {
    let root = fixture_dir("empty-corpus");
    let schemas = schema_dir(&root);
    for r in SCAN_ROOTS {
        fs::create_dir_all(root.join(r)).expect("root");
    }
    write(&root, "apps/a.rs", "// no tags here\n");

    let scan = scan_citations(&root, &schemas).expect("scan");
    assert_eq!(scan.citations, 0);
    assert_eq!(scan.scope_errors.len(), 1, "got: {:?}", scan.scope_errors);
    assert!(scan.scope_errors[0].contains("0 @contract citation(s)"));
    let _ = fs::remove_dir_all(&root);
}

/// The printed scope sentence is generated, so it cannot drift from the walker's config.
#[test]
fn scope_line_is_generated_from_the_constants() {
    let scope = citation_scope();
    for e in CODE_EXTS {
        assert!(scope.contains(&format!(".{e}")), "{scope} omits .{e}");
    }
    for r in SCAN_ROOTS {
        assert!(scope.contains(&format!("{r}/")), "{scope} omits {r}/");
    }
    assert!(CODE_EXTS.contains(&"rs"), "T-611: rs must stay scanned");
    assert!(
        SCAN_ROOTS.contains(&"tools_v2"),
        "tools_v2/ must stay scanned"
    );
}
