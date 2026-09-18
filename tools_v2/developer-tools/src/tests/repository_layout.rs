use super::*;
use crate::repository_paths::find_repo_root;

/// Every declared location must exist in a real checkout.
///
/// A path helper that silently resolves to nothing is worse than a literal: callers read it as a
/// guarantee, and a directory walk over a missing root reports "no findings" rather than failing.
/// This pins the whole surface against the live tree so a relocation cannot half-land.
#[test]
fn every_declared_location_exists_in_the_checkout() {
    let root = find_repo_root().expect("active checkout");

    for dir in [
        contracts_dir(&root),
        contract_definitions_dir(&root),
        contract_rules_dir(&root),
        contract_catalogs_dir(&root),
        contract_fixtures_dir(&root),
        mission_fixtures_valid_dir(&root),
        mission_fixtures_invalid_dir(&root),
        map_fixtures_dir(&root),
        density_fixtures_dir(&root),
        registry_fixtures_dir(&root),
        enfusion_sample_fixtures_dir(&root),
        bridge_sample_fixtures_dir(&root),
        terrain_assets_dir(&root),
        terrain_dir(&root, "everon"),
        glyph_assets_dir(&root),
    ] {
        assert!(dir.is_dir(), "not a directory: {}", dir.display());
    }

    for file in [
        definition_path(&root, "mission.schema.json"),
        prefab_classify_path(&root),
        kit_aliases_path(&root),
        registry_items_catalog_path(&root),
        registry_compat_catalog_path(&root),
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
    let root = find_repo_root().expect("active checkout");
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
fn locations_resolve_against_the_given_root() {
    let root = Path::new("/tmp/checkout");
    for path in [
        contracts_dir(root),
        contract_definitions_dir(root),
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
