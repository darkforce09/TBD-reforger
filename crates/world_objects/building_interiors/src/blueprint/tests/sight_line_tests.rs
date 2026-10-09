//! The blueprint's JSON shape, its level bands and its sight-line attribution over the synthetic
//! room and the FarmHouse blueprint; and the camelCase round trip of a compound instance record.
//!
//! **Role:** pins the FarmHouse blueprint parse, the band clipping on a shared floor, and every
//! attribution rule of [`crate::blueprint::sight_line`]: walls, windows, doors, roof, stairs,
//! furniture, solid hits and the plate grid round trip.
//! **Position:** mounted by `blueprint/mod.rs` under `cfg(test)`; builds its rooms from
//! [`crate::test_fixtures`].
//! **Signals & state:** none.
//! **Invariants:** every case states the event list or verdict it expects, never a tolerance
//! looser than the attribution's own.

use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;
use spatial_indexes::test_fixtures::cube;
use world_file_formats::ids::{FurnitureId, StairsId};

use crate::building_ids::BuildingFeatureId;

use crate::blueprint::footprint::*;
use crate::blueprint::sight_line::*;
use crate::blueprint::structure::*;
use crate::test_fixtures::{room_blueprint, room_sidecar};

fn farmhouse() -> BuildingBlueprint {
    let json_str = include_str!(
        "../../../../../../assets/terrains/everon/prefabs/buildings/FarmHouse_E_1L01.json"
    );
    serde_json::from_str(json_str).expect("Valid blueprint JSON")
}

#[test]
fn parses_farmhouse_blueprint_json() {
    let bp = farmhouse();
    assert_eq!(bp.prefab_id, "FarmHouse_E_1L01");
    assert_eq!(bp.levels.len(), 2);
    assert_eq!(bp.overall_footprint.polygon2_d.len(), 6);
    assert_eq!(bp.levels[0].windows.len(), 3);
    assert_eq!(bp.levels[0].doors.len(), 2);
    assert_eq!(bp.levels[0].stairs.len(), 1);
    assert!(bp.levels[0].stairs[0].transparent_steps);
}

#[test]
fn clip_t_to_band_horizontal_boundary_belongs_to_upper_level() {
    assert_eq!(clip_t_to_band(2.8, 2.8, [0.0, 2.8], false), None);
    assert_eq!(clip_t_to_band(2.8, 2.8, [2.8, 5.6], true), Some((0.0, 1.0)));
}

fn room() -> (BuildingBlueprint, BvhSidecar) {
    (room_blueprint(), room_sidecar(&[]))
}

#[test]
fn blocked_wall_is_named() {
    let (bp, sc) = room();
    let los = bp.annotate_sight_line(&sc, [1.0, 1.5, -3.0], [1.0, 1.5, 3.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some(BuildingFeatureId::new("w_s0")));
    assert!((los.concealment - 1.0).abs() < f64::EPSILON);

    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Wall);
    assert_eq!(last.id, "w_s0");
    assert!((last.concealment - 1.0).abs() < f64::EPSILON);

    assert!((last.pos[2] - -0.1).abs() < 1e-9, "pos = {:?}", last.pos);
    assert!((last.t - 2.9 / 6.0).abs() < 1e-9, "t = {}", last.t);
    assert_eq!(los.hits.len(), 1, "hits: {:?}", los.hits);
}

#[test]
fn window_is_named_and_traversed_through_mesh_hole() {
    let (bp, sc) = room();

    let los = bp.annotate_sight_line(&sc, [3.0, 1.5, -3.0], [3.0, 1.5, 3.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(
        los.window_ids_traversed,
        vec![BuildingFeatureId::new("win_s0")]
    );
    assert_eq!(los.blocked_by_wall_id, None);
    assert!(los.concealment.abs() < f64::EPSILON);
    let win = los
        .hits
        .iter()
        .find(|h| h.kind == LosHitKind::Window)
        .expect("window annotation");
    assert_eq!(win.id, "win_s0");
    assert!(win.pos[2].abs() < 1e-9, "annotation sits on the centerline");
    assert!((win.t - 0.5).abs() < 1e-9);
    assert!(win.concealment.abs() < f64::EPSILON);

    let los = bp.annotate_sight_line(&sc, [3.0, 1.5, -3.0], [3.0, 1.5, 7.0]);
    assert!(!los.is_clear);
    assert_eq!(
        los.window_ids_traversed,
        vec![BuildingFeatureId::new("win_s0")]
    );
    assert_eq!(los.blocked_by_wall_id, Some(BuildingFeatureId::new("w_n0")));
    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Wall);
    assert!((last.pos[2] - 5.9).abs() < 1e-9, "pos = {:?}", last.pos);
    assert_eq!(los.hits.len(), 2);
}

#[test]
fn door_is_traversed() {
    let (bp, sc) = room();
    let los = bp.annotate_sight_line(&sc, [3.0, 1.0, 3.0], [9.0, 1.0, 3.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(
        los.door_ids_traversed,
        vec![BuildingFeatureId::new("door_e0")]
    );
    assert!(los.window_ids_traversed.is_empty());
    let door = los.hits.last().expect("door annotation");
    assert_eq!(door.kind, LosHitKind::DoorOpen);
    assert!((door.pos[0] - 6.0).abs() < 1e-9);

    let los = bp.annotate_sight_line(&sc, [3.0, 2.5, 3.0], [9.0, 2.5, 3.0]);
    assert!(!los.is_clear);
    assert!(los.door_ids_traversed.is_empty());
    assert_eq!(los.blocked_by_wall_id, Some(BuildingFeatureId::new("w_e0")));
}

#[test]
fn cross_band_blames_correct_floors_wall() {
    let (bp, sc) = room();
    let los = bp.annotate_sight_line(&sc, [1.0, 1.0, -4.0], [1.0, 5.0, 2.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some(BuildingFeatureId::new("w_s1")));
    assert!(
        los.hits.iter().all(|h| h.id != "w_s0"),
        "hits: {:?}",
        los.hits
    );
    let last = los.hits.last().expect("terminal hit");
    assert!((last.t - 0.65).abs() < 1e-9, "t = {}", last.t);
    assert!((last.pos[1] - 3.6).abs() < 1e-9, "y = {}", last.pos[1]);
}

#[test]
fn solid_when_blueprint_silent() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[cube([3.0, 1.5, 3.0], [0.3, 1.5, 0.3])]);
    let los = bp.annotate_sight_line(&sc, [1.0, 1.5, 3.0], [5.0, 1.5, 3.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, None);
    assert_eq!(los.cover_furniture_id, None);
    assert!((los.concealment - 1.0).abs() < f64::EPSILON);
    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Solid);
    assert_eq!(last.id, "solid");
    assert!((last.pos[0] - 2.7).abs() < 1e-9, "pos = {:?}", last.pos);
    assert_eq!(los.hits.len(), 1);

    let (bp, sc) = room();
    let los = bp.annotate_sight_line(&sc, [1.0, 1.5, 3.0], [5.0, 1.5, 3.0]);
    assert!(los.is_clear && los.hits.is_empty(), "hits: {:?}", los.hits);
}

#[test]
fn roof_attribution_near_surface() {
    let (bp, sc) = room();
    let los = bp.annotate_sight_line(&sc, [3.0, 8.0, 3.0], [3.0, 4.5, 3.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, None);
    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Roof);
    assert_eq!(last.id, "roof");
    assert!(
        (last.pos[1] - 6.2).abs() < 1e-9,
        "pierce height = {}",
        last.pos[1]
    );
    assert_eq!(los.hits.len(), 1);

    let mut broken = room_blueprint();
    broken.roof.as_mut().expect("has roof").heights_m.pop();
    assert!(!broken.roof.as_ref().expect("has roof").is_valid());
    let los = broken.annotate_sight_line(&sc, [3.0, 8.0, 3.0], [3.0, 4.5, 3.0]);
    assert!(!los.is_clear);
    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Solid);
    assert!((last.pos[1] - 6.2).abs() < 1e-9);
}

fn table(los_cover: &str) -> BuildingFurniture {
    BuildingFurniture {
        id: FurnitureId::new("furn_table_01"),
        name: "table".to_string(),
        category: "prop".to_string(),
        prefab_resource: "synthetic://table".to_string(),
        pos2_d: [4.0, 4.0],
        rotation_deg: 0.0,
        size2_d: [1.2, 0.8],
        height_m: 0.78,
        blocks_movement: true,
        los_cover: los_cover.to_string(),
    }
}

#[test]
fn furniture_low_cover_conceals_without_blocking() {
    let mut bp = room_blueprint();
    bp.levels[0].furniture.push(table("low_cover"));
    let sc = room_sidecar(&[]);

    let los = bp.annotate_sight_line(&sc, [3.0, 0.6, 4.0], [5.0, 0.4, 4.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(
        los.cover_furniture_id,
        Some(BuildingFeatureId::new("furn_table_01"))
    );
    assert!((los.concealment - 0.60).abs() < f64::EPSILON);
    assert!(
        los.hits
            .iter()
            .any(|h| h.kind == LosHitKind::Furniture && h.id == "furn_table_01")
    );

    let los = bp.annotate_sight_line(&sc, [3.0, 1.0, 4.0], [5.0, 1.0, 4.0]);
    assert!(los.is_clear && los.hits.is_empty(), "hits: {:?}", los.hits);
}

#[test]
fn furniture_full_cover_is_terminal_without_mesh_hit() {
    let mut bp = room_blueprint();
    bp.levels[0].furniture.push(table("full_cover"));
    let sc = room_sidecar(&[]);
    let los = bp.annotate_sight_line(&sc, [3.0, 0.6, 4.0], [5.0, 0.4, 4.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, None);
    assert_eq!(
        los.cover_furniture_id,
        Some(BuildingFeatureId::new("furn_table_01"))
    );
    assert!((los.concealment - 1.0).abs() < f64::EPSILON);
    assert_eq!(los.hits.len(), 1);
    let last = &los.hits[0];
    assert_eq!(last.kind, LosHitKind::Furniture);
    assert!((last.concealment - 1.0).abs() < f64::EPSILON);
}

#[test]
fn stairs_conceal_transparent_treads() {
    let mut bp = room_blueprint();
    bp.levels[0].stairs.push(BuildingStairs {
        id: StairsId::new("stairs_01"),
        bounds: [[1.0, 1.0], [2.0, 2.0]],
        connects_to_level: 1,
        direction_deg: 0.0,
        step_count: 10,
        transparent_steps: true,
        los_concealment: 0.3,
    });
    let sc = room_sidecar(&[]);
    let los = bp.annotate_sight_line(&sc, [0.5, 1.0, 1.5], [3.0, 1.0, 1.5]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert!((los.concealment - 0.3).abs() < f64::EPSILON);
    assert_eq!(los.hits.len(), 1);
    assert_eq!(los.hits[0].kind, LosHitKind::Stairs);
    assert_eq!(los.hits[0].id, "stairs_01");
    assert!(
        (los.hits[0].pos[0] - 1.0).abs() < 1e-9,
        "entry at the AABB edge"
    );

    bp.levels[0].stairs[0].transparent_steps = false;
    let los = bp.annotate_sight_line(&sc, [0.5, 1.0, 1.5], [3.0, 1.0, 1.5]);
    assert!(los.is_clear && los.hits.is_empty());
}

#[test]
fn terminal_hit_inside_aperture_is_not_traversed() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[cube([3.0, 1.5, 0.0], [0.05, 0.5, 0.1])]);
    let los = bp.annotate_sight_line(&sc, [3.0, 1.5, -3.0], [3.0, 1.5, 3.0]);
    assert!(!los.is_clear);
    assert!(los.window_ids_traversed.is_empty(), "{:?}", los.hits);
    assert_eq!(los.blocked_by_wall_id, None);
    assert!((los.concealment - 1.0).abs() < f64::EPSILON);
    assert_eq!(los.hits.len(), 1, "hits: {:?}", los.hits);
    let last = &los.hits[0];
    assert_eq!(last.kind, LosHitKind::Window);
    assert_eq!(last.id, "win_s0");
    assert!((last.concealment - 1.0).abs() < f64::EPSILON);
    assert!(
        (last.pos[2] - -0.1).abs() < 1e-9,
        "stopped on the mullion face"
    );
}

#[test]
fn blueprint_without_roof_is_unchanged() {
    let bp = farmhouse();
    assert!(bp.roof.is_none());
    let v = serde_json::to_value(&bp).expect("serialize");
    assert!(v.get("roof").is_none(), "absent roof must not serialize");

    let room = room_blueprint();
    let roof = room.roof.as_ref().expect("has roof");
    assert!(roof.is_valid());
    assert_eq!(roof.height_at(3.0, 3.0), Some(6.0));
    assert_eq!(roof.height_at(-1.0, 3.0), None, "outside grid");
    let mut broken = roof.clone();
    broken.heights_m.pop();
    assert!(!broken.is_valid());
}

#[test]
fn plate_grid_and_floor_polygons_round_trip() {
    let bp = farmhouse();
    assert!(bp.levels.iter().all(|l| l.plate.is_none()));
    assert!(bp.levels.iter().all(|l| l.floor_polygons.is_empty()));
    let v = serde_json::to_value(&bp).expect("serialize");
    let l0 = &v["levels"][0];
    assert!(l0.get("plate").is_none(), "absent plate must not serialize");
    assert!(
        l0.get("floorPolygons").is_none(),
        "empty floorPolygons must not serialize"
    );

    let mut bp = room_blueprint();
    bp.levels[0].plate = Some(PlateGrid {
        origin: [0.0, 0.0],
        cell_size_m: 0.5,
        nx: 2,
        nz: 2,
        heights_m: vec![Some(0.1), None, Some(0.15), Some(0.1)],
    });
    bp.levels[0].floor_polygons = vec![FloorPolygon {
        outer: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        holes: vec![vec![[0.4, 0.4], [0.4, 0.6], [0.6, 0.6], [0.6, 0.4]]],
    }];
    let json = serde_json::to_string(&bp).expect("serialize");
    assert!(json.contains("\"cellSizeM\":0.5") && json.contains("\"heightsM\""));
    let back: BuildingBlueprint = serde_json::from_str(&json).expect("reparse");
    assert_eq!(back, bp);

    let plate = back.levels[0].plate.as_ref().expect("plate");
    assert!(plate.is_valid());
    assert_eq!(plate.height_at(0.25, 0.25), Some(0.1));
    assert_eq!(plate.height_at(0.25, 0.75), None, "void cell");
    assert_eq!(plate.height_at(-0.1, 0.2), None, "outside grid");
    let mut bad = plate.clone();
    bad.heights_m.pop();
    assert!(!bad.is_valid());
}

#[test]
fn upstairs_window_is_traversed_flat_and_cross_band() {
    let (bp, sc) = room();
    let los = bp.annotate_sight_line(&sc, [3.0, 4.5, -3.0], [3.0, 4.5, 3.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(
        los.window_ids_traversed,
        vec![BuildingFeatureId::new("win_s1")]
    );

    let los = bp.annotate_sight_line(&sc, [3.0, 3.0, -6.0], [3.0, 5.0, 2.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(
        los.window_ids_traversed,
        vec![BuildingFeatureId::new("win_s1")]
    );
    assert!(
        los.hits.iter().all(|h| !h.id.as_str().ends_with('0')),
        "no level-0 events"
    );

    let los = bp.annotate_sight_line(&sc, [3.0, 2.0, -6.0], [3.0, 4.0, 2.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some(BuildingFeatureId::new("w_s1")));
    assert!(los.window_ids_traversed.is_empty());
}

/// The instances file's JSON shape: a record written and read back equals the original.
mod compound_instances {
    use geometry_primitives::rigid_transform::Rigid;

    use crate::compound::assembly::*;
    use crate::compound::doors::*;
    use crate::compound::instances::*;

    #[test]
    fn instances_json_round_trips_camel_case() {
        let door = InstanceRecord {
            id: "door_int_left_01/leaf".into(),
            kind: InstanceKind::DoorLeaf,
            prefab: "Prefabs/Doors/Leaf.et".into(),
            blas: "blas/Leaf.bvh".into(),
            xob: Some("Assets/Doors/Leaf.xob".into()),
            local: LocalTransform::from_rigid(&Rigid::from_enfusion(
                [1.0, 0.0, -2.0],
                [0.0, 90.0, 0.0],
                1.0,
            )),
            door: Some(DoorRecord {
                angle_range_deg: -120.0,
                closed_angle_deg: 0.0,
                initial_angle_deg: 0.0,
                angle_range_explicit: true,
                opened_distance: None,
            }),
            cover: CoverTier::Full,
            source: PlacementSource::XobSocket,
            parent: Some("door_int_left_01".into()),
        };
        let file = InstancesFile {
            schema_version: INSTANCES_SCHEMA_VERSION.into(),
            prefab_id: "House".into(),
            resource_name: "Prefabs/Houses/House.et".into(),
            shell_bvh: "House.bvh".into(),
            instances: vec![
                door.clone(),
                InstanceRecord {
                    id: "furn/T1".into(),
                    kind: InstanceKind::Furniture,
                    prefab: "Prefabs/Props/Table.et".into(),
                    blas: "blas/Table.bvh".into(),
                    xob: None,
                    local: LocalTransform::identity(),
                    door: None,
                    cover: CoverTier::Low,
                    source: PlacementSource::PrefabCoords,
                    parent: None,
                },
            ],
            notes: vec![],
        };
        let json = serde_json::to_string_pretty(&file).unwrap();
        assert!(json.contains("\"schemaVersion\""), "{json}");
        assert!(json.contains("\"kind\": \"doorLeaf\""));
        assert!(json.contains("\"cover\": \"full\""));
        assert!(json.contains("\"source\": \"xobSocket\""));
        assert!(json.contains("\"angleRangeDeg\": -120.0"));
        assert!(!json.contains("openedDistance"), "None is skipped");
        assert!(!json.contains("\"notes\""), "empty notes are skipped");
        let back: InstancesFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, file);
        assert_eq!(back.blas_paths(), ["blas/Leaf.bvh", "blas/Table.bvh"]);

        let r = back.instances[0].local.rigid();
        let d = r.dir([1.0, 0.0, 0.0]);
        assert!(d[0].abs() < 1e-9 && (d[2] + 1.0).abs() < 1e-9);

        let minimal: LocalTransform =
            serde_json::from_str(r#"{"pos":[0,0,0],"quat":[0,0,0,1]}"#).unwrap();
        assert_eq!(minimal.scale, 1.0);
    }
}
