//! Unit tests for the blueprint contract and the LOS evaluator over the BVH sidecar — split out
//! of `building_blueprint.rs` (SIZE gate) via `#[path]`, same super-scope semantics.
//!
//! The LOS battery runs on a synthetic two-level box room: a hand-built blueprint (four walls
//! per floor, one window per floor, one open door, a flat roof grid) paired with a COLL-style
//! slab trimesh that really has the holes the apertures describe — so every verdict is the
//! mesh's and every name is the blueprint's.

use super::*;
use crate::bvh::tests::{Scene, concat, cube};
use crate::bvh::{Bvh, BvhSidecar};

fn farmhouse() -> BuildingBlueprint {
    let json_str =
        include_str!("../../../packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01.json");
    serde_json::from_str(json_str).expect("Valid blueprint JSON")
}

#[test]
fn parses_farmhouse_blueprint_json() {
    let bp = farmhouse();
    assert_eq!(bp.prefab_id, "FarmHouse_E_1L01");
    assert_eq!(bp.levels.len(), 2);
    assert_eq!(bp.overall_footprint.polygon2_d.len(), 6); // L-shape 6 vertices
    assert_eq!(bp.levels[0].windows.len(), 3);
    assert_eq!(bp.levels[0].doors.len(), 2);
    assert_eq!(bp.levels[0].stairs.len(), 1);
    assert!(bp.levels[0].stairs[0].transparent_steps);
}

#[test]
fn clip_t_to_band_horizontal_boundary_belongs_to_upper_level() {
    // A horizontal ray exactly on the shared 2.8 boundary: excluded from the half-open
    // ground band, included in the closed top band.
    assert_eq!(clip_t_to_band(2.8, 2.8, [0.0, 2.8], false), None);
    assert_eq!(clip_t_to_band(2.8, 2.8, [2.8, 5.6], true), Some((0.0, 1.0)));
}

/* ───────────── synthetic two-level box room: blueprint + COLL-style slab mesh ───────────── */

/// Axis-aligned slab from absolute extents `[x0, x1] × [y0, y1] × [z0, z1]`.
fn slab(x: [f64; 2], y: [f64; 2], z: [f64; 2]) -> Scene {
    cube(
        [
            0.5 * (x[0] + x[1]),
            0.5 * (y[0] + y[1]),
            0.5 * (z[0] + z[1]),
        ],
        [
            0.5 * (x[1] - x[0]),
            0.5 * (y[1] - y[0]),
            0.5 * (z[1] - z[0]),
        ],
    )
}

fn wall(id: &str, start: [f64; 2], end: [f64; 2]) -> BuildingWall {
    BuildingWall {
        id: id.to_string(),
        start,
        end,
        thickness: 0.2,
        is_exterior: true,
        material: "synthetic".to_string(),
    }
}

fn window(id: &str, wall_id: &str, pos2_d: [f64; 2]) -> BuildingWindow {
    BuildingWindow {
        id: id.to_string(),
        prefab_resource: "synthetic://window".to_string(),
        wall_id: wall_id.to_string(),
        pos2_d,
        width_m: 1.0,
        sill_height_m: 1.0,
        window_height_m: 1.0,
        normal: [0.0, -1.0],
        fov_deg: 120.0,
        has_glass: true,
        glass_pane_count: 1,
    }
}

/// One 6 × 6 m level: walls `w_{s,n,w,e}{suffix}` on the centerlines z = 0 / z = 6 / x = 0 /
/// x = 6, one window `win_s{suffix}` centered on the south wall (x = 3, sill +1, height 1).
fn level(index: usize, band: [f64; 2], suffix: &str) -> BuildingLevel {
    BuildingLevel {
        level_index: index,
        name: format!("L{index}"),
        elevation_range: band,
        slice_height_m: 0.5 * (band[0] + band[1]),
        footprint_polygon: vec![[0.0, 0.0], [6.0, 0.0], [6.0, 6.0], [0.0, 6.0]],
        plate: None,
        floor_polygons: vec![],
        walls: vec![
            wall(&format!("w_s{suffix}"), [0.0, 0.0], [6.0, 0.0]),
            wall(&format!("w_n{suffix}"), [0.0, 6.0], [6.0, 6.0]),
            wall(&format!("w_w{suffix}"), [0.0, 0.0], [0.0, 6.0]),
            wall(&format!("w_e{suffix}"), [6.0, 0.0], [6.0, 6.0]),
        ],
        doors: vec![],
        windows: vec![window(
            &format!("win_s{suffix}"),
            &format!("w_s{suffix}"),
            [3.0, 0.0],
        )],
        stairs: vec![],
        furniture: vec![],
    }
}

/// Two-level 6 × 6 m box room, bands [0, 3) and [3, 6], flat roof at 6.0: one window per floor
/// on the south wall (x ∈ [2.5, 3.5], y ∈ [1, 2] / [4, 5]), an open door in the ground-floor
/// east wall (z ∈ [2.5, 3.5], y < 2.1).
fn room_blueprint() -> BuildingBlueprint {
    let square = vec![[0.0, 0.0], [6.0, 0.0], [6.0, 6.0], [0.0, 6.0]];
    let mut ground = level(0, [0.0, 3.0], "0");
    ground.doors.push(BuildingDoor {
        id: "door_e0".to_string(),
        prefab_resource: "synthetic://door".to_string(),
        wall_id: "w_e0".to_string(),
        pos2_d: [6.0, 3.0],
        width_m: 1.0,
        height_m: 2.1,
        hinge_side: "left".to_string(),
        swing_direction: "in".to_string(),
        is_exterior: true,
        has_glass: false,
        default_state: "open".to_string(),
    });
    BuildingBlueprint {
        schema_version: "1.0.0".to_string(),
        prefab_id: "SyntheticRoom".to_string(),
        resource_name: "synthetic://room".to_string(),
        model_mesh: None,
        label: None,
        kind: "building".to_string(),
        category: "generic".to_string(),
        destructible: false,
        vertical_profile: VerticalProfile {
            pivot_elevation_offset_m: 0.0,
            foundation_skirt_depth_m: 0.0,
            total_height_m: 6.2,
            eave_height_m: 6.0,
            ridge_height_m: 6.0,
            chimney_height_m: None,
            roof_type: "flat".to_string(),
        },
        overall_footprint: OverallFootprint {
            polygon2_d: square,
            bounding_box2_d: BBox2D {
                min: [0.0, 0.0],
                max: [6.0, 6.0],
                width_m: 6.0,
                depth_m: 6.0,
            },
            footprint_sq_m: 36.0,
        },
        roof: Some(RoofGrid {
            origin: [0.0, 0.0],
            cell_size_m: 1.0,
            nx: 6,
            nz: 6,
            heights_m: vec![Some(6.0); 36],
        }),
        levels: vec![ground, level(1, [3.0, 6.0], "1")],
    }
}

/// The COLL-style trimesh matching [`room_blueprint`]: 0.2 m slabs on the wall centerlines with
/// the window / door holes cut, plus the roof slab. `extra` scenes are appended verbatim (a
/// pillar the blueprint does not know about, a mullion inside a window hole).
fn room_sidecar(extra: &[Scene]) -> BvhSidecar {
    let mut scenes = vec![
        // South wall (z = 0): window holes x ∈ [2.5, 3.5] at y ∈ [1, 2] (L0) and [4, 5] (L1).
        slab([0.0, 2.5], [0.0, 6.0], [-0.1, 0.1]),
        slab([3.5, 6.0], [0.0, 6.0], [-0.1, 0.1]),
        slab([2.5, 3.5], [0.0, 1.0], [-0.1, 0.1]),
        slab([2.5, 3.5], [2.0, 4.0], [-0.1, 0.1]),
        slab([2.5, 3.5], [5.0, 6.0], [-0.1, 0.1]),
        // East wall (x = 6): door hole z ∈ [2.5, 3.5] below 2.1.
        slab([5.9, 6.1], [0.0, 6.0], [0.0, 2.5]),
        slab([5.9, 6.1], [0.0, 6.0], [3.5, 6.0]),
        slab([5.9, 6.1], [2.1, 6.0], [2.5, 3.5]),
        // North + west walls, solid.
        slab([0.0, 6.0], [0.0, 6.0], [5.9, 6.1]),
        slab([-0.1, 0.1], [0.0, 6.0], [0.0, 6.0]),
        // Flat roof slab.
        slab([-0.1, 6.1], [6.0, 6.2], [-0.1, 6.1]),
    ];
    scenes.extend_from_slice(extra);
    let (verts, tris) = concat(&scenes);
    let bvh = Bvh::build(&verts, &tris);
    BvhSidecar { verts, tris, bvh }
}

fn room() -> (BuildingBlueprint, BvhSidecar) {
    (room_blueprint(), room_sidecar(&[]))
}

#[test]
fn blocked_wall_is_named() {
    let (bp, sc) = room();
    let los = bp.evaluate_los(&sc, [1.0, 1.5, -3.0], [1.0, 1.5, 3.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some("w_s0".to_string()));
    assert!((los.concealment - 1.0).abs() < f64::EPSILON);
    // The terminal block is the LAST hit; nothing is recorded past it.
    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Wall);
    assert_eq!(last.id, "w_s0");
    assert!((last.concealment - 1.0).abs() < f64::EPSILON);
    // Stopped on the slab's outer FACE (z = -0.1), not the blueprint centerline.
    assert!((last.pos[2] - -0.1).abs() < 1e-9, "pos = {:?}", last.pos);
    assert!((last.t - 2.9 / 6.0).abs() < 1e-9, "t = {}", last.t);
    assert_eq!(los.hits.len(), 1, "hits: {:?}", los.hits);
}

#[test]
fn window_is_named_and_traversed_through_mesh_hole() {
    let (bp, sc) = room();
    // Straight through the ground-floor window hole at sill + 0.5.
    let los = bp.evaluate_los(&sc, [3.0, 1.5, -3.0], [3.0, 1.5, 3.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(los.window_ids_traversed, vec!["win_s0".to_string()]);
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

    // Through the window and on into the far wall: the traversal is recorded, then the mesh
    // stops the ray on the north wall's inner face.
    let los = bp.evaluate_los(&sc, [3.0, 1.5, -3.0], [3.0, 1.5, 7.0]);
    assert!(!los.is_clear);
    assert_eq!(los.window_ids_traversed, vec!["win_s0".to_string()]);
    assert_eq!(los.blocked_by_wall_id, Some("w_n0".to_string()));
    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Wall);
    assert!((last.pos[2] - 5.9).abs() < 1e-9, "pos = {:?}", last.pos);
    assert_eq!(los.hits.len(), 2);
}

#[test]
fn door_is_traversed() {
    let (bp, sc) = room();
    let los = bp.evaluate_los(&sc, [3.0, 1.0, 3.0], [9.0, 1.0, 3.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(los.door_ids_traversed, vec!["door_e0".to_string()]);
    assert!(los.window_ids_traversed.is_empty());
    let door = los.hits.last().expect("door annotation");
    assert_eq!(door.kind, LosHitKind::DoorOpen);
    assert!((door.pos[0] - 6.0).abs() < 1e-9);
    // Above the door head the same crossing is lintel: the mesh stops it, blamed on the wall.
    let los = bp.evaluate_los(&sc, [3.0, 2.5, 3.0], [9.0, 2.5, 3.0]);
    assert!(!los.is_clear);
    assert!(los.door_ids_traversed.is_empty());
    assert_eq!(los.blocked_by_wall_id, Some("w_e0".to_string()));
}

/// The multi-band regression: a ray climbing from ground level outside to the upper floor
/// crosses the south wall FACE while in the level-1 band (t = 0.65, y = 3.6), so the blocker
/// must be `w_s1`, not the ground floor's `w_s0` — and no phantom level-0 events.
#[test]
fn cross_band_blames_correct_floors_wall() {
    let (bp, sc) = room();
    let los = bp.evaluate_los(&sc, [1.0, 1.0, -4.0], [1.0, 5.0, 2.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some("w_s1".to_string()));
    assert!(
        los.hits.iter().all(|h| h.id != "w_s0"),
        "hits: {:?}",
        los.hits
    );
    let last = los.hits.last().expect("terminal hit");
    assert!((last.t - 0.65).abs() < 1e-9, "t = {}", last.t);
    assert!((last.pos[1] - 3.6).abs() < 1e-9, "y = {}", last.pos[1]);
}

/// Mesh the blueprint never described (an interior pillar): the ray still stops — the mesh is
/// the structure — and the hit is reported as plain `Solid` with no wall to blame.
#[test]
fn solid_when_blueprint_silent() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[cube([3.0, 1.5, 3.0], [0.3, 1.5, 0.3])]);
    let los = bp.evaluate_los(&sc, [1.0, 1.5, 3.0], [5.0, 1.5, 3.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, None);
    assert_eq!(los.cover_furniture_id, None);
    assert!((los.concealment - 1.0).abs() < f64::EPSILON);
    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Solid);
    assert_eq!(last.id, "solid");
    assert!((last.pos[0] - 2.7).abs() < 1e-9, "pos = {:?}", last.pos);
    assert_eq!(los.hits.len(), 1);
    // Without the pillar the same ray is open: nothing in the mesh, nothing in the blueprint.
    let (bp, sc) = room();
    let los = bp.evaluate_los(&sc, [1.0, 1.5, 3.0], [5.0, 1.5, 3.0]);
    assert!(los.is_clear && los.hits.is_empty(), "hits: {:?}", los.hits);
}

/// A structural hit on the roof slab, above every level band and within `ROOF_ATTR_TOL_M` of
/// the heightfield, is attributed to the roof; with the grid shape-invalid it is not trusted
/// and the same hit falls through to `Solid` — the verdict is identical either way.
#[test]
fn roof_attribution_near_surface() {
    let (bp, sc) = room();
    let los = bp.evaluate_los(&sc, [3.0, 8.0, 3.0], [3.0, 4.5, 3.0]);
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
    let los = broken.evaluate_los(&sc, [3.0, 8.0, 3.0], [3.0, 4.5, 3.0]);
    assert!(!los.is_clear);
    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Solid);
    assert!((last.pos[1] - 6.2).abs() < 1e-9);
}

fn table(los_cover: &str) -> BuildingFurniture {
    BuildingFurniture {
        id: "furn_table_01".to_string(),
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

/// Low-cover furniture conceals without blocking. Props are world siblings, absent from the
/// COLL mesh — the annotation is the only record of them.
#[test]
fn furniture_low_cover_conceals_without_blocking() {
    let mut bp = room_blueprint();
    bp.levels[0].furniture.push(table("low_cover"));
    let sc = room_sidecar(&[]);
    // Enters the table's AABB at y ≈ 0.56 (below the 0.78 top): concealed, still clear.
    let los = bp.evaluate_los(&sc, [3.0, 0.6, 4.0], [5.0, 0.4, 4.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(los.cover_furniture_id, Some("furn_table_01".to_string()));
    assert!((los.concealment - 0.60).abs() < f64::EPSILON);
    assert!(
        los.hits
            .iter()
            .any(|h| h.kind == LosHitKind::Furniture && h.id == "furn_table_01")
    );
    // Over the top of the table: no cover at all.
    let los = bp.evaluate_los(&sc, [3.0, 1.0, 4.0], [5.0, 1.0, 4.0]);
    assert!(los.is_clear && los.hits.is_empty(), "hits: {:?}", los.hits);
}

/// `full_cover` is the one annotation that terminates — with NO mesh hit anywhere on the ray.
#[test]
fn furniture_full_cover_is_terminal_without_mesh_hit() {
    let mut bp = room_blueprint();
    bp.levels[0].furniture.push(table("full_cover"));
    let sc = room_sidecar(&[]);
    let los = bp.evaluate_los(&sc, [3.0, 0.6, 4.0], [5.0, 0.4, 4.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, None);
    assert_eq!(los.cover_furniture_id, Some("furn_table_01".to_string()));
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
        id: "stairs_01".to_string(),
        bounds: [[1.0, 1.0], [2.0, 2.0]],
        connects_to_level: 1,
        direction_deg: 0.0,
        step_count: 10,
        transparent_steps: true,
        los_concealment: 0.3,
    });
    let sc = room_sidecar(&[]);
    let los = bp.evaluate_los(&sc, [0.5, 1.0, 1.5], [3.0, 1.0, 1.5]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert!((los.concealment - 0.3).abs() < f64::EPSILON);
    assert_eq!(los.hits.len(), 1);
    assert_eq!(los.hits[0].kind, LosHitKind::Stairs);
    assert_eq!(los.hits[0].id, "stairs_01");
    assert!(
        (los.hits[0].pos[0] - 1.0).abs() < 1e-9,
        "entry at the AABB edge"
    );
    // Solid treads are not an annotation (the mesh would carry them).
    bp.levels[0].stairs[0].transparent_steps = false;
    let los = bp.evaluate_los(&sc, [0.5, 1.0, 1.5], [3.0, 1.0, 1.5]);
    assert!(los.is_clear && los.hits.is_empty());
}

/// A mullion inside the window hole: the ray stops on frame mass INSIDE the aperture rect. The
/// structural hit is attributed to the window (so the viewer can say which), is terminal, and
/// the window is NOT listed as traversed — the annotation on the centerline behind it is never
/// reached.
#[test]
fn terminal_hit_inside_aperture_is_not_traversed() {
    let bp = room_blueprint();
    let sc = room_sidecar(&[cube([3.0, 1.5, 0.0], [0.05, 0.5, 0.1])]);
    let los = bp.evaluate_los(&sc, [3.0, 1.5, -3.0], [3.0, 1.5, 3.0]);
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

/// Roofless blueprints (the hand-authored pre-scan JSONs on map-assets) parse with `roof:
/// None` and the absent field round-trips away; a populated grid answers lookups and a
/// shape-invalid one reports itself.
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

/// Plate contract: pre-plate JSONs parse to `None`/empty and the absent fields round-trip
/// away; populated plate + rings survive a serde round trip; shape checks work.
#[test]
fn plate_grid_and_floor_polygons_round_trip() {
    // Back-compat: the committed pre-plate asset has neither field.
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

    // Round trip with both populated.
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

/// The upstairs window (level 1, y ∈ [4, 5]) is traversed both by a flat ray on that floor and
/// by a ray that climbs into it from the level-0 band outside — the level-1 clip owns the
/// crossing, and no ground-floor annotation appears.
#[test]
fn upstairs_window_is_traversed_flat_and_cross_band() {
    let (bp, sc) = room();
    let los = bp.evaluate_los(&sc, [3.0, 4.5, -3.0], [3.0, 4.5, 3.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(los.window_ids_traversed, vec!["win_s1".to_string()]);

    // Climbs 3.0 → 5.0 over z -6 → 2: crosses the south face at y ≈ 4.48, inside the hole.
    let los = bp.evaluate_los(&sc, [3.0, 3.0, -6.0], [3.0, 5.0, 2.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(los.window_ids_traversed, vec!["win_s1".to_string()]);
    assert!(
        los.hits.iter().all(|h| !h.id.ends_with('0')),
        "no level-0 events"
    );
    // One floor lower the same climb meets the slab between the two holes: blocked, `w_s1`
    // (the crossing is at y ≈ 3.48, in the level-1 band, below its sill).
    let los = bp.evaluate_los(&sc, [3.0, 2.0, -6.0], [3.0, 4.0, 2.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some("w_s1".to_string()));
    assert!(los.window_ids_traversed.is_empty());
}
