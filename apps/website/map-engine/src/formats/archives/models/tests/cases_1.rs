//! Role: Domain regression cases.
//! Position: `formats/archives/models/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

#[test]
fn road_network_round_trips_and_rejects_corruption() {
    let v = road_network();
    let bytes = round_trip(&v);
    let a = access_checked::<RoadNetworkArchive>(&bytes).expect("access");
    assert_eq!(a.segments.len(), 2);
    assert_eq!(a.segments[0].id.as_str(), "road-1");
    assert_eq!(a.segments[1].width_m.to_native(), 20.0);
    assert_eq!(a.segments[0].centerline.len(), 3);
    corruption_is_rejected::<RoadNetworkArchive>(&bytes);
}

#[test]
fn map_labels_round_trips_and_rejects_corruption() {
    let bytes = round_trip(&map_labels());
    let a = access_checked::<MapLabelsArchive>(&bytes).expect("access");
    assert_eq!(a.towns[0].name.as_str(), "Montignac");
    assert_eq!(a.height_labels.len(), 1);
    assert_eq!(a.road_names[0].road_class, 1);
    corruption_is_rejected::<MapLabelsArchive>(&bytes);
}

#[test]
fn water_vectors_round_trips_and_rejects_corruption() {
    let bytes = round_trip(&water_vectors());
    let a = access_checked::<WaterVectorsArchive>(&bytes).expect("access");
    assert_eq!(a.lakes[0].ring.len(), 4);
    assert_eq!(a.rivers[0].width_m.to_native(), 6.0);
    assert!(
        a.ponds.is_empty(),
        "an empty Vec must survive the round trip"
    );
    corruption_is_rejected::<WaterVectorsArchive>(&bytes);
}

#[test]
fn prefab_catalog_round_trips_and_rejects_corruption() {
    let bytes = round_trip(&prefab_catalog());
    let a = access_checked::<PrefabCatalogArchive>(&bytes).expect("access");
    assert_eq!(a.prefabs[0].prefab_id.to_native(), 1622);
    assert_eq!(a.type_inventory.total_instances.to_native(), 1_216_066);
    assert_eq!(a.type_inventory.by_kind.len(), 2);
    corruption_is_rejected::<PrefabCatalogArchive>(&bytes);
}

#[test]
fn forest_regions_round_trips_and_rejects_corruption() {
    let bytes = round_trip(&forest_regions());
    let a = access_checked::<ForestRegionsArchive>(&bytes).expect("access");
    assert_eq!(a.regions[0].polygon.len(), 2, "outer ring plus one hole");
    assert_eq!(a.regions[0].tree_count.to_native(), 4821);
    corruption_is_rejected::<ForestRegionsArchive>(&bytes);
}

#[test]
fn building_blueprints_round_trip_and_reject_corruption() {
    let bytes = round_trip(&building_blueprints());
    let a = access_checked::<BuildingBlueprintArchive>(&bytes).expect("access");
    assert_eq!(a.descriptors[0].blas.len(), 2);
    assert_eq!(a.blas_index[0].tris.to_native(), 1204);
    let level = &a.blueprints[0].levels[0];
    assert_eq!(level.level_index, 0);
    assert_eq!(level.walls[0].id.as_str(), "w0");
    assert_eq!(level.doors[0].wall_id.as_str(), "w0");
    assert_eq!(level.windows[0].fov_deg.to_native(), 140.0);
    assert_eq!(level.stairs[0].step_count.to_native(), 16);
    assert_eq!(level.furniture[0].los_cover.as_str(), "partial");
    corruption_is_rejected::<BuildingBlueprintArchive>(&bytes);
}

#[test]
fn sat_index_round_trips_and_rejects_corruption() {
    let bytes = round_trip(&sat_index());
    let a = access_checked::<TbdSatIndexV2>(&bytes).expect("access");
    assert_eq!(a.tile_px.to_native(), 256);
    assert_eq!(a.levels.len(), 2);
    assert_eq!(a.levels[0].tiles[1].offset.to_native(), 32_768);
    corruption_is_rejected::<TbdSatIndexV2>(&bytes);
}

#[test]
fn sat_v2_header_frames_an_index_that_still_validates() {
    let index = to_bytes(&sat_index()).expect("serialise index");
    let head = TbdsHeader::new(u32::try_from(index.len()).expect("index fits u32"));
    let tiles: [u8; 12] = [0xAB; 12];

    let mut file = Vec::with_capacity(head.tiles_offset() + tiles.len());
    file.extend_from_slice(&head.to_header_bytes());
    file.extend_from_slice(&index);
    file.extend_from_slice(&tiles);
    assert_eq!(file.len(), head.tiles_offset() + tiles.len());

    let (h, payload) = TbdsHeader::read(&file).expect("header");
    let index_bytes = h.index(payload).expect("index block");
    assert_eq!(index_bytes, index.as_slice());

    let mut aligned = rkyv::util::AlignedVec::<16>::new();
    aligned.extend_from_slice(index_bytes);
    let a = access_checked::<TbdSatIndexV2>(&aligned).expect("index validates in place");
    assert_eq!(a.base_w.to_native(), 12800);
    assert_eq!(&file[h.tiles_offset()..], &tiles[..]);
}

#[test]
fn a_different_archive_type_does_not_validate() {
    let bytes = to_bytes(&map_labels()).expect("serialise");
    let err = access_checked::<TbdSatIndexV2>(&bytes)
        .expect_err("MapLabelsArchive bytes are not a TbdSatIndexV2");
    assert!(matches!(err, BinaryError::Archive { .. }), "{err}");
}

#[test]
fn empty_archives_round_trip() {
    round_trip(&RoadNetworkArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        segments: vec![],
    });
    round_trip(&ForestRegionsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        regions: vec![],
    });
    round_trip(&TbdSatIndexV2 {
        base_w: 0,
        base_h: 0,
        tile_px: 0,
        levels: vec![],
    });
}
