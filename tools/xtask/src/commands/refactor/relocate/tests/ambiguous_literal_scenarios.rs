//! Whole relocation runs on throwaway checkouts for relative literals the moves cannot pin to one
//! anchor: a literal that every crate spells for its own files (`src/lib.rs`, `src/`), a fixture
//! path relative to a temporary checkout (`../../engine/map`) and a comment's example path whose
//! tail names nothing stay as written in a file that moves out of its crate, and the dry run lists
//! each as ambiguous with its `path:line`, while a literal only its own crate can mean, and a path
//! its syntax anchors, still follow the moves.

use std::path::Path;

use super::fixture_repository::FixtureRepository;
use super::manifest::parse_manifest;
use super::relocation_plan::build_plan;
use super::repository_files::RepositorySnapshot;
use super::{apply, dry_run, verify};

/// The laws file before the moves; line 1 to 5 are generic, line 6 is crate-specific, line 7 is
/// anchored by `include_str!`.
const LAWS: &str = "//! A `#[path = \"../tests/cases_1.rs\"] mod tests;` line places a test file.\n\
                    pub const LIBRARY: &str = \"src/lib.rs\";\n\
                    pub fn crate_relative(rel: &str) -> &str { rel.strip_prefix(\"src/\").unwrap_or(rel) }\n\
                    pub const FIXTURE: &str = \"[dependencies]\\nmap = { path = \\\"../../engine/map\\\" }\\n\";\n\
                    pub const MAP_CRATE: &str = \"../../engine/map\";\n\
                    pub const OWN: &str = \"fixtures/only_core.json\";\n\
                    pub const DATA: &str = include_str!(\"../../../../engine/map/data.json\");\n";

/// The rows of a crate split: the crate's library stays one crate under `tools/foundation`, and its
/// laws folder leaves it for a crate folder of its own, one level shallower.
const SPLIT_ROWS: &str = "path\ttools/core/Cargo.toml\ttools/foundation/core/Cargo.toml\t\n\
                          path\ttools/core/src/lib.rs\ttools/foundation/core/src/lib.rs\t\n\
                          path\ttools/core/src/tests\ttools/foundation/core/src/tests\t\n\
                          path\ttools/core/src/laws\ttools/laws/src\t\n";

fn split_checkout() -> FixtureRepository {
    let repo = FixtureRepository::new("crate-generic-literals");
    repo.write("apps/web/Cargo.toml", "[package]\nname = \"web\"\n")
        .write("apps/web/src/lib.rs", "\n")
        .write("engine/map/Cargo.toml", "[package]\nname = \"map\"\n")
        .write("engine/map/src/lib.rs", "\n")
        .write("engine/map/data.json", "{}\n")
        .write("tools/core/Cargo.toml", "[package]\nname = \"core\"\n")
        .write("tools/core/src/lib.rs", "pub mod laws;\n")
        .write("tools/core/src/tests/lib_tests.rs", "\n")
        .write("tools/core/fixtures/only_core.json", "{}\n")
        .write("tools/core/src/laws/mod.rs", LAWS)
        .track();
    repo
}

#[test]
fn relocate_crate_generic_literals_in_a_file_leaving_its_crate_stay_as_written() {
    let repo = split_checkout();
    let manifest = repo.manifest(SPLIT_ROWS);

    let text = std::fs::read_to_string(&manifest).expect("read the manifest");
    let rows = parse_manifest(&text).expect("a valid manifest");
    let snapshot = RepositorySnapshot::load(repo.root()).expect("list the checkout");
    let plan = build_plan(&snapshot, &rows, None).expect("the plan builds");
    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    let expected = LAWS
        .replace(
            "\"fixtures/only_core.json\"",
            "\"tools/core/fixtures/only_core.json\"",
        )
        .replace(
            "include_str!(\"../../../../engine/map/data.json\")",
            "include_str!(\"../../../engine/map/data.json\")",
        );
    assert_eq!(repo.read("tools/laws/src/mod.rs"), expected);
    assert_eq!(verify(repo.root(), Some(Path::new(&manifest))), 0);

    let reported: Vec<String> = plan
        .ambiguous
        .iter()
        .map(|item| format!("{}:{}", item.path, item.line))
        .collect();
    let laws = "tools/core/src/laws/mod.rs";
    assert_eq!(
        reported,
        [1, 2, 3, 4, 5].map(|line| format!("{laws}:{line}")),
        "{:?}",
        plan.ambiguous
    );
    assert!(plan.unresolved.is_empty(), "{:?}", plan.unresolved);
}
