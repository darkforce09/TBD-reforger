//! Unit tests for the blueprint contract and the 2.5D LOS evaluator — split out of
//! `building_blueprint.rs` (SIZE gate) via `#[path]`, same super-scope semantics.

use super::*;

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
fn evaluates_los_through_window() {
    let bp = farmhouse();

    // Outside observer looking straight through the south window at [-3.8, -4.5]
    let obs = [-3.8, 1.4, -8.0];
    let tgt = [-3.8, 1.4, -1.0];

    let los = bp.evaluate_los(obs, tgt);
    assert!(los.is_clear);
    assert!(
        los.window_ids_traversed
            .contains(&"win_gf_front_left".to_string())
    );
    assert_eq!(los.blocked_by_wall_id, None);
    // The ordered hit trace carries the same event with its position on the wall plane.
    let win_hit = los
        .hits
        .iter()
        .find(|h| h.kind == LosHitKind::Window)
        .expect("window hit present");
    assert_eq!(win_hit.id, "win_gf_front_left");
    assert!((win_hit.pos[2] - -4.5).abs() < 1e-9);
    assert!(win_hit.t > 0.0 && win_hit.t < 1.0);
}

#[test]
fn evaluates_los_blocked_by_log_wall() {
    let bp = farmhouse();

    // Outside observer looking at solid wall at [-5.5, -4.5] (away from windows and doors)
    let obs = [-5.5, 1.4, -8.0];
    let tgt = [-5.5, 1.4, -1.0];

    let los = bp.evaluate_los(obs, tgt);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some("w_ext_south".to_string()));
    assert_eq!(los.concealment, 1.0);
    // The terminal block is the LAST hit; nothing is recorded past it.
    let last = los.hits.last().expect("blocking hit recorded");
    assert_eq!(last.kind, LosHitKind::Wall);
    assert_eq!(last.id, "w_ext_south");
    assert!((last.concealment - 1.0).abs() < f64::EPSILON);
}

/// Low-cover furniture conceals without blocking. Hand-built host: the farmhouse fixture's
/// furniture went empty when 62d6ae3ad made it the v7 scan output (dump meta furniture: 0)
/// — the viewer's static-lanes test mirrors that; this one keeps the semantics covered.
#[test]
fn evaluates_los_furniture_low_cover() {
    let mut bp = gable_blueprint();
    bp.levels[0].furniture.push(BuildingFurniture {
        id: "furn_table_01".to_string(),
        name: "table".to_string(),
        category: "prop".to_string(),
        prefab_resource: "synthetic://table".to_string(),
        pos2_d: [4.0, 5.0],
        rotation_deg: 0.0,
        size2_d: [1.2, 0.8],
        height_m: 0.78,
        blocks_movement: true,
        los_cover: "low_cover".to_string(),
    });

    // Enters the table's AABB at y ≈ 0.64 (below the 0.78 top): concealed, still clear.
    let los = bp.evaluate_los([3.0, 0.7, 4.5], [5.0, 0.4, 5.5]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert_eq!(los.cover_furniture_id, Some("furn_table_01".to_string()));
    assert!(los.concealment >= 0.60);
    assert!(
        los.hits
            .iter()
            .any(|h| h.kind == LosHitKind::Furniture && h.id == "furn_table_01")
    );
}

#[test]
fn evaluates_los_second_floor_dormer_window() {
    let bp = farmhouse();

    // Observer outside on elevated slope looking into upper floor dormer window at [0.0, -4.5] (Y=3.8m)
    let obs = [0.0, 4.0, -9.0];
    let tgt = [0.0, 3.8, -1.0];

    let los = bp.evaluate_los(obs, tgt);
    assert!(los.is_clear);
    assert!(
        los.window_ids_traversed
            .contains(&"win_f1_dormer_south".to_string())
    );
}

/// The multi-band regression the old average-height pick got wrong: a ray climbing from
/// ground level outside to above the upstairs windows crosses the south wall PLANE while in
/// the level-1 band, so the blocker must be `w_f1_south` (upstairs siding), not the ground
/// floor's `w_ext_south` (which its sub-segment never reaches). The old code averaged
/// (0.9 + 4.5)/2 = 2.7 → picked level 0 for the whole ray and blamed `w_ext_south`.
#[test]
fn evaluates_los_cross_band_blames_correct_floor_wall() {
    let bp = farmhouse();

    let obs = [-3.8, 0.9, -12.0];
    let tgt = [-3.8, 4.5, 1.0];

    let los = bp.evaluate_los(obs, tgt);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some("w_f1_south".to_string()));
    // And no phantom ground-floor events: the level-0 clip ends before z = -4.5.
    assert!(los.hits.iter().all(|h| h.id != "w_ext_south"));
}

/// Same geometry through the aperture: raise the observer so the ray passes the south wall
/// plane inside the dormer window's sill..top band — clear, via the level-1 window, even
/// though the ray STARTED in the level-0 band.
#[test]
fn evaluates_los_cross_band_through_upstairs_window() {
    let bp = farmhouse();

    let obs = [0.0, 2.0, -12.0];
    let tgt = [0.0, 4.6, -1.0];

    let los = bp.evaluate_los(obs, tgt);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert!(
        los.window_ids_traversed
            .contains(&"win_f1_dormer_south".to_string())
    );
}

#[test]
fn clip_t_to_band_horizontal_boundary_belongs_to_upper_level() {
    // A horizontal ray exactly on the shared 2.8 boundary: excluded from the half-open
    // ground band, included in the closed top band.
    assert_eq!(clip_t_to_band(2.8, 2.8, [0.0, 2.8], false), None);
    assert_eq!(clip_t_to_band(2.8, 2.8, [2.8, 5.6], true), Some((0.0, 1.0)));
}

/// 14×14 half-metre synthetic gable: ridge along x at z = 3.5, pitch `h = 5.0 − |z_c − 3.5|
/// · 0.8` per cell center (eave 2.8, ridge row 4.8), a `None` silhouette ring, one chimney
/// cell at (3, 3) poking to 6.1, and a two-cell dormer pit at (8, 5..=6) dropping to 2.9.
fn gable_roof() -> RoofGrid {
    let (nx, nz) = (14usize, 14usize);
    let mut heights = vec![None; nx * nz];
    for ix in 1..nx - 1 {
        for iz in 1..nz - 1 {
            let z_c = (iz as f64 + 0.5) * 0.5;
            heights[ix * nz + iz] = Some(5.0 - (z_c - 3.5).abs() * 0.8);
        }
    }
    heights[3 * nz + 3] = Some(6.1); // chimney
    heights[8 * nz + 5] = Some(2.9); // dormer pit
    heights[8 * nz + 6] = Some(2.9);
    RoofGrid {
        origin: [0.0, 0.0],
        cell_size_m: 0.5,
        nx,
        nz,
        heights_m: heights,
    }
}

/// Minimal host blueprint for the synthetic gable: one 7×7 level with a single wall plane at
/// x = 2.0 (band [0, 2.4]) under the [`gable_roof`] heightfield.
fn gable_blueprint() -> BuildingBlueprint {
    let square = vec![[0.0, 0.0], [7.0, 0.0], [7.0, 7.0], [0.0, 7.0]];
    BuildingBlueprint {
        schema_version: "1.0.0".to_string(),
        prefab_id: "SyntheticGable".to_string(),
        resource_name: "synthetic://gable".to_string(),
        model_mesh: None,
        label: None,
        kind: "building".to_string(),
        category: "generic".to_string(),
        destructible: false,
        vertical_profile: VerticalProfile {
            pivot_elevation_offset_m: 0.0,
            foundation_skirt_depth_m: 0.0,
            total_height_m: 6.1,
            eave_height_m: 2.8,
            ridge_height_m: 5.0,
            chimney_height_m: Some(6.1),
            roof_type: "scanned".to_string(),
        },
        overall_footprint: OverallFootprint {
            polygon2_d: square.clone(),
            bounding_box2_d: BBox2D {
                min: [0.0, 0.0],
                max: [7.0, 7.0],
                width_m: 7.0,
                depth_m: 7.0,
            },
            footprint_sq_m: 49.0,
        },
        roof: Some(gable_roof()),
        levels: vec![BuildingLevel {
            level_index: 0,
            name: "1st".to_string(),
            elevation_range: [0.0, 2.4],
            slice_height_m: 1.2,
            footprint_polygon: square,
            plate: None,
            floor_polygons: vec![],
            walls: vec![BuildingWall {
                id: "w_test".to_string(),
                start: [2.0, 0.0],
                end: [2.0, 7.0],
                thickness: 0.2,
                is_exterior: true,
                material: "scanned".to_string(),
            }],
            doors: vec![],
            windows: vec![],
            stairs: vec![],
            furniture: vec![],
        }],
    }
}

#[test]
fn roof_crossing_blocks_through_pitch() {
    let bp = gable_blueprint();
    // Descends over the near eave into the rising pitch: d flips + → − on one continuous run.
    let los = bp.evaluate_los([3.5, 6.0, 0.2], [3.5, 2.0, 3.4]);
    assert!(!los.is_clear);
    let last = los.hits.last().expect("terminal roof hit");
    assert_eq!(last.kind, LosHitKind::Roof);
    assert_eq!(last.id, "roof");
    assert!((last.concealment - 1.0).abs() < f64::EPSILON);
    assert!(last.t > 0.4 && last.t < 0.75, "t = {}", last.t);
    assert!(
        last.pos[1] > 3.2 && last.pos[1] < 4.2,
        "pierce height = {}",
        last.pos[1]
    );
    assert_eq!(los.blocked_by_wall_id, None);
    assert!((los.concealment - 1.0).abs() < f64::EPSILON);
}

#[test]
fn roof_over_ridge_stays_clear() {
    let bp = gable_blueprint();
    // Horizontal at 6.0 — above the 4.8 ridge row everywhere along the path.
    let los = bp.evaluate_los([3.5, 6.0, 0.2], [3.5, 6.0, 6.8]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert!(los.hits.is_empty());
}

/// The fatal-flaw regression: a low ray entering the silhouette (surface jumps from the
/// `None` ring straight to roof height) must NOT read the jump as a roof crossing.
#[test]
fn roof_silhouette_entry_is_not_a_crossing() {
    let bp = gable_blueprint();
    let los = bp.evaluate_los([3.5, 1.0, -1.0], [3.5, 1.0, 8.0]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
    assert!(los.hits.is_empty());
}

/// An attic-height ray passing UNDER the dormer pit sees d flip − (under the pitch) to +
/// (above the pit floor) — the >0.9 m cheek step splits the runs, so no crossing. Without
/// the continuity guard this is a phantom block.
#[test]
fn roof_dormer_pit_passes() {
    let bp = gable_blueprint();
    let los = bp.evaluate_los([4.25, 3.2, 1.0], [4.25, 3.2, 4.6]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
}

/// Flying past the chimney cell at flank height: pitch → chimney → pitch are three separate
/// runs (2+ m steps), so neither flank flip is a crossing. Lean clear by design.
#[test]
fn roof_chimney_flank_needs_continuity() {
    let bp = gable_blueprint();
    let los = bp.evaluate_los([1.75, 5.0, 0.8], [1.75, 5.0, 2.8]);
    assert!(los.is_clear, "hits: {:?}", los.hits);
}

/// Skimming the flat ridge row inside the ±0.15 margin band — above or below — never
/// anchors a side, so no crossing can fire.
#[test]
fn roof_margin_rejects_skims() {
    let bp = gable_blueprint();
    let over = bp.evaluate_los([0.8, 4.9, 3.75], [6.2, 4.9, 3.75]);
    assert!(over.is_clear, "hits: {:?}", over.hits);
    let under = bp.evaluate_los([0.8, 4.7, 3.75], [6.2, 4.7, 3.75]);
    assert!(under.is_clear, "hits: {:?}", under.hits);
}

/// Vertical attic → sky ray: d flips − → + through the ridge-row surface = a genuine
/// piercing from below. Also exercises the zero-plan-length sampling path.
#[test]
fn roof_exit_from_below_blocks() {
    let bp = gable_blueprint();
    let los = bp.evaluate_los([3.5, 3.0, 3.75], [3.5, 7.0, 3.75]);
    assert!(!los.is_clear);
    let last = los.hits.last().expect("terminal roof hit");
    assert_eq!(last.kind, LosHitKind::Roof);
    assert!(
        (last.pos[1] - 4.8).abs() < 1e-9,
        "exit height = {}",
        last.pos[1]
    );
}

#[test]
fn wall_closer_than_roof_wins_attribution() {
    let bp = gable_blueprint();
    // Crosses the x = 2.0 wall plane at t = 0.25 (y = 2.25, inside the band) and would
    // pierce the ridge row at t ≈ 0.79 — the walk stops at the nearer wall.
    let los = bp.evaluate_los([0.5, 1.0, 3.6], [6.5, 6.0, 3.6]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some("w_test".to_string()));
    let last = los.hits.last().expect("terminal hit");
    assert_eq!(last.kind, LosHitKind::Wall);
    assert!(los.hits.iter().all(|h| h.kind != LosHitKind::Roof));
}

/// Roofless blueprints (every pre-roof JSON on map-assets) evaluate exactly as before, the
/// absent field round-trips away, and a shape-invalid grid is skipped, not trusted.
#[test]
fn blueprint_without_roof_is_unchanged() {
    let bp = farmhouse();
    assert!(bp.roof.is_none());
    let v = serde_json::to_value(&bp).expect("serialize");
    assert!(v.get("roof").is_none(), "absent roof must not serialize");
    let los = bp.evaluate_los([-5.5, 1.4, -8.0], [-5.5, 1.4, -1.0]);
    assert!(!los.is_clear);
    assert_eq!(los.blocked_by_wall_id, Some("w_ext_south".to_string()));

    let mut broken = gable_blueprint();
    broken.roof.as_mut().expect("has roof").heights_m.pop();
    assert!(!broken.roof.as_ref().expect("has roof").is_valid());
    // The pitch-piercing ray from `roof_crossing_blocks_through_pitch` — with the grid
    // invalid the roof test is skipped and nothing else is on the path.
    let los = broken.evaluate_los([3.5, 6.0, 0.2], [3.5, 2.0, 3.4]);
    assert!(los.is_clear, "invalid grid must be ignored: {:?}", los.hits);
}

/// 2.5D wall planes span their band uniformly, but real gable/knee walls are triangles under
/// the roof: a wall hit ABOVE the roof heightfield is open air and must not block; the same
/// wall below the surface still does.
#[test]
fn wall_hits_above_the_roof_surface_are_void() {
    let mut bp = gable_blueprint();
    bp.levels[0].elevation_range = [0.0, 6.0]; // wall plane now extends past the roof

    // Horizontal at 5.5 across the x = 2.0 wall where the pitch sits at ~3.2 — open air.
    let over = bp.evaluate_los([0.5, 5.5, 1.0], [3.5, 5.5, 1.0]);
    assert!(over.is_clear, "hits: {:?}", over.hits);

    // Same crossing at 2.0 — under the roof, the wall is real.
    let under = bp.evaluate_los([0.5, 2.0, 1.0], [3.5, 2.0, 1.0]);
    assert!(!under.is_clear);
    assert_eq!(under.blocked_by_wall_id, Some("w_test".to_string()));
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
    let mut bp = gable_blueprint();
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
