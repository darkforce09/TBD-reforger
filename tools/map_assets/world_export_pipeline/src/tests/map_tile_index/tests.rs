use super::*;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

static SCRATCH_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A fresh checkout-shaped scratch root with one terrain manifest.
fn scratch_root(terrain: &str, min_zoom: u32, max_zoom: u32) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tbd-tile-index-{}-{}",
        std::process::id(),
        SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    let terrain_root = terrain_dir(&root, terrain);
    fs::create_dir_all(&terrain_root).expect("terrain dir");
    let manifest = json!({
        "terrainId": terrain,
        "tiles": {
            "minZoom": min_zoom,
            "maxZoom": max_zoom,
            "map": {
                "path": "tiles/map",
                "urlTemplate": format!("/map-assets/{terrain}/tiles/map/{{z}}/{{x}}/{{y}}.webp"),
            },
        },
    });
    fs::write(terrain_root.join("manifest.json"), manifest.to_string()).expect("manifest");
    root
}

fn write_tile(root: &Path, terrain: &str, z: u32, x: u32, y: u32, bytes: usize) {
    let dir = terrain_dir(root, terrain).join(format!("tiles/map/{z}/{x}"));
    fs::create_dir_all(&dir).expect("tile dir");
    fs::write(dir.join(format!("{y}.webp")), vec![7u8; bytes]).expect("tile");
}

fn schema_validator() -> jsonschema::Validator {
    let path = repository_layout::find_repository_root_from(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .expect("repository root")
    .join("contracts/definitions/map-tile-index.schema.json");
    let schema: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("schema file")).expect("schema");
    jsonschema::validator_for(&schema).expect("validator")
}

#[test]
fn map_tile_index_parses_the_terrain_flag_and_refuses_everything_else() {
    let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    assert_eq!(
        parse_args(&args(&["--terrain", "everon"])),
        TileIndexArgs::Terrain("everon".into())
    );
    assert_eq!(parse_args(&args(&[])), TileIndexArgs::Usage);
    assert_eq!(parse_args(&args(&["--terrain"])), TileIndexArgs::Usage);
    assert_eq!(
        parse_args(&args(&["--terrain", "--x"])),
        TileIndexArgs::Usage
    );
    assert_eq!(
        parse_args(&args(&["everon"])),
        TileIndexArgs::Unknown("everon".into())
    );
}

#[test]
fn map_tile_index_lists_every_tile_sorted_and_skips_non_tiles() {
    let root = scratch_root("everon", 0, 1);
    write_tile(&root, "everon", 1, 1, 0, 30);
    write_tile(&root, "everon", 0, 0, 0, 10);
    write_tile(&root, "everon", 1, 0, 1, 20);
    let pyramid_dir = terrain_dir(&root, "everon").join("tiles/map");
    fs::write(pyramid_dir.join("full.webp"), b"preview").expect("preview");
    fs::write(pyramid_dir.join("1/0/notes.txt"), b"x").expect("stray");
    fs::write(pyramid_dir.join("1/0/0.png"), b"x").expect("other extension");

    let pyramid = read_pyramid(&terrain_dir(&root, "everon")).expect("pyramid");
    assert_eq!(pyramid.tile_extension, "webp");
    let index = build_index(&pyramid).expect("index");
    assert_eq!(index.schema_version, 1);
    assert_eq!(index.terrain_id, "everon");
    let listed: Vec<(u32, u32, u32, u64)> = index
        .tiles
        .iter()
        .map(|tile| (tile.z, tile.x, tile.y, tile.bytes))
        .collect();
    assert_eq!(listed, vec![(0, 0, 0, 10), (1, 0, 1, 20), (1, 1, 0, 30)]);
    assert!(missing_zoom_levels(&pyramid, &index).is_empty());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn map_tile_index_refuses_a_tile_outside_the_grid_or_the_zoom_range() {
    let root = scratch_root("everon", 0, 1);
    write_tile(&root, "everon", 1, 2, 0, 1);
    let pyramid = read_pyramid(&terrain_dir(&root, "everon")).expect("pyramid");
    assert_eq!(
        build_index(&pyramid),
        Err(TileIndexError::TileOutsidePyramid { z: 1, x: 2, y: 0 })
    );
    let _ = fs::remove_dir_all(&root);

    let root = scratch_root("everon", 0, 1);
    write_tile(&root, "everon", 2, 0, 0, 1);
    let pyramid = read_pyramid(&terrain_dir(&root, "everon")).expect("pyramid");
    let refusal = build_index(&pyramid).expect_err("zoom 2 is outside 0..=1");
    assert_eq!(
        refusal,
        TileIndexError::TileOutsidePyramid { z: 2, x: 0, y: 0 }
    );
    assert_eq!(refusal.exit_code(), 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn map_tile_index_refuses_a_missing_or_empty_pyramid_with_exit_2_and_writes_nothing() {
    let root = scratch_root("everon", 0, 1);
    let args = vec!["--terrain".to_owned(), "everon".to_owned()];
    assert_eq!(run_with_root(&root, &args).expect("run"), 2);
    let pyramid_dir = terrain_dir(&root, "everon").join("tiles/map");
    assert!(!pyramid_dir.join(TILE_INDEX_FILE_NAME).exists());

    fs::create_dir_all(pyramid_dir.join("0/0")).expect("empty pyramid");
    assert_eq!(run_with_root(&root, &args).expect("run"), 2);
    assert!(!pyramid_dir.join(TILE_INDEX_FILE_NAME).exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn map_tile_index_refuses_a_template_without_a_tile_extension() {
    let root = scratch_root("everon", 0, 0);
    let manifest = json!({
        "terrainId": "everon",
        "tiles": {"minZoom": 0, "maxZoom": 0,
                  "map": {"path": "tiles/map", "urlTemplate": "/map-assets/everon/tiles/map/{z}/{x}/{y}"}},
    });
    fs::write(
        terrain_dir(&root, "everon").join("manifest.json"),
        manifest.to_string(),
    )
    .expect("manifest");
    assert!(matches!(
        read_pyramid(&terrain_dir(&root, "everon")),
        Err(TileIndexError::UrlTemplateWithoutExtension(_))
    ));
    assert!(matches!(
        read_pyramid(&root.join("absent-terrain")),
        Err(TileIndexError::ManifestUnreadable(_))
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn map_tile_index_writes_a_schema_valid_index_next_to_the_pyramid() {
    let root = scratch_root("everon", 0, 2);
    write_tile(&root, "everon", 0, 0, 0, 5);
    write_tile(&root, "everon", 1, 1, 1, 6);
    let args = vec!["--terrain".to_owned(), "everon".to_owned()];
    assert_eq!(run_with_root(&root, &args).expect("run"), 0);

    let written = terrain_dir(&root, "everon").join("tiles/map/index.json");
    let text = fs::read_to_string(&written).expect("index written");
    let value: Value = serde_json::from_str(&text).expect("json");
    let validator = schema_validator();
    let errors: Vec<String> = validator
        .iter_errors(&value)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "schema errors: {errors:?}");
    assert_eq!(
        value,
        json!({"schemaVersion": 1, "terrainId": "everon",
               "tiles": [{"z": 0, "x": 0, "y": 0, "bytes": 5}, {"z": 1, "x": 1, "y": 1, "bytes": 6}]})
    );
    let pyramid = read_pyramid(&terrain_dir(&root, "everon")).expect("pyramid");
    let index = build_index(&pyramid).expect("index");
    assert_eq!(missing_zoom_levels(&pyramid, &index), vec![2]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn map_tile_index_schema_refuses_unknown_keys_and_a_wrong_version() {
    let validator = schema_validator();
    let good = json!({"schemaVersion": 1, "terrainId": "everon", "tiles": []});
    assert!(validator.is_valid(&good));
    let wrong_version = json!({"schemaVersion": 2, "terrainId": "everon", "tiles": []});
    assert!(!validator.is_valid(&wrong_version));
    let extra_key = json!({"schemaVersion": 1, "terrainId": "everon",
                           "tiles": [{"z": 0, "x": 0, "y": 0, "bytes": 1, "etag": "x"}]});
    assert!(!validator.is_valid(&extra_key));
    let snake_case = json!({"schema_version": 1, "terrain_id": "everon", "tiles": []});
    assert!(!validator.is_valid(&snake_case));
}
