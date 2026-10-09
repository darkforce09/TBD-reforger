use super::*;
use serde_json::json;

fn fixture_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("terrain-manifest-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).expect("fixture root");
    d
}

/// The §5 manifest, complete. `parse_manifest_binary` reads only these keys.
fn full_manifest() -> Value {
    json!({
        "objects": { "binary": {
            "schemaVersion": "1.0.0", "container": "TBDC", "containerVersion": 1,
            "pod": "ObjectInstancePod", "podBytes": 32,
            "chunks": "objects/chunks/{cx}_{cy}.bin", "prefabs": "objects/prefabs.rkyv",
            "roads": "roads/road_network.rkyv", "regions": "objects/forest-regions.rkyv",
            "typeInventory": "objects/type-inventory.rkyv" } },
        "dem": { "raw": { "path": "dem/elevation.dem", "encoding": "tbde-v1" } },
        "labels": { "path": "locations/map_labels.rkyv", "encoding": "rkyv-map-labels-v1" },
        "water": { "vectors": "water/water_vectors.rkyv",
                   "bathymetry": "water/bathymetry.tbd-bath", "encoding": "tbdb-v1" },
        "buildings": { "archive": "prefabs/building_blueprints.rkyv", "blas": "prefabs/blas" }
    })
}

/// Every file the §5 manifest names, materialised. Content is irrelevant — the gate checks
/// existence only, because in a slice worktree these are git-LFS pointer files.
fn materialise(dir: &Path) {
    for rel in [
        "objects/chunks/0_0.bin",
        "objects/prefabs.rkyv",
        "roads/road_network.rkyv",
        "objects/forest-regions.rkyv",
        "objects/type-inventory.rkyv",
        "dem/elevation.dem",
        "locations/map_labels.rkyv",
        "water/water_vectors.rkyv",
        "water/bathymetry.tbd-bath",
        "prefabs/building_blueprints.rkyv",
    ] {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        fs::write(&p, b"x").expect("write");
    }
    fs::create_dir_all(dir.join("prefabs/blas")).expect("blas dir");
}

fn live_instance_schema() -> Value {
    let root = ::repository_root::find_repository_root().expect("repository root");
    read_json_file(&definition_path(&root, "map-object-instance.schema.json"))
        .expect("map-object-instance.schema.json")
}

/// The committed row doc describes exactly the POD this build links.
#[test]
fn live_pod_row_doc_matches_the_rust_pod() {
    assert_eq!(
        pod_row_doc_failures(&live_instance_schema()),
        Vec::<String>::new()
    );
}

/// ACCEPTANCE: a manifest WITH every §5 block, every path present.
#[test]
fn every_binary_block_is_accepted_when_its_paths_exist() {
    let dir = fixture_dir("full");
    materialise(&dir);
    let (declared, errs) = manifest_binary_failures(&full_manifest(), &dir);
    assert_eq!(declared, 5, "five blocks declared");
    assert_eq!(errs, Vec::<String>::new());
}

/// ACCEPTANCE: the same manifest over an EMPTY tree. Every path dangles, and a dangling binary
/// path is silent at runtime (the loader falls back to JSON), so it has to be loud here.
#[test]
fn dangling_binary_paths_are_rejected_one_by_one() {
    let dir = fixture_dir("empty");
    let (declared, errs) = manifest_binary_failures(&full_manifest(), &dir);
    assert_eq!(declared, 5);
    for kind in [
        "objects.binary.chunks",
        "objects.binary.prefabs",
        "objects.binary.roads",
        "objects.binary.regions",
        "objects.binary.typeInventory",
        "dem.raw.path",
        "labels.path",
        "water.vectors",
        "water.bathymetry",
        "buildings.archive",
        "buildings.blas",
    ] {
        assert!(
            errs.iter().any(|e| e.starts_with(kind)),
            "{kind} not reported: {errs:?}"
        );
    }
}

/// A chunks dir that exists but was never emitted into is still dangling — the directory alone
/// proves nothing, and `objects/chunks/` is populated with `.json.gz` on every shipped terrain.
#[test]
fn a_chunks_dir_holding_no_bin_is_dangling() {
    let dir = fixture_dir("gzonly");
    materialise(&dir);
    fs::remove_file(dir.join("objects/chunks/0_0.bin")).expect("rm");
    fs::write(dir.join("objects/chunks/0_0.json.gz"), b"x").expect("write");
    let (_, errs) = manifest_binary_failures(&full_manifest(), &dir);
    assert!(errs.iter().any(|e| e.contains("holds no .bin")), "{errs:?}");
}

/// The shape check, which is the reason `pod`/`podBytes`/`containerVersion` are on the wire:
/// a 24-byte row read at a 32-byte stride does not error, it draws garbage.
#[test]
fn a_row_shape_this_build_cannot_read_is_refused() {
    let dir = fixture_dir("shape");
    materialise(&dir);
    let mut m = full_manifest();
    m["objects"]["binary"]["podBytes"] = json!(24);
    let (_, errs) = manifest_binary_failures(&m, &dir);
    assert!(
        errs.iter().any(|e| e.contains("this build reads")),
        "{errs:?}"
    );

    // …and a template missing a placeholder, which would resolve the whole world to one URL.
    let mut t = full_manifest();
    t["objects"]["binary"]["chunks"] = json!("objects/chunks/all.bin");
    let (_, errs) = manifest_binary_failures(&t, &dir);
    assert!(errs.iter().any(|e| e.contains("{cx}")), "{errs:?}");
}
