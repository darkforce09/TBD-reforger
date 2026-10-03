use super::*;
use crate::find_repository_root;

/// Every committed map-asset location this module declares exists in a real checkout.
#[test]
fn every_map_asset_location_exists_in_the_checkout() {
    let root = find_repository_root().expect("active checkout");
    for dir in [
        terrain_assets_dir(&root),
        terrain_dir(&root, "everon"),
        glyph_assets_dir(&root),
    ] {
        assert!(dir.is_dir(), "not a directory: {}", dir.display());
    }
    for file in [
        terrain_registry_path(&root),
        terrain_manifest_path(&root, "everon"),
        glyph_manifest_path(&root),
    ] {
        assert!(file.is_file(), "not a file: {}", file.display());
    }
}

/// Export scratch is deliberately absent from a fresh clone, so it is pinned by shape rather than
/// existence: it is named for its island, and it sits OUTSIDE the terrain tree.
///
/// That separation is the invariant worth a test. The terrain tree is served wholesale at
/// `/map-assets`, so scratch nested inside it would publish gigabytes of uncommitted export
/// intermediates to every map client.
#[test]
fn export_scratch_is_named_for_its_island_and_sits_outside_the_served_tree() {
    let root = find_repository_root().expect("active checkout");
    let scratch = map_scratch_dir(&root, "everon");
    assert!(scratch.ends_with("everon"), "{}", scratch.display());
    assert!(
        !scratch.starts_with(terrain_assets_dir(&root)),
        "{}",
        scratch.display()
    );
}

/// Locations are resolved against the caller's root, never an ambient one.
#[test]
fn map_asset_locations_resolve_against_the_given_root() {
    let root = Path::new("/tmp/checkout");
    for path in [
        terrain_assets_dir(root),
        glyph_assets_dir(root),
        map_scratch_dir(root, "everon"),
    ] {
        assert!(
            path.starts_with(root),
            "escaped the root: {}",
            path.display()
        );
    }
}
