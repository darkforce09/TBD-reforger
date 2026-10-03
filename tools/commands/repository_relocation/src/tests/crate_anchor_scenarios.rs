//! Whole relocation runs on throwaway checkouts for literals built on `CARGO_MANIFEST_DIR` in files
//! that move into another crate folder. The owning crate after the moves is the nearest folder of
//! the planned tree holding a crate manifest: one the same manifest moves there, or an untracked
//! one already on disk. A file whose new place has no crate manifest below the repository root
//! (the root manifest is the workspace's) leaves its literals unresolved, so the dry run fails and
//! the apply refuses with nothing written; such a literal is never re-anchored at the root.

use std::path::Path;

use super::fixture_repository::FixtureRepository;
use super::manifest::parse_manifest;
use super::relocation_plan::build_plan;
use super::repository_files::RepositorySnapshot;
use super::{apply, dry_run, verify};

/// The loader before the moves: line 3 joins onto `CARGO_MANIFEST_DIR`, line 5 concatenates.
const LOADER: &str = "use std::path::Path;\n\
                      pub fn shapes() -> std::path::PathBuf {\n    \
                      Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"../../assets/shapes.json\")\n\
                      }\n\
                      pub const SHAPES: &str =\n    \
                      include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../assets/shapes.json\"));\n";

/// Where the loader lies before the moves.
const LOADER_BEFORE: &str = "legacy/engine/src/geometry/loader.rs";

/// Where the loader lies after the moves.
const LOADER_AFTER: &str = "crates/geometry/shapes/src/loader.rs";

/// The row that moves the geometry module out of the engine crate into a new crate folder.
const GEOMETRY_ROW: &str = "path\tlegacy/engine/src/geometry\tcrates/geometry/shapes/src\t\n";

/// An engine crate whose geometry module reads a shared asset from its crate folder, under a root
/// workspace manifest.
fn engine_checkout(tag: &str) -> FixtureRepository {
    let repo = FixtureRepository::new(tag);
    repo.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"legacy/*\", \"crates/*/*\"]\n",
    )
    .write("legacy/engine/Cargo.toml", "[package]\nname = \"engine\"\n")
    .write("legacy/engine/src/lib.rs", "\n")
    .write("legacy/engine/src/geometry/mod.rs", "pub mod loader;\n")
    .write(LOADER_BEFORE, LOADER)
    .write("assets/shapes.json", "{}\n");
    repo
}

/// The loader as it reads from the new crate folder, one level deeper than the engine's.
fn loader_in_the_new_crate() -> String {
    LOADER.replace("../../assets/shapes.json", "../../../assets/shapes.json")
}

#[test]
fn relocate_manifest_dir_joins_follow_a_crate_manifest_the_same_manifest_moves() {
    let repo = engine_checkout("manifest-dir-moved-crate-manifest");
    repo.write(
        "drafts/geometry_shapes/Cargo.toml",
        "[package]\nname = \"geometry_shapes\"\n",
    )
    .track();
    let manifest = repo.manifest(&format!(
        "{GEOMETRY_ROW}path\tdrafts/geometry_shapes/Cargo.toml\tcrates/geometry/shapes/Cargo.toml\t\n"
    ));

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(repo.read(LOADER_AFTER), loader_in_the_new_crate());
    assert!(repo.exists("crates/geometry/shapes/Cargo.toml"));
    assert_eq!(verify(repo.root(), Some(Path::new(&manifest))), 0);
}

#[test]
fn relocate_manifest_dir_joins_follow_an_untracked_crate_manifest_at_the_destination() {
    let repo = engine_checkout("manifest-dir-untracked-crate-manifest");
    repo.track();
    // The crate being born: its manifest is on disk, untracked, before the moves fill the folder.
    repo.write(
        "crates/geometry/shapes/Cargo.toml",
        "[package]\nname = \"geometry_shapes\"\n",
    );
    let manifest = repo.manifest(GEOMETRY_ROW);

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    assert_eq!(repo.read(LOADER_AFTER), loader_in_the_new_crate());
    assert_eq!(verify(repo.root(), Some(Path::new(&manifest))), 0);
}

#[test]
fn relocate_manifest_dir_joins_into_a_folder_with_no_crate_manifest_are_unresolved() {
    let repo = engine_checkout("manifest-dir-no-destination-crate");
    repo.track();
    let manifest = repo.manifest(GEOMETRY_ROW);

    let text = std::fs::read_to_string(&manifest).expect("read the manifest");
    let rows = parse_manifest(&text).expect("a valid manifest");
    let snapshot = RepositorySnapshot::load(repo.root()).expect("list the checkout");
    let plan = build_plan(&snapshot, &rows, None).expect("the plan builds");
    let reported: Vec<String> = plan
        .unresolved
        .iter()
        .map(|item| format!("{}:{}", item.path, item.line))
        .collect();
    assert_eq!(
        reported,
        [3, 6].map(|line| format!("{LOADER_BEFORE}:{line}")),
        "{:?}",
        plan.unresolved
    );
    assert!(
        plan.unresolved.iter().all(|item| item
            .message
            .contains("no crate manifest lies at or above `crates/geometry/shapes/src`")),
        "{:?}",
        plan.unresolved
    );

    assert_eq!(dry_run(repo.root(), &manifest), 1);
    assert_eq!(apply(repo.root(), &manifest), 1);
    assert_eq!(repo.read(LOADER_BEFORE), LOADER, "nothing is rewritten");
    assert!(!repo.exists("crates/geometry"), "nothing moved");
}
