//! The synthetic rooms the line-of-sight tests of this crate and of the map engine build on.
//!
//! **Role:** builds a two-level 6 × 6 m box room as a blueprint ([`room_blueprint`]) and as the
//! occlusion trimesh that matches it ([`room_sidecar`]), and the axis-aligned slabs both are made
//! of ([`slab`]).
//! **Position:** compiled for this crate's tests and, through the `test_fixtures` feature, for the
//! tests of crates that list this crate as a dev-dependency (the map engine's interior line of
//! sight and visibility wash); built on `spatial_indexes`' cuboid scenes.
//! **Signals & state:** none; every call builds fresh values.
//! **Invariants:** the sidecar's walls lie on the blueprint's wall centrelines with the window and
//! door holes cut at the blueprint's aperture rectangles, so an attribution test's mesh hit and
//! blueprint feature agree.

use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;
use spatial_indexes::bounding_volume_hierarchy::triangle_tree::Bvh;
use spatial_indexes::test_fixtures::{Scene, concat, cube};
use world_file_formats::ids::{DoorId, WallId, WindowId};

use crate::blueprint::footprint::{OverallFootprint, RoofGrid, VerticalProfile};
use crate::blueprint::structure::{
    BBox2D, BuildingBlueprint, BuildingDoor, BuildingLevel, BuildingWall, BuildingWindow,
};
use crate::building_ids::BuildingPrefabId;

/// Axis-aligned slab from absolute extents `[x0, x1] × [y0, y1] × [z0, z1]`.
pub fn slab(x: [f64; 2], y: [f64; 2], z: [f64; 2]) -> Scene {
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
        id: WallId::new(id),
        start,
        end,
        thickness: 0.2,
        is_exterior: true,
        material: "synthetic".to_string(),
    }
}

fn window(id: &str, wall_id: &str, pos2_d: [f64; 2]) -> BuildingWindow {
    BuildingWindow {
        id: WindowId::new(id),
        prefab_resource: "synthetic://window".to_string(),
        wall_id: WallId::new(wall_id),
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

/// Two-level 6 × 6 m box room, bands [0, 3) and [3, 6], flat roof at 6.0: one window per floor on the south wall (x ∈ [2.5, 3.5], y ∈ [1, 2] / [4, 5]), an open door in the ground-floor east wall (z ∈ [2.5, 3.5], y < 2.1).
pub fn room_blueprint() -> BuildingBlueprint {
    let square = vec![[0.0, 0.0], [6.0, 0.0], [6.0, 6.0], [0.0, 6.0]];
    let mut ground = level(0, [0.0, 3.0], "0");
    ground.doors.push(BuildingDoor {
        id: DoorId::new("door_e0"),
        prefab_resource: "synthetic://door".to_string(),
        wall_id: WallId::new("w_e0"),
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
        prefab_id: BuildingPrefabId::new("SyntheticRoom"),
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

/// The COLL-style trimesh matching [`room_blueprint`]: 0.2 m slabs on the wall centerlines with the window / door holes cut, plus the roof slab. `extra` scenes are appended verbatim (a pillar the blueprint does not know about, a mullion inside a window hole).
pub fn room_sidecar(extra: &[Scene]) -> BvhSidecar {
    let mut scenes = vec![
        slab([0.0, 2.5], [0.0, 6.0], [-0.1, 0.1]),
        slab([3.5, 6.0], [0.0, 6.0], [-0.1, 0.1]),
        slab([2.5, 3.5], [0.0, 1.0], [-0.1, 0.1]),
        slab([2.5, 3.5], [2.0, 4.0], [-0.1, 0.1]),
        slab([2.5, 3.5], [5.0, 6.0], [-0.1, 0.1]),
        slab([5.9, 6.1], [0.0, 6.0], [0.0, 2.5]),
        slab([5.9, 6.1], [0.0, 6.0], [3.5, 6.0]),
        slab([5.9, 6.1], [2.1, 6.0], [2.5, 3.5]),
        slab([0.0, 6.0], [0.0, 6.0], [5.9, 6.1]),
        slab([-0.1, 0.1], [0.0, 6.0], [0.0, 6.0]),
        slab([-0.1, 6.1], [6.0, 6.2], [-0.1, 6.1]),
    ];
    scenes.extend_from_slice(extra);
    let (verts, tris) = concat(&scenes);
    let bvh = Bvh::build(&verts, &tris);
    BvhSidecar::opaque(verts, tris, bvh)
}
