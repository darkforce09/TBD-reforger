use super::*;
use crate::browser_testing::server::repo_root;

fn committed_catalog() -> BallisticsCatalog {
    read_catalog(&repo_root().join(COMMITTED_CATALOG)).unwrap()
}

#[test]
fn mortar_offline_catalog_reads_come_from_the_committed_catalog_identity() {
    let catalog = committed_catalog();
    let reads = catalog_reads(&catalog).unwrap();
    assert_eq!(reads.list_file, "GET__ballistics-catalogs.json");
    assert_eq!(
        reads.version_file,
        format!(
            "GET__ballistics-catalogs__{}__versions__{}.json",
            catalog.catalog_id, catalog.catalog_version
        )
    );
}

#[test]
fn mortar_offline_required_files_name_the_pack_sources_and_both_reads() {
    let reads = catalog_reads(&committed_catalog()).unwrap();
    let root = Path::new("/repo");
    let files = required_files(root, Path::new("/dist"), &root.join(API_CORPUS_DIR), &reads);
    let paths: Vec<String> = files.iter().map(|(p, _)| p.display().to_string()).collect();
    for expected in [
        "/dist/index.html".to_string(),
        "/dist/service_worker.js".to_string(),
        "/repo/assets_v2/terrains/everon/manifest.json".to_string(),
        "/repo/assets_v2/terrains/everon/dem/everon-dem-16bit.png".to_string(),
        "/repo/assets_v2/terrains/everon/satellite/everon-sat.tbd-sat".to_string(),
        "/repo/assets_v2/terrains/everon/tiles/map/index.json".to_string(),
        format!("/repo/{API_CORPUS_DIR}/{}", reads.list_file),
        format!("/repo/{API_CORPUS_DIR}/{}", reads.version_file),
    ] {
        assert!(paths.contains(&expected), "{expected} not in {paths:?}");
    }
}

#[test]
fn mortar_offline_require_files_names_every_missing_file() {
    let present = repo_root().join(COMMITTED_CATALOG);
    let files = vec![
        (present.clone(), "the catalog".to_string()),
        (
            PathBuf::from("/no/such/tile-index.json"),
            "the index".to_string(),
        ),
        (
            PathBuf::from("/no/such/golden.json"),
            "the golden".to_string(),
        ),
    ];
    let error = require_files(&files).unwrap_err().to_string();
    assert!(error.starts_with("assets missing"), "{error}");
    assert!(
        error.contains("/no/such/tile-index.json (the index)"),
        "{error}"
    );
    assert!(
        error.contains("/no/such/golden.json (the golden)"),
        "{error}"
    );
    assert!(!error.contains("the catalog"), "{error}");
    assert!(require_files(&files[..1]).is_ok());
}

#[test]
fn mortar_offline_high_explosive_shell_is_an_unfuzed_he_shell_of_the_weapon() {
    let catalog = committed_catalog();
    for weapon in &catalog.weapons {
        let shell_id = high_explosive_shell(&catalog, &weapon.weapon_id).unwrap();
        assert!(weapon.shell_ids.contains(&shell_id));
        let shell = catalog
            .shells
            .iter()
            .find(|s| s.shell_id == shell_id)
            .unwrap();
        assert_eq!(shell.role, ShellRole::He);
        assert!(shell.time_fuze.is_none());
    }
    assert!(high_explosive_shell(&catalog, "no-such-weapon").is_err());
}

#[test]
fn mortar_offline_gun_stands_south_of_the_target_unless_that_leaves_the_map() {
    assert_eq!(
        gun_position(6_400.0, 5_000.0),
        (6_400.0, 5_000.0 - GUN_OFFSET_M)
    );
    assert_eq!(
        gun_position(6_400.0, 800.0),
        (6_400.0, 800.0 + GUN_OFFSET_M)
    );
}
