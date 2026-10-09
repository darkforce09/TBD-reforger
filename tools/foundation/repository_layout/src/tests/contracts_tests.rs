use super::*;
use repository_root::find_repository_root;

/// Every contract location this module declares exists in a real checkout.
///
/// A path helper that silently resolves to nothing is worse than a literal: callers read it as a
/// guarantee, and a directory walk over a missing root reports "no findings" rather than failing.
/// This pins the whole surface against the live tree so a relocation cannot half-land.
#[test]
fn every_contract_location_exists_in_the_checkout() {
    let root = find_repository_root().expect("active checkout");
    for dir in [
        contracts_dir(&root),
        contract_definitions_dir(&root),
        contract_rules_dir(&root),
        contract_catalogs_dir(&root),
        contract_fixtures_dir(&root),
        mission_fixtures_valid_dir(&root),
        mission_fixtures_invalid_dir(&root),
        map_fixtures_dir(&root),
        registry_fixtures_dir(&root),
        enfusion_sample_fixtures_dir(&root),
        bridge_sample_fixtures_dir(&root),
    ] {
        assert!(dir.is_dir(), "not a directory: {}", dir.display());
    }
    for file in [
        definition_path(&root, "mission.schema.json"),
        prefab_classify_path(&root),
        kit_aliases_path(&root),
        registry_items_catalog_path(&root),
        registry_compat_catalog_path(&root),
    ] {
        assert!(file.is_file(), "not a file: {}", file.display());
    }
}

/// Locations are resolved against the caller's root, never an ambient one.
#[test]
fn contract_locations_resolve_against_the_given_root() {
    let root = Path::new("/tmp/checkout");
    for path in [
        contracts_dir(root),
        contract_definitions_dir(root),
        contract_fixtures_dir(root),
    ] {
        assert!(
            path.starts_with(root.join(CONTRACTS_DIR)),
            "escaped: {}",
            path.display()
        );
    }
}
