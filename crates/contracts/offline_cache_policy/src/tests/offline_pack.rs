//! Tests for [`super`] — the terrain offline pack over the committed Everon manifest.

use super::*;

const EVERON_MANIFEST: &str = include_str!("../../../../../assets/terrains/everon/manifest.json");
const ROOT: &str = "/map-assets";

/// A full pyramid index for zoom levels `0..=max_zoom`, every tile 100 bytes.
fn full_index(terrain: &str, max_zoom: u32) -> MapTileIndex {
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
        terrain_id: TerrainId::new(terrain),
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
