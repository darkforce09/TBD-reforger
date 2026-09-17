use super::*;
use website_map_engine::spatial::bvh::sidecar::BvhSidecar;
use website_map_engine::spatial::bvh::traversal::Bvh;
use website_map_engine::spatial::los::interior::wash::level_washes;
use website_map_engine::spatial::los::interior::wash::WashParams;
use website_map_engine::world::architecture::section::cutter::building_drawing;

fn farmhouse() -> BuildingBlueprint {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01.json"
    )))
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

#[test]
fn static_lanes_cover_every_feature_of_the_active_floor() {
    let bp = farmhouse();
    let l0 = build_static_lanes(&bp, None, ViewFloor::Level(0));
    assert_eq!(l0.wall_count, 7);
    assert_eq!(l0.aperture_count, 3 + 2); // windows + doors
    assert!(!l0.floor_pos.is_empty() && !l0.floor_idx.is_empty());
    assert_eq!(l0.floor_col.len() / 4, l0.floor_pos.len() / 2);
    // Furniture lanes mirror the data: the v7 extract carries none for the FarmHouse
    // (dump meta furniture: 0), and the lane must stay empty exactly when the level is.
    assert_eq!(l0.furn_pos.is_empty(), bp.levels[0].furniture.is_empty());
    // Ghost centerlines for the OTHER floor are present (4 upstairs walls).
    assert!(l0.hairline_count >= 4);
    let l1 = build_static_lanes(&bp, None, ViewFloor::Level(1));
    assert_eq!(l1.wall_count, 4);
    assert_eq!(l1.aperture_count, 2);
}

#[test]
fn roof_view_is_footprint_plus_all_ghost_walls() {
    let bp = farmhouse();
    let roof = build_static_lanes(&bp, None, ViewFloor::Roof);
    assert_eq!(roof.wall_count, 0);
    assert_eq!(roof.aperture_count, 0);
    assert!(roof.furn_pos.is_empty());
    assert!(!roof.floor_pos.is_empty());
    // Roofless fixture: no heightfield cells painted.
    assert_eq!(roof.roof_cell_count, 0);
    // Ghosts: 7 ground + 4 upstairs centerlines.
    assert_eq!(roof.hairline_count, 11);
    // Roof band sits above the top level and reaches the ridge-carrying total height.
    let (band, last) = ViewFloor::Roof.band(&bp);
    assert_eq!(band, [5.6, 7.8]);
    assert!(last);
}

/// Level views paint the PlateGrid verbatim: one quad per covered cell replacing the
/// polygon fill (a partial mezzanine's void stays unpainted); nulls skip; stairs
/// plates still land on the floor lane.
#[test]
fn level_view_paints_plate_grid_verbatim() {
    use website_map_engine::world::architecture::blueprint::footprint::PlateGrid;
    let mut bp = farmhouse();
    bp.levels[0].plate = Some(PlateGrid {
        origin: [-2.0, -2.0],
        cell_size_m: 0.5,
        nx: 4,
        nz: 4,
        heights_m: (0..16).map(|i| (i % 4 != 0).then_some(0.0)).collect(),
    });
    let lanes = build_static_lanes(&bp, None, ViewFloor::Level(0));
    let covered = (0..16).filter(|i| i % 4 != 0).count();
    assert_eq!(lanes.plate_cell_count, covered as u32);
    // Floor lane = plate quads (4 verts each) + the one stairs plate (4 verts);
    // the footprint-polygon fill is REPLACED, not underlaid.
    assert_eq!(lanes.floor_pos.len(), covered * 8 + 8);
}

/// Pre-plate assets keep the traced-polygon fill path byte-for-byte.
#[test]
fn plate_none_falls_back_to_polygon() {
    let bp = farmhouse();
    let lanes = build_static_lanes(&bp, None, ViewFloor::Level(0));
    assert_eq!(lanes.plate_cell_count, 0);
    assert!(!lanes.floor_pos.is_empty());
}

/// floorPolygons rings (outer + holes) draw as closed hairline loops over the plate.
#[test]
fn floor_rings_draw_closed_hairline_loops() {
    use website_map_engine::world::architecture::blueprint::structure::FloorPolygon;
    let bp = farmhouse();
    let base = build_static_lanes(&bp, None, ViewFloor::Level(0));
    let mut bp = farmhouse();
    bp.levels[0].floor_polygons = vec![FloorPolygon {
        outer: vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]],
        holes: vec![vec![[0.5, 0.5], [0.5, 1.0], [1.0, 1.0], [1.0, 0.5]]],
    }];
    let lanes = build_static_lanes(&bp, None, ViewFloor::Level(0));
    // 4 outer edges + 4 hole edges, closed loops.
    assert_eq!(lanes.hairline_count, base.hairline_count + 8);
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

/// The Roof view paints the emitted RoofGrid verbatim: one tinted quad per covered cell
/// on the floor lane, ramping dark→light with height; nulls skip; ghosts unaffected.
#[test]
fn roof_view_paints_the_heightfield() {
    use website_map_engine::world::architecture::blueprint::footprint::RoofGrid;
    let mut bp = farmhouse();
    let base = build_static_lanes(&bp, None, ViewFloor::Roof);
    bp.roof = Some(RoofGrid {
        origin: [-2.0, -2.0],
        cell_size_m: 1.0,
        nx: 4,
        nz: 4,
        // Every 3rd cell null; the rest climb 3.0 … 6.0.
        heights_m: (0..16)
            .map(|i| (i % 3 != 0).then(|| 3.0 + f64::from(i) * 0.2))
            .collect(),
    });
    let lanes = build_static_lanes(&bp, None, ViewFloor::Roof);
    let covered = (0..16).filter(|i| i % 3 != 0).count();
    assert_eq!(lanes.roof_cell_count, covered as u32);
    // One 4-vertex rect (8 floats) per covered cell on top of the plate mesh.
    assert_eq!(lanes.floor_pos.len(), base.floor_pos.len() + covered * 8);
    // The ramp actually ramps: the cell colors are not all identical.
    let cell_cols: std::collections::HashSet<[u32; 4]> = lanes.floor_col[base.floor_col.len()..]
        .chunks_exact(4)
        .map(|c| {
            [
                c[0].to_bits(),
                c[1].to_bits(),
                c[2].to_bits(),
                c[3].to_bits(),
            ]
        })
        .collect();
    assert!(cell_cols.len() >= 3, "distinct tints: {}", cell_cols.len());
    // Ghost walls unchanged by the paint.
    assert_eq!(lanes.hairline_count, base.hairline_count);
}

const GROUND_BAND: [f64; 2] = [0.0, 2.8];

/// Axis-aligned cuboid as 12 triangles — the same quad table as `map_engine_core`'s
/// `bvh_tests::cube` (crate-private there).
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
    use website_map_engine::spatial::bvh::traversal::Bvh;
    use website_map_engine::world::architecture::blueprint::footprint::OverallFootprint;
    use website_map_engine::world::architecture::blueprint::footprint::VerticalProfile;
    use website_map_engine::world::architecture::blueprint::structure::BBox2D;
    use website_map_engine::world::architecture::blueprint::structure::BuildingWall;
    use website_map_engine::world::architecture::blueprint::structure::BuildingWindow;
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

/// Append axis-aligned slabs (absolute extents) to a scene and rebuild its BVH.
fn with_slabs(sc: BvhSidecar, slabs: &[([f64; 2], [f64; 2], [f64; 2])]) -> BvhSidecar {
    let (mut verts, mut tris) = (sc.verts, sc.tris);
    for &(x, y, z) in slabs {
        let (v, t) = cube(
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
        );
        let base = verts.len() as u32;
        verts.extend_from_slice(&v);
        tris.extend(
            t.iter()
                .map(|tri| [tri[0] + base, tri[1] + base, tri[2] + base]),
        );
    }
    let bvh = Bvh::build(&verts, &tris);
    BvhSidecar::opaque(verts, tris, bvh)
}

/// [`box_room`] plus an upper level [3, 6]: upper walls, a ceiling slab y ∈ [2.9, 3.1]
/// with a stairwell hole x, z ∈ [0.5, 1.5], an interior ground-floor wall x ∈ [0.9, 1.1]
/// running under the hole, and (optionally) a roof slab y ∈ [6, 6.2].
fn box_room_two_level(with_roof: bool) -> (BuildingBlueprint, BvhSidecar) {
    let (mut bp, sc) = box_room();
    let mut upper = bp.levels[0].clone();
    upper.level_index = 1;
    upper.name = "upper".into();
    upper.elevation_range = [3.0, 6.0];
    upper.windows.clear();
    bp.levels.push(upper);
    bp.vertical_profile.total_height_m = if with_roof { 6.2 } else { 6.0 };
    let mut slabs = vec![
        // Upper walls (solid).
        ([-5.0, 5.0], [3.0, 6.0], [-5.1, -4.9]),
        ([-5.0, 5.0], [3.0, 6.0], [4.9, 5.1]),
        ([-5.1, -4.9], [3.0, 6.0], [-5.0, 5.0]),
        ([4.9, 5.1], [3.0, 6.0], [-5.0, 5.0]),
        // Ceiling / upper floor slab around the stairwell hole.
        ([-5.0, 5.0], [2.9, 3.1], [-5.0, 0.5]),
        ([-5.0, 5.0], [2.9, 3.1], [1.5, 5.0]),
        ([-5.0, 0.5], [2.9, 3.1], [0.5, 1.5]),
        ([1.5, 5.0], [2.9, 3.1], [0.5, 1.5]),
        // Interior ground-floor wall running under the hole.
        ([0.9, 1.1], [0.0, 2.0], [-2.0, 3.0]),
    ];
    if with_roof {
        slabs.push(([-5.1, 5.1], [6.0, 6.2], [-5.1, 5.1]));
    }
    (bp, with_slabs(sc, &slabs))
}

/// Hairline segment midpoints (local plan) of one colour in a packed hairline lane.
fn hairline_mids(packed: &[f32], col: [f32; 4]) -> Vec<[f64; 2]> {
    packed
        .chunks_exact(12)
        .filter(|v| v[2..6] == col[..])
        .map(|v| {
            from_world([
                0.5 * (f64::from(v[0]) + f64::from(v[6])),
                0.5 * (f64::from(v[1]) + f64::from(v[7])),
            ])
        })
        .collect()
}

/// With a drawing the walls are the mesh's eye-height section (window gap and all),
/// the low cut is added, the plate is the clipped heightfield, and the blueprint's
/// apertures stay.
#[test]
fn mesh_drawing_replaces_walls_and_paints_heightfield() {
    let (bp, sc) = box_room();
    let d = building_drawing(&bp, &sc);
    let plain = build_static_lanes(&bp, None, ViewFloor::Level(0));
    let mesh = build_static_lanes(&bp, Some(&d), ViewFloor::Level(0));
    assert_eq!(plain.wall_count, 4, "blueprint walls on the fallback path");
    assert_eq!((plain.cut_count, plain.mesh_cell_count), (0, 0));
    assert!(mesh.cut_count > 4, "section segments: {}", mesh.cut_count);
    assert_eq!(mesh.wall_count, mesh.cut_count, "one strip per cut segment");
    assert_eq!(
        mesh.aperture_count, 1,
        "blueprint apertures stay as annotation"
    );
    assert!(
        mesh.mesh_cell_count > 0,
        "wall footprints at floor level are surfaces"
    );
    assert!(mesh.hairline_count > plain.hairline_count, "low cut added");
    assert!(
        mesh.floor_pos.len() > plain.floor_pos.len(),
        "heightfield cells replace the polygon fill"
    );
    // The ramp is monotone and clamped.
    let stops = [(0.0, [0.0, 0.0, 0.0, 0.0]), (1.0, [1.0, 1.0, 1.0, 1.0])];
    assert_eq!(ramp(&stops, -1.0), [0.0; 4]);
    assert_eq!(ramp(&stops, 2.0), [1.0; 4]);
    assert!((ramp(&stops, 0.25)[0] - 0.25).abs() < 1e-6);
    // The window hole opens the eye-height section: no south-wall segment spans x = 0.
    let spans_window = d.levels[0].cut_main.iter().any(|s| {
        (s[0][1] + 5.0).abs() < 0.15
            && (s[1][1] + 5.0).abs() < 0.15
            && s[0][0].min(s[1][0]) < 0.0
            && s[0][0].max(s[1][0]) > 0.0
    });
    assert!(!spans_window, "the window hole must open the section");
    // …while the low cut runs under the sill.
    let sill = d.levels[0].cut_low.iter().any(|s| {
        (s[0][1] + 5.1).abs() < 1e-6
            && (s[1][1] + 5.1).abs() < 1e-6
            && s[0][0].min(s[1][0]) < 0.0
            && s[0][0].max(s[1][0]) > 0.0
    });
    assert!(sill, "the low cut must run continuous under the window");
}

/// On the upper floor the ground floor's section shows ONLY through the stairwell:
/// ghost pieces exist in the hole, none under the solid slab.
#[test]
fn lower_floor_ghosts_only_through_voids() {
    let (bp, sc) = box_room_two_level(false);
    let d = building_drawing(&bp, &sc);
    let up = build_static_lanes(&bp, Some(&d), ViewFloor::Level(1));
    assert!(
        up.mesh_cell_count > 0,
        "the ceiling slab is the upper floor"
    );
    let ghosts = hairline_mids(&up.hairlines, COL_GHOST);
    // Pieces are 0.2 m long, so a midpoint can sit on the hole edge: allow half a piece.
    let in_hole = |m: &[f64; 2]| (0.35..=1.65).contains(&m[0]) && (0.35..=1.65).contains(&m[1]);
    assert!(
        ghosts.iter().any(in_hole),
        "no ghost through the stairwell: {ghosts:?}"
    );
    let leaked: Vec<&[f64; 2]> = ghosts
        .iter()
        .filter(|m| m[0].abs() < 4.9 && m[1].abs() < 4.9 && !in_hole(m))
        .collect();
    assert!(leaked.is_empty(), "ghosts under solid floor: {leaked:?}");
    // The ground floor itself has no ghosts (nothing below it).
    let ground = build_static_lanes(&bp, Some(&d), ViewFloor::Level(0));
    assert!(hairline_mids(&ground.hairlines, COL_GHOST).is_empty());
}

/// The Roof view paints the mesh top surface as a heightfield and ghosts the blueprint's
/// wall centerlines (few and clean), never the mesh cuts.
#[test]
fn roof_view_paints_heightfield_and_centerline_ghosts() {
    let (bp, sc) = box_room_two_level(true);
    let d = building_drawing(&bp, &sc);
    let roof = build_static_lanes(&bp, Some(&d), ViewFloor::Roof);
    assert!(roof.mesh_cell_count > 0, "roof slab cells");
    assert_eq!(roof.roof_cell_count, 0, "no RoofGrid on the synthetic room");
    assert_eq!((roof.wall_count, roof.cut_count), (0, 0));
    assert_eq!(roof.hairline_count, 8, "4 + 4 blueprint centerlines");
    // 1e-5, not 1e-9: T-938.4 made HeightField storage sparse `f32` + NaN, so every
    // height that reaches here is f32-rounded (6.2 reads back 6.199999809…). The
    // sibling goldens in building_section_tests.rs carry the same widened tolerance.
    assert!((d.roof_y[1] - 6.2).abs() < 1e-5, "roof_y {:?}", d.roof_y);
    // Every painted roof cell is at the slab top (the highest surface wins).
    let top = d.roof.value_at(0.0, 0.0).expect("roof over the room");
    assert!((top - 6.2).abs() < 1e-5, "roof top {top}");
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
