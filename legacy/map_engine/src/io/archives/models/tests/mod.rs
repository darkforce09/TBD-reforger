//! Role: Module boundary for formats/archives/models/tests.
//! Position: `io/archives/models/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

use crate::io::containers::header::ContainerHeader;

use crate::io::containers::tbds::TbdsHeader;

use crate::io::archives::codec::BinaryError;

use crate::io::archives::codec::access_checked;

use crate::io::archives::codec::to_bytes;

use rkyv::rancor::Error as RkyvError;

fn round_trip<T>(value: &T) -> rkyv::util::AlignedVec
where
    T: rkyv::Archive
        + Clone
        + core::fmt::Debug
        + PartialEq
        + for<'a> rkyv::Serialize<
            rkyv::api::high::HighSerializer<
                rkyv::util::AlignedVec,
                rkyv::ser::allocator::ArenaHandle<'a>,
                RkyvError,
            >,
        >,
    rkyv::Archived<T>: rkyv::Portable
        + for<'a> rkyv::bytecheck::CheckBytes<rkyv::api::high::HighValidator<'a, RkyvError>>
        + rkyv::Deserialize<T, rkyv::api::high::HighDeserializer<RkyvError>>,
{
    let bytes = to_bytes(value).expect("serialise");
    let archived = access_checked::<T>(&bytes).expect("validated access");
    let back: T = rkyv::deserialize::<T, RkyvError>(archived).expect("deserialise");
    assert_eq!(&back, value, "deserialised value differs from the original");
    let again = to_bytes(&back).expect("re-serialise");
    assert_eq!(
        bytes.as_slice(),
        again.as_slice(),
        "re-serialising the round-tripped value produced different bytes"
    );
    bytes
}

fn corruption_is_rejected<T>(bytes: &rkyv::util::AlignedVec)
where
    T: rkyv::Archive,
    rkyv::Archived<T>: rkyv::Portable
        + for<'a> rkyv::bytecheck::CheckBytes<rkyv::api::high::HighValidator<'a, RkyvError>>,
{
    let mut rejected = 0_usize;
    let root = bytes.len().saturating_sub(size_of::<rkyv::Archived<T>>());
    for i in root..bytes.len() {
        let mut bad = bytes.clone();
        bad[i] ^= 0xFF;
        if let Err(e) = access_checked::<T>(&bad) {
            assert!(matches!(e, BinaryError::Archive { .. }), "{e}");
            rejected += 1;
        }
    }
    assert!(
        rejected > 0,
        "flipping every byte of the archive root was accepted — validation is not running"
    );

    for cut in [0, 1, bytes.len() / 2, bytes.len() - 1] {
        assert!(
            access_checked::<T>(&bytes[..cut]).is_err(),
            "a {cut}-byte prefix of a {}-byte archive was accepted",
            bytes.len()
        );
    }
}

fn road_network() -> RoadNetworkArchive {
    RoadNetworkArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        segments: vec![
            RoadSegmentArchive {
                id: "road-1".into(),
                road_class: 3,
                width_m: 2.5,
                centerline: vec![[0.0, 0.0], [10.5, -4.25], [22.0, 9.75]],
            },
            RoadSegmentArchive {
                id: "runway-0".into(),
                road_class: 6,
                width_m: 20.0,
                centerline: vec![[100.0, 100.0], [1100.0, 100.0]],
            },
        ],
    }
}

fn map_labels() -> MapLabelsArchive {
    MapLabelsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        towns: vec![TownLabel {
            name: "Montignac".into(),
            position: [6400.0, 6400.0],
            importance: 0.875,
            kind: "town".into(),
        }],
        height_labels: vec![HeightLabel {
            position: [1024.0, 2048.0],
            elevation_m: 375.53,
        }],
        road_names: vec![RoadNameLabel {
            name: "Route 1".into(),
            position: [12.0, 34.0],
            angle_deg: -33.5,
            road_class: 1,
        }],
    }
}

fn water_vectors() -> WaterVectorsArchive {
    WaterVectorsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        lakes: vec![WaterBody {
            id: "lake-0".into(),
            surface_y: 12.5,
            ring: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 80.0], [0.0, 80.0]],
        }],
        rivers: vec![WaterLine {
            id: "river-0".into(),
            width_m: 6.0,
            centerline: vec![[0.0, 0.0], [50.0, 60.0]],
        }],
        ponds: vec![],
    }
}

fn prefab_catalog() -> PrefabCatalogArchive {
    PrefabCatalogArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        prefabs: vec![PrefabEntry {
            prefab_id: 1622,
            kind: "building".into(),
            class: "house".into(),
            class_code: 2,
            label: "Farm House".into(),
            resource_name: "Prefabs/Buildings/FarmHouse_E_1L01_Wood.et".into(),
            half_extents: [6.5, 4.25, 8.0],
            height_m: 8.0,
            icon_key: "building".into(),
            base_size_px: 14.0,
            default_color: [0xC8, 0xB4, 0x96, 0xFF],
            importance_zoom: -2.5,
        }],
        type_inventory: TypeInventory {
            terrain_id: "everon".into(),
            census_status: "partial".into(),
            unique_prefabs: 1623,
            total_instances: 1_216_066,
            by_kind: vec![
                KindCensus {
                    kind: "building".into(),
                    prefab_types: 287,
                    instances: 4131,
                },
                KindCensus {
                    kind: "tree".into(),
                    prefab_types: 51,
                    instances: 501_828,
                },
            ],
        },
    }
}

fn forest_regions() -> ForestRegionsArchive {
    ForestRegionsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        regions: vec![ForestRegion {
            id: "forest-0".into(),
            kind: "forest".into(),
            polygon: vec![
                vec![[0.0, 0.0], [200.0, 0.0], [200.0, 150.0], [0.0, 150.0]],
                vec![[50.0, 50.0], [60.0, 50.0], [60.0, 60.0]],
            ],
            tree_count: 4821,
            dominant_species_class: "pine".into(),
            density_per_ha: 160.5,
            area_ha: 3.0,
            cover_type: "dense".into(),
        }],
    }
}

fn building_blueprints() -> BuildingBlueprintArchive {
    BuildingBlueprintArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        descriptors: vec![OccluderDescriptor {
            prefab_id: 1622,
            slug: "FarmHouse_E_1L01_Wood".into(),
            kind: "building".into(),
            blocks: true,
            canopy: false,
            local_bounds: [[-6.5, -4.25, 0.0], [6.5, 4.25, 8.0]],
            blas: vec![0, 1],
        }],
        blas_index: vec![
            BlasEntry {
                path: "blas/FarmHouse_E_1L01_Wood.bvh".into(),
                bytes: 40_960,
                tris: 1204,
                kinds: [1100, 96, 8],
            },
            BlasEntry {
                path: "blas/Door_Wood_01.bvh".into(),
                bytes: 2048,
                tris: 24,
                kinds: [24, 0, 0],
            },
        ],
        blueprints: vec![BuildingBlueprint {
            prefab_id: 1622,
            slug: "FarmHouse_E_1L01_Wood".into(),
            vertical_profile: VerticalProfile {
                pivot_elevation_offset_m: 0.0,
                foundation_skirt_depth_m: 0.35,
                total_height_m: 8.0,
                eave_height_m: 5.5,
                ridge_height_m: 8.0,
                roof_type: "gable".into(),
            },
            levels: vec![BuildingLevel {
                level_index: 0,
                elevation_range: [0.0, 2.8],
                footprint_polygon: vec![[0.0, 0.0], [13.0, 0.0], [13.0, 8.5], [0.0, 8.5]],
                walls: vec![WallRec {
                    id: "w0".into(),
                    start: [0.0, 0.0],
                    end: [13.0, 0.0],
                    thickness_m: 0.3,
                    is_exterior: true,
                    material: "brick".into(),
                }],
                doors: vec![DoorRec {
                    id: "d0".into(),
                    wall_id: "w0".into(),
                    position: [4.0, 0.0],
                    width_m: 0.9,
                    height_m: 2.1,
                    is_exterior: true,
                    has_glass: false,
                }],
                windows: vec![WindowRec {
                    id: "n0".into(),
                    wall_id: "w0".into(),
                    position: [9.0, 0.0],
                    width_m: 1.2,
                    sill_height_m: 0.9,
                    window_height_m: 1.4,
                    normal: [0.0, -1.0],
                    fov_deg: 140.0,
                    has_glass: true,
                }],
                stairs: vec![StairsRec {
                    id: "s0".into(),
                    bounds: [[10.0, 5.0], [12.0, 8.0]],
                    connects_to_level: 1,
                    direction_deg: 90.0,
                    step_count: 16,
                    transparent_steps: false,
                    los_concealment: 0.75,
                }],
                furniture: vec![FurnitureRec {
                    id: "f0".into(),
                    name: "Table".into(),
                    category: "table".into(),
                    position: [6.0, 4.0],
                    rotation_deg: 45.0,
                    height_m: 0.75,
                    blocks_movement: true,
                    los_cover: "partial".into(),
                }],
            }],
        }],
    }
}

fn sat_index() -> TbdSatIndexV2 {
    TbdSatIndexV2 {
        base_w: 12800,
        base_h: 12800,
        tile_px: 256,
        levels: vec![
            SatLevel {
                w_tiles: 50,
                h_tiles: 50,
                tiles: vec![
                    SatTile {
                        offset: 0,
                        len: 32_768,
                        format: 0,
                    },
                    SatTile {
                        offset: 32_768,
                        len: 30_000,
                        format: 0,
                    },
                ],
            },
            SatLevel {
                w_tiles: 25,
                h_tiles: 25,
                tiles: vec![SatTile {
                    offset: 62_768,
                    len: 28_000,
                    format: 0,
                }],
            },
        ],
    }
}

mod cases_1;
