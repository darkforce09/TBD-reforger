use super::*;
use interior_line_of_sight::floor_wash::WashParams;
use interior_line_of_sight::floor_wash::level_washes;
use spatial_indexes::bounding_volume_hierarchy::sidecar::BvhSidecar;

fn farmhouse() -> BuildingBlueprint {
    serde_json::from_str(frontend_test_support::repository_root::repository_text(
        env!("CARGO_MANIFEST_DIR"),
        "assets/terrains/everon/prefabs/buildings/FarmHouse_E_1L01.json",
    ))
    .expect("golden parses")
}

#[test]
fn world_mapping_round_trips_and_points_north_up() {
    let p = [-3.8, -4.5];
    let w = to_world(p);
    let back = from_world(w);
    // The ±6400 anchor shift costs a few low bits — sub-micrometer, irrelevant on screen.
    assert!((back[0] - p[0]).abs() < 1e-9 && (back[1] - p[1]).abs() < 1e-9);
    // The engine renders world +y UP (deck.gl flipY:false — pinned by the A.1 screenshot
    // mirror bug), so north (+z) must INCREASE world y…
    assert!(to_world([0.0, 5.0])[1] > to_world([0.0, 0.0])[1]);
    // …and larger world y must land HIGHER on screen (smaller CSS y).
    let css = (1280.0, 800.0);
    let hi = world_to_screen(to_world([0.0, 5.0]), 6400.0, 6400.0, 4.0, css);
    let lo = world_to_screen(to_world([0.0, 0.0]), 6400.0, 6400.0, 4.0, css);
    assert!(hi[1] < lo[1]);
}

#[test]
fn screen_projection_round_trips() {
    let css = (1280.0, 800.0);
    let (tx, ty, zoom) = (6400.0, 6400.0, 4.2);
    let w = [6403.3, 6396.1];
    let s = world_to_screen(w, tx, ty, zoom, css);
    let back = screen_to_world(s, tx, ty, zoom, css);
    assert!((back[0] - w[0]).abs() < 1e-9 && (back[1] - w[1]).abs() < 1e-9);
}

#[test]
fn fit_camera_centers_the_farmhouse() {
    let bp = farmhouse();
    let (tx, ty, zoom) = fit_camera(&bp, (1280.0, 800.0));
    // Camera target = world-mapped center of overall bbox (the fit_camera contract).
    // v7 note: bounding_box2_d is the ENTITY bounds from the dump meta, not the
    // polygon's extremes — the two legitimately differ, so no cross-check here.
    let bb = &bp.overall_footprint.bounding_box2_d;
    let c = to_world([(bb.min[0] + bb.max[0]) * 0.5, (bb.min[1] + bb.max[1]) * 0.5]);
    assert!((tx - c[0]).abs() < 1e-9, "tx {tx} vs {}", c[0]);
    assert!((ty - c[1]).abs() < 1e-9, "ty {ty} vs {}", c[1]);
    // A farmhouse-sized footprint × 1.25 margin across 1280 px sits inside the zoom clamp.
    assert!(zoom > 4.0 && zoom <= 6.0, "zoom {zoom}");
}

#[test]
fn point_in_polygon_l_shape() {
    let bp = farmhouse();
    let ring = &bp.overall_footprint.polygon2_d;
    assert!(point_in_polygon([0.0, 0.0], ring));
    assert!(point_in_polygon([-3.0, 4.0], ring)); // west wing
    assert!(!point_in_polygon([5.0, 4.0], ring)); // the L notch
    assert!(!point_in_polygon([0.0, -6.0], ring)); // outside south
}

/// A third (attic) level slots into the band math: its own band is not the topmost,
/// and the Roof band shifts above it.
#[test]
fn attic_third_level_band_math() {
    let mut bp = farmhouse();
    let mut attic = bp.levels[1].clone();
    attic.level_index = 2;
    attic.name = "Attic".to_string();
    attic.elevation_range = [5.6, 7.0];
    attic.footprint_polygon = Vec::new();
    attic.walls.clear();
    attic.windows.clear();
    bp.levels.push(attic);
    let (band, last) = ViewFloor::Level(2).band(&bp);
    assert_eq!(band, [5.6, 7.0]);
    assert!(!last, "roof still owns the space above the attic");
    let (roof_band, roof_last) = ViewFloor::Roof.band(&bp);
    assert_eq!(roof_band, [7.0, 7.8]);
    assert!(roof_last);
}

/// Axis-aligned cuboid as 12 triangles — the same quad table as
/// `spatial_indexes::test_fixtures::cube` (behind that crate's `test_fixtures` feature).
fn cube(center: [f64; 3], half: [f64; 3]) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
    let mut verts = Vec::new();
    for corner in 0..8u32 {
        verts.push([
            center[0] + if corner & 1 != 0 { half[0] } else { -half[0] },
            center[1] + if corner & 2 != 0 { half[1] } else { -half[1] },
            center[2] + if corner & 4 != 0 { half[2] } else { -half[2] },
        ]);
    }
    const QUADS: [[u32; 4]; 6] = [
        [0, 4, 6, 2],
        [1, 3, 7, 5],
        [0, 1, 5, 4],
        [2, 6, 7, 3],
        [0, 2, 3, 1],
        [4, 5, 7, 6],
    ];
    let mut tris = Vec::new();
    for q in QUADS {
        tris.push([q[0], q[1], q[2]]);
        tris.push([q[0], q[2], q[3]]);
    }
    (verts, tris)
}

/// One-level 10 × 10 m box room (band [0, 3]) with a single window hole in the south
/// wall (x ∈ [-1, 1], y ∈ [1, 2]): the blueprint names it, the 0.2 m slab mesh HAS it.
fn box_room() -> (BuildingBlueprint, BvhSidecar) {
    use building_interiors::blueprint::footprint::OverallFootprint;
    use building_interiors::blueprint::footprint::VerticalProfile;
    use building_interiors::blueprint::structure::BBox2D;
    use building_interiors::blueprint::structure::BuildingWall;
    use building_interiors::blueprint::structure::BuildingWindow;
    use spatial_indexes::bounding_volume_hierarchy::triangle_tree::Bvh;
    let wall = |id: &str, start: [f64; 2], end: [f64; 2]| BuildingWall {
        id: id.into(),
        start,
        end,
        thickness: 0.2,
        is_exterior: true,
        material: "synthetic".into(),
    };
    let square = vec![[-5.0, -5.0], [5.0, -5.0], [5.0, 5.0], [-5.0, 5.0]];
    let bp = BuildingBlueprint {
        schema_version: "1.0.0".into(),
        prefab_id: "BoxRoom".into(),
        resource_name: "synthetic://box".into(),
        model_mesh: None,
        label: None,
        kind: "building".into(),
        category: "generic".into(),
        destructible: false,
        vertical_profile: VerticalProfile {
            pivot_elevation_offset_m: 0.0,
            foundation_skirt_depth_m: 0.0,
            total_height_m: 3.0,
            eave_height_m: 3.0,
            ridge_height_m: 3.0,
            chimney_height_m: None,
            roof_type: "flat".into(),
        },
        overall_footprint: OverallFootprint {
            polygon2_d: square.clone(),
            bounding_box2_d: BBox2D {
                min: [-5.0, -5.0],
                max: [5.0, 5.0],
                width_m: 10.0,
                depth_m: 10.0,
            },
            footprint_sq_m: 100.0,
        },
        roof: None,
        levels: vec![BuildingLevel {
            level_index: 0,
            name: "ground".into(),
            elevation_range: [0.0, 3.0],
            slice_height_m: 1.5,
            footprint_polygon: square,
            plate: None,
            floor_polygons: vec![],
            walls: vec![
                wall("w_s", [-5.0, -5.0], [5.0, -5.0]),
                wall("w_n", [-5.0, 5.0], [5.0, 5.0]),
                wall("w_w", [-5.0, -5.0], [-5.0, 5.0]),
                wall("w_e", [5.0, -5.0], [5.0, 5.0]),
            ],
            doors: vec![],
            windows: vec![BuildingWindow {
                id: "win_s".into(),
                prefab_resource: "synthetic://window".into(),
                wall_id: "w_s".into(),
                pos2_d: [0.0, -5.0],
                width_m: 2.0,
                sill_height_m: 1.0,
                window_height_m: 1.0,
                normal: [0.0, -1.0],
                fov_deg: 120.0,
                has_glass: true,
                glass_pane_count: 1,
            }],
            stairs: vec![],
            furniture: vec![],
        }],
    };
    let slab = |x: [f64; 2], y: [f64; 2], z: [f64; 2]| {
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
    };
    let pieces = [
        // South wall around the hole.
        slab([-5.0, -1.0], [0.0, 3.0], [-5.1, -4.9]),
        slab([1.0, 5.0], [0.0, 3.0], [-5.1, -4.9]),
        slab([-1.0, 1.0], [0.0, 1.0], [-5.1, -4.9]),
        slab([-1.0, 1.0], [2.0, 3.0], [-5.1, -4.9]),
        // North, west, east: solid.
        slab([-5.0, 5.0], [0.0, 3.0], [4.9, 5.1]),
        slab([-5.1, -4.9], [0.0, 3.0], [-5.0, 5.0]),
        slab([4.9, 5.1], [0.0, 3.0], [-5.0, 5.0]),
    ];
    let mut verts = Vec::new();
    let mut tris = Vec::new();
    for (v, t) in pieces {
        let base = verts.len() as u32;
        verts.extend_from_slice(&v);
        tris.extend(
            t.iter()
                .map(|tri| [tri[0] + base, tri[1] + base, tri[2] + base]),
        );
    }
    let bvh = Bvh::build(&verts, &tris);
    (bp, BvhSidecar::opaque(verts, tris, bvh))
}

/// The encoder's contract with the engine lane: GREEN per visible cell, nothing for
/// hidden / unknown, rows straight through (row 0 = north = texture row 0), rows
/// padded to a 256-byte stride, and the world rect from the local plan rect.
#[test]
fn wash_texture_maps_green_visible_and_world_rect() {
    let mut cells = vec![Visibility::Visible; 6];
    cells[1] = Visibility::Hidden; // row 0, col 1
    cells[2] = Visibility::Unknown; // row 0, col 2
    let w = LevelWash {
        level_index: 0,
        eye_y: 1.0,
        obs: [0.0; 3],
        radius_m: 10.0,
        min_x: -1.0,
        min_z: -2.0,
        max_x: 2.0,
        max_z: 0.0,
        cell_m: 1.0,
        cols: 3,
        rows: 2,
        cells,
    };
    let t = wash_texture(&w);
    assert_eq!((t.tex_w, t.tex_h, t.stride_bytes), (3, 2, 256));
    assert_eq!(t.rgba.len(), 512);
    assert_eq!(&t.rgba[0..4], &WASH_VISIBLE_RGBA);
    assert_eq!(&t.rgba[4..8], &WASH_CLEAR_RGBA, "hidden is not inked");
    assert_eq!(
        &t.rgba[8..12],
        &WASH_CLEAR_RGBA,
        "beyond the disc is not inked"
    );
    assert_eq!(
        &t.rgba[256..260],
        &WASH_VISIBLE_RGBA,
        "row 1 starts at the stride"
    );
    assert!((t.min_x - (ANCHOR[0] - 1.0)).abs() < 1e-9);
    assert!((t.min_y - (ANCHOR[1] - 2.0)).abs() < 1e-9);
    assert!((t.max_x - (ANCHOR[0] + 2.0)).abs() < 1e-9);
    assert!((t.max_y - ANCHOR[1]).abs() < 1e-9);
}

/// The raster on the one-level box room: every interior cell lit, the exterior lit
/// ONLY in the cone the south window hole subtends, walls dark — the per-cell rays
/// cannot leak the way the retired centre-fan did.
#[test]
fn wash_escapes_only_through_the_window() {
    let (bp, sc) = box_room();
    // Observer mid-room at standing eye height, inside the window's sill..top band.
    let obs = [0.0, 1.4, 0.0];
    // The viewer's radius: the footprint diagonal + 5 m (the old fan's range).
    let p = WashParams {
        radius_m: 10f64.hypot(10.0) + 5.0,
        ..WashParams::default()
    };
    let washes = level_washes(&bp, &sc, obs, &p);
    assert_eq!(washes.len(), 1);
    let w = &washes[0];
    assert_eq!(
        w.visibility_at(15.0, -15.0),
        Visibility::Unknown,
        "beyond the disc, inside the square"
    );
    let mut south_lit = 0usize;
    for row in 0..w.rows {
        for col in 0..w.cols {
            let c = w.cell_center(col, row);
            let v = w.at(col, row);
            if c[0].abs() < 4.5 && c[1].abs() < 4.5 {
                assert_eq!(v, Visibility::Visible, "interior cell {c:?}");
            }
            if c[1] < -5.1 && v == Visibility::Visible {
                south_lit += 1;
            }
        }
    }
    // The hole subtends ±atan(1 / 4.9) ≈ ±11.5°: a cone 2–8 m wide over the ~14 m of
    // disc south of the wall, on the order of a thousand 0.25 m cells — never the
    // whole exterior.
    assert!(
        south_lit > 100 && south_lit < 2500,
        "south cone: {south_lit} cells lit"
    );
    assert_eq!(
        w.visibility_at(0.0, -7.0),
        Visibility::Visible,
        "dead ahead through the window"
    );
    assert_eq!(
        w.visibility_at(3.0, -7.0),
        Visibility::Hidden,
        "beside the window"
    );
    assert_eq!(w.visibility_at(0.0, 7.0), Visibility::Hidden, "north wall");
}

#[test]
fn rect_corners_rotation_preserves_area_orientation() {
    let c = rect_corners([1.0, 2.0], [2.0, 1.0], 90.0);
    // 90° rotation swaps extents around the center.
    let xs: Vec<f64> = c.iter().map(|p| p[0]).collect();
    let zs: Vec<f64> = c.iter().map(|p| p[1]).collect();
    let (w, d) = (
        xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min),
        zs.iter().cloned().fold(f64::MIN, f64::max) - zs.iter().cloned().fold(f64::MAX, f64::min),
    );
    assert!((w - 1.0).abs() < 1e-9 && (d - 2.0).abs() < 1e-9);
}
