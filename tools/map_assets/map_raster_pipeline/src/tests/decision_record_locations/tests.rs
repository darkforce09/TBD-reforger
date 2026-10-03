use super::*;
use ::repository_layout::ARTIFACTS_DIR;

/// The decision records are records of a run, so they lie inside the agent
/// artifact tree and resolve against the caller's root, never an ambient one.
#[test]
fn run_records_lie_inside_the_agent_artifact_tree_of_the_given_root() {
    let root = Path::new("/tmp/checkout");
    let artifacts = root.join(ARTIFACTS_DIR);
    for path in [
        inland_water_artifacts_dir(root),
        aerial_orthophoto_artifacts_dir(root),
        root.join(CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR),
    ] {
        assert!(
            path.starts_with(&artifacts),
            "outside the artifact tree: {}",
            path.display()
        );
    }
}
