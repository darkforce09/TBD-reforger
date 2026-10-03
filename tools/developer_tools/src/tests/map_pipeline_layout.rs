use super::*;
use ::repository_layout::{ARTIFACTS_DIR, find_repository_root};

/// The committed density fixtures exist in a real checkout, inside the map fixture folder.
///
/// A path helper that silently resolves to nothing is worse than a literal: re-densifying reads a
/// missing folder as "no committed fixtures" rather than failing.
#[test]
fn the_density_fixtures_exist_in_the_checkout() {
    let root = find_repository_root().expect("active checkout");
    let density = density_fixtures_dir(&root);
    assert!(density.is_dir(), "not a directory: {}", density.display());
    assert!(density.starts_with(::repository_layout::map_fixtures_dir(&root)));
}

/// The operation logs and decision records are records of a run, so they lie inside the agent
/// artifact tree and resolve against the caller's root, never an ambient one.
#[test]
fn run_records_lie_inside_the_agent_artifact_tree_of_the_given_root() {
    let root = Path::new("/tmp/checkout");
    let artifacts = root.join(ARTIFACTS_DIR);
    for path in [
        export_operations_log(root, "everon"),
        object_type_inventory(root, "everon"),
        inland_water_artifacts_dir(root),
        aerial_orthophoto_artifacts_dir(root),
        cartographic_rendering_artifacts_dir(root),
    ] {
        assert!(
            path.starts_with(&artifacts),
            "outside the artifact tree: {}",
            path.display()
        );
    }
}
