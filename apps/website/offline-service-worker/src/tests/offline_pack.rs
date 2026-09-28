//! Tests for [`super`] — the terrain offline pack over the committed Everon and Arland manifests.

use super::*;

const EVERON_MANIFEST: &str =
    include_str!("../../../../../assets_v2/terrains/everon/manifest.json");
const ARLAND_MANIFEST: &str =
    include_str!("../../../../../assets_v2/terrains/arland/manifest.json");
const ROOT: &str = "/map-assets";

/// A full pyramid index for zoom levels `0..=max_zoom`, every tile 100 bytes.
fn full_index(terrain_id: &str, max_zoom: u32) -> MapTileIndex {
    let mut tiles = Vec::new();
    for z in 0..=max_zoom {
        for x in 0..1u32 << z {
            for y in 0..1u32 << z {
                tiles.push(MapTileIndexEntry {
                    z,
                    x,
                    y,
                    bytes: 100,
                });
            }
        }
    }
    MapTileIndex {
        schema_version: MAP_TILE_INDEX_SCHEMA_VERSION,
        terrain_id: terrain_id.to_owned(),
        tiles,
    }
}

fn json(index: &MapTileIndex) -> String {
    serde_json::to_string(index).unwrap()
}

#[test]
fn the_everon_pack_lists_manifest_elevation_mosaic_and_every_map_tile() {
    let pack = terrain_pack(ROOT, EVERON_MANIFEST, Some(&json(&full_index("everon", 6)))).unwrap();
    assert_eq!(pack.terrain_id, "everon");
    assert!(pack.is_complete());
    assert_eq!(pack.entries.len(), 3 + 5461);
    assert_eq!(
        pack.entries[..3]
            .iter()
            .map(|entry| (entry.url.as_str(), entry.kind))
            .collect::<Vec<_>>(),
        vec![
            (
                "/map-assets/everon/manifest.json",
                PackEntryKind::TerrainManifest
            ),
            (
                "/map-assets/everon/dem/everon-dem-16bit.png",
                PackEntryKind::ElevationModel
            ),
            (
                "/map-assets/everon/satellite/everon-sat.tbd-sat",
                PackEntryKind::SatelliteMosaic
            ),
        ]
    );
    assert_eq!(pack.entries[2].expected_bytes, Some(152_713_114));
    assert_eq!(
        pack.entries[3].url,
        "/map-assets/everon/tiles/map/0/0/0.webp"
    );
    assert!(
        pack.entries
            .iter()
            .all(|entry| !entry.url.contains("/tiles/satellite/"))
    );
    assert_eq!(pack.declared_bytes(), 152_713_114 + 5461 * 100);
}

#[test]
fn a_missing_empty_or_partial_index_is_incomplete_and_never_complete() {
    let missing = terrain_pack(ROOT, EVERON_MANIFEST, None).unwrap();
    assert_eq!(
        missing.completeness,
        PackCompleteness::Incomplete(IncompleteReason::TileIndexMissing)
    );
    assert_eq!(missing.entries.len(), 3);
    assert!(!missing.is_complete());

    let mut empty = full_index("everon", 6);
    empty.tiles.clear();
    assert_eq!(
        terrain_pack(ROOT, EVERON_MANIFEST, Some(&json(&empty)))
            .unwrap()
            .completeness,
        PackCompleteness::Incomplete(IncompleteReason::TileIndexEmpty)
    );

    let shallow =
        terrain_pack(ROOT, EVERON_MANIFEST, Some(&json(&full_index("everon", 4)))).unwrap();
    assert_eq!(
        shallow.completeness,
        PackCompleteness::Incomplete(IncompleteReason::ZoomLevelsMissing(vec![5, 6]))
    );
}

#[test]
fn a_stub_terrain_or_a_malformed_document_is_an_error_and_not_an_empty_pack() {
    assert_eq!(
        terrain_pack(ROOT, ARLAND_MANIFEST, None),
        Err(OfflinePackError::StubElevationModel)
    );
    assert!(matches!(
        terrain_pack(ROOT, "{not json", None),
        Err(OfflinePackError::ManifestUnreadable(_))
    ));
    assert!(matches!(
        terrain_pack(ROOT, EVERON_MANIFEST, Some("[]")),
        Err(OfflinePackError::TileIndexUnreadable(_))
    ));
    let mut unknown_field: serde_json::Value =
        serde_json::to_value(full_index("everon", 0)).unwrap();
    unknown_field["extra"] = serde_json::json!(true);
    assert!(matches!(
        terrain_pack(ROOT, EVERON_MANIFEST, Some(&unknown_field.to_string())),
        Err(OfflinePackError::TileIndexUnreadable(_))
    ));
}

#[test]
fn an_index_of_another_terrain_version_or_grid_is_refused() {
    assert_eq!(
        terrain_pack(ROOT, EVERON_MANIFEST, Some(&json(&full_index("arland", 6)))),
        Err(OfflinePackError::TerrainMismatch {
            manifest: "everon".to_owned(),
            index: "arland".to_owned(),
        })
    );
    let mut future = full_index("everon", 6);
    future.schema_version = 2;
    assert_eq!(
        terrain_pack(ROOT, EVERON_MANIFEST, Some(&json(&future))),
        Err(OfflinePackError::UnsupportedTileIndexVersion(2))
    );
    for outside in [
        MapTileIndexEntry {
            z: 7,
            x: 0,
            y: 0,
            bytes: 1,
        },
        MapTileIndexEntry {
            z: 2,
            x: 4,
            y: 0,
            bytes: 1,
        },
        MapTileIndexEntry {
            z: 2,
            x: 0,
            y: 4,
            bytes: 1,
        },
    ] {
        let mut index = full_index("everon", 6);
        index.tiles.push(outside);
        assert_eq!(
            terrain_pack(ROOT, EVERON_MANIFEST, Some(&json(&index))),
            Err(OfflinePackError::TileOutsidePyramid(outside))
        );
    }
}

#[test]
fn manifest_sections_the_pack_needs_are_required() {
    let everon: serde_json::Value = serde_json::from_str(EVERON_MANIFEST).unwrap();
    for (pointer, section) in [
        ("/dem", "dem"),
        ("/tiles", "tiles"),
        ("/tiles/satellite/unified", "tiles.satellite.unified"),
        ("/tiles/map", "tiles.map"),
        ("/tiles/minZoom", "tiles.minZoom"),
        ("/tiles/maxZoom", "tiles.maxZoom"),
    ] {
        let mut manifest = everon.clone();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        let parent = if parent.is_empty() {
            &mut manifest
        } else {
            manifest.pointer_mut(parent).unwrap()
        };
        parent.as_object_mut().unwrap().remove(key).unwrap();
        assert_eq!(
            terrain_pack(ROOT, &manifest.to_string(), None),
            Err(OfflinePackError::ManifestSectionMissing(section)),
            "{pointer}"
        );
    }
    let mut bad_template = everon.clone();
    bad_template["tiles"]["map"]["urlTemplate"] =
        serde_json::json!("/map-assets/everon/tiles/map/{z}.webp");
    assert!(matches!(
        terrain_pack(ROOT, &bad_template.to_string(), None),
        Err(OfflinePackError::MapUrlTemplateInvalid(_))
    ));
    let mut bad_id = everon;
    bad_id["terrainId"] = serde_json::json!("../etc");
    assert_eq!(
        terrain_pack(ROOT, &bad_id.to_string(), None),
        Err(OfflinePackError::InvalidTerrainId("../etc".to_owned()))
    );
}

#[test]
fn the_served_paths_of_manifest_and_tile_index_follow_the_root() {
    assert_eq!(
        manifest_url("/map-assets/", "everon"),
        "/map-assets/everon/manifest.json"
    );
    assert_eq!(
        tile_index_url("/map-assets", "everon"),
        "/map-assets/everon/tiles/map/index.json"
    );
}
