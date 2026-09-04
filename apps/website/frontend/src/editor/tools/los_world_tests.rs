//! Goldens for [`super`] — the combined verdict language, the styling map, the blocking marker,
//! the merged palette and the coarse-to-fine object pass over a synthetic viewshed.

use std::cell::Cell;

use map_engine_core::dem::sample::{Viewshed, Visibility};

use super::*;
use crate::editor::tools::los_tool::{world_key, LosVerdict};

fn blocked(d: f64) -> LosVerdict {
    LosVerdict::Blocked {
        blocking_dist_m: d,
        blocking_elev_m: 150.0,
    }
}

fn obj_blocked(d: f64) -> ObjectVerdict {
    ObjectVerdict::Blocked {
        dist_m: d,
        label: "FarmHouse_E_1L01_Wood".into(),
        kind: "building".into(),
    }
}

#[test]
fn combined_header_names_the_nearer_blocker_and_the_object_layer_state() {
    let clear = ObjectVerdict::Clear {
        concealment: 0.0,
        glass_panes: 0,
    };
    assert_eq!(
        format_combined(&combine(LosVerdict::Clear, clear.clone()), 640.0),
        "LoS clear · 640 m"
    );
    assert_eq!(
        format_combined(
            &combine(
                LosVerdict::Clear,
                ObjectVerdict::Clear {
                    concealment: 0.43,
                    glass_panes: 0
                }
            ),
            640.0
        ),
        "LoS clear · 640 m · canopy 43 %"
    );
    assert_eq!(
        format_combined(
            &combine(LosVerdict::Clear, ObjectVerdict::NotLoaded),
            1240.0
        ),
        "LoS clear · 1.24 km · objects not loaded"
    );
    assert_eq!(
        format_combined(&combine(LosVerdict::Clear, obj_blocked(96.4)), 640.0),
        "LoS blocked at 96 m — FarmHouse_E_1L01_Wood (building)"
    );
    assert_eq!(
        format_combined(&combine(blocked(412.0), clear.clone()), 640.0),
        "LoS blocked at 412 m — terrain"
    );
    assert_eq!(
        format_combined(&combine(blocked(412.0), ObjectVerdict::NotLoaded), 640.0),
        "LoS blocked at 412 m — terrain"
    );
    // Both block: the nearer one names the header.
    assert_eq!(
        format_combined(&combine(blocked(412.0), obj_blocked(96.0)), 640.0),
        "LoS blocked at 96 m — FarmHouse_E_1L01_Wood (building)"
    );
    assert_eq!(
        format_combined(&combine(blocked(50.0), obj_blocked(96.0)), 640.0),
        "LoS blocked at 50 m — terrain"
    );
    assert_eq!(
        format_combined(
            &combine(
                LosVerdict::Clear,
                ObjectVerdict::Provisional {
                    dist_m: 96.0,
                    label: "Barn_01".into()
                }
            ),
            640.0
        ),
        "LoS provisional at 96 m — Barn_01 (geometry loading)"
    );
    assert_eq!(
        format_combined(&combine(LosVerdict::Unknown, obj_blocked(1.0)), 640.0),
        "LoS —"
    );
}

#[test]
fn first_block_and_styling_follow_the_nearest_block() {
    assert_eq!(
        first_block_dist(&combine(LosVerdict::Clear, ObjectVerdict::NotLoaded)),
        None
    );
    assert_eq!(
        first_block_dist(&combine(blocked(412.0), obj_blocked(96.0))),
        Some(96.0)
    );
    assert_eq!(
        first_block_dist(&combine(blocked(50.0), obj_blocked(96.0))),
        Some(50.0)
    );
    assert_eq!(
        first_block_dist(&combine(
            LosVerdict::Clear,
            ObjectVerdict::Provisional {
                dist_m: 7.0,
                label: "x".into()
            }
        )),
        Some(7.0)
    );
    assert!(styling_verdict(&combine(LosVerdict::Clear, obj_blocked(96.0))).is_blocked());
    assert!(styling_verdict(&combine(LosVerdict::Clear, ObjectVerdict::NotLoaded)).is_clear());
    assert_eq!(
        styling_verdict(&combine(LosVerdict::Unknown, obj_blocked(1.0))),
        LosVerdict::Unknown
    );
    // The object block keeps the terrain's elevation when the terrain also blocks (chart marker).
    assert_eq!(
        styling_verdict(&combine(blocked(412.0), obj_blocked(96.0))),
        LosVerdict::Blocked {
            blocking_dist_m: 96.0,
            blocking_elev_m: 150.0
        }
    );
}

#[test]
fn apply_objects_moves_the_marker_to_the_nearest_block() {
    let mut shot = ProjectedShot {
        obs_px: 100.0,
        obs_py: 100.0,
        tgt_px: 300.0,
        tgt_py: 100.0,
        verdict: LosVerdict::Clear,
        total_m: 400.0,
        block_px: None,
        objects: ObjectVerdict::NotLoaded,
        key: world_key(0.0, 0.0, 400.0, 0.0),
    };
    apply_objects(&mut shot, obj_blocked(100.0));
    assert_eq!(shot.block_px, Some((150.0, 100.0)));
    assert!(styling_of(&shot).is_blocked());
    shot.verdict = blocked(40.0);
    apply_objects(&mut shot, obj_blocked(100.0));
    assert_eq!(
        shot.block_px,
        Some((120.0, 100.0)),
        "the terrain block at 40 m is nearer"
    );
    apply_objects(&mut shot, ObjectVerdict::NotLoaded);
    assert_eq!(shot.block_px, Some((120.0, 100.0)));
    shot.verdict = LosVerdict::Clear;
    apply_objects(
        &mut shot,
        ObjectVerdict::Clear {
            concealment: 0.1,
            glass_panes: 0,
        },
    );
    assert_eq!(shot.block_px, None);
}

#[test]
fn merged_palette_is_terrain_first_then_the_object_verdict() {
    assert_eq!(
        object_cell_rgba(Visibility::Hidden, ObjectCell::Clear),
        VIEWSHED_HIDDEN_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Unknown, ObjectCell::Hidden),
        VIEWSHED_UNKNOWN_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Untested),
        VIEWSHED_VISIBLE_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Clear),
        VIEWSHED_VISIBLE_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Hidden),
        OBJECT_HIDDEN_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Provisional),
        OBJECT_PROVISIONAL_RGBA
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Concealed(0)),
        [34, 84, 36, 30]
    );
    assert_eq!(
        object_cell_rgba(Visibility::Visible, ObjectCell::Concealed(255)),
        [34, 84, 36, 90]
    );
    // Pinned constants: the object wash must stay visually distinct from the terrain wash.
    assert_ne!(OBJECT_HIDDEN_RGBA, VIEWSHED_HIDDEN_RGBA);
    assert_eq!(OBJECT_HIDDEN_RGBA, [78, 30, 24, 110]);
    assert_eq!(OBJECT_PROVISIONAL_RGBA, [128, 92, 20, 84]);
    assert_eq!(OBJECT_LEVELS, [4, 2, 1]);
    assert_eq!(OBJECT_PASS_BUDGET_MS, 8.0);
    assert_eq!(OBJECT_FINE_RADIUS_M, 1000.0);
    assert_eq!(map_to_engine(1.0, 2.0, 3.0), [1.0, 3.0, 2.0]);
}

/// A 9×9 raster of 8 m cells, all visible except a hidden column at col 8; observer at the
/// centre cell (4, 4).
fn synth_viewshed() -> Viewshed {
    let (cols, rows) = (9, 9);
    let mut cells = vec![Visibility::Visible; cols * rows];
    for r in 0..rows {
        cells[r * cols + 8] = Visibility::Hidden;
    }
    Viewshed {
        cols,
        rows,
        cells,
        min_x: 1000.0,
        min_y: 2000.0,
        max_x: 1064.0,
        max_y: 2064.0,
        obs_x: 1032.0,
        obs_y: 2032.0,
    }
}

#[test]
fn object_pass_runs_coarse_to_fine_nearest_first_and_only_over_visible_cells() {
    let vs = synth_viewshed();
    let mut pass = ObjectPass::new(&vs, 1);
    assert_eq!(pass.block(), 4);
    assert_eq!(pass.level_m(), 32.0);
    assert_eq!(pass.cell_center(0, 0), (1000.0, 2000.0));
    assert_eq!(pass.cell_center(4, 4), (1032.0, 2032.0));
    // 9 cells → blocks at 0, 4, 8 per axis = 9 blocks; the (8, *) column blocks are hidden-only
    // except they still contain… col 8 only → hidden → those 3 blocks are dropped: 6 queued.
    assert_eq!(pass.queue.len(), 6, "{:?}", pass.queue);
    // Nearest-first: the block holding the observer (anchor col 4, row 4) comes first.
    assert_eq!(pass.queue[0], (4 * 9 + 4) as u32);
    let ground = |_x: f64, _y: f64| Some(10.0);
    let calls = Cell::new(0u32);
    // Everything east of x = 1040 is blocked, everything else clear.
    let test = |obs: [f64; 3], tgt: [f64; 3]| {
        calls.set(calls.get() + 1);
        assert_eq!(
            obs,
            [1032.0, 11.8, 2032.0],
            "observer eye in the engine frame"
        );
        assert!((tgt[1] - (10.0 + EYE_HEIGHT_TARGET_M)).abs() < 1e-9);
        Some(if tgt[0] > 1040.0 {
            ObjectCell::Hidden
        } else {
            ObjectCell::Clear
        })
    };
    let clock = Cell::new(0.0f64);
    let now = || {
        clock.set(clock.get() + 0.5);
        clock.get()
    };
    // A 1 ms budget: the first step processes at most a couple of blocks, never all.
    let changed = pass.step(&vs, 11.8, &ground, &test, 1.0, &now);
    assert!(changed);
    assert!(
        pass.cursor >= 1 && pass.cursor < 6,
        "cursor {}",
        pass.cursor
    );
    assert!(!pass.done);
    // Run to completion with a generous budget.
    let mut guard = 0;
    while !pass.done && guard < 100 {
        pass.step(&vs, 11.8, &ground, &test, 1e9, &now);
        guard += 1;
    }
    assert!(pass.done);
    assert_eq!(pass.level_idx, 2);
    // Hidden-terrain cells stay Untested; visible cells got a verdict at 8 m within 1 km.
    for r in 0..9 {
        assert_eq!(pass.cells[r * 9 + 8], ObjectCell::Untested);
        for c in 0..8 {
            let x = 1000.0 + c as f64 * 8.0;
            let want = if x > 1040.0 {
                ObjectCell::Hidden
            } else {
                ObjectCell::Clear
            };
            assert_eq!(pass.cells[r * 9 + c], want, "cell ({c}, {r})");
        }
    }
    // 6 + 12 + … — every level tested its own blocks; the fine level tests every visible cell.
    assert_eq!(
        pass.tested,
        6 + 20 + 72,
        "blocks tested over the three levels"
    );
    assert_eq!(calls.get(), pass.tested);
    // Encoding: north row first, hidden column keeps the terrain wash, blocked cells the object wash.
    let rgba = encode_viewshed_rgba_merged(&vs, &pass);
    assert_eq!(rgba.len(), 9 * 9 * 4);
    let px = |c: usize, r_from_north: usize| {
        let i = (r_from_north * 9 + c) * 4;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    };
    assert_eq!(px(8, 0), VIEWSHED_HIDDEN_RGBA);
    assert_eq!(px(0, 0), VIEWSHED_VISIBLE_RGBA);
    assert_eq!(px(7, 8), OBJECT_HIDDEN_RGBA);
}

#[test]
fn object_pass_waits_when_the_occluder_is_unreachable_and_skips_off_dem_cells() {
    let vs = synth_viewshed();
    let mut pass = ObjectPass::new(&vs, 1);
    let no_ground = |_x: f64, _y: f64| None::<f64>;
    let never = |_o: [f64; 3], _t: [f64; 3]| Some(ObjectCell::Hidden);
    let now = || 0.0;
    // Off-DEM cells are skipped (stay Untested) but the pass still advances and completes.
    let mut guard = 0;
    while !pass.done && guard < 100 {
        pass.step(&vs, 11.8, &no_ground, &never, 1e9, &now);
        guard += 1;
    }
    assert!(pass.done);
    assert!(pass.cells.iter().all(|c| *c == ObjectCell::Untested));
    // An unreachable occluder: nothing marked, cursor does not move, not done.
    let mut pass = ObjectPass::new(&vs, 2);
    let ground = |_x: f64, _y: f64| Some(10.0);
    let unreachable = |_o: [f64; 3], _t: [f64; 3]| None::<ObjectCell>;
    assert!(!pass.step(&vs, 11.8, &ground, &unreachable, 1e9, &now));
    assert_eq!(pass.cursor, 0);
    assert!(!pass.done);
    assert_eq!(pass.progress(), (0, 6, 0));
}

#[test]
fn fine_level_is_bounded_to_one_kilometre() {
    // A raster far larger than the fine radius: 300 × 1 cells east of the observer.
    let cols = 300;
    let vs = Viewshed {
        cols,
        rows: 1,
        cells: vec![Visibility::Visible; cols],
        min_x: 0.0,
        min_y: 0.0,
        max_x: (cols - 1) as f64 * 8.0,
        max_y: 0.0,
        obs_x: 0.0,
        obs_y: 0.0,
    };
    let mut pass = ObjectPass::new(&vs, 1);
    let ground = |_x: f64, _y: f64| Some(0.0);
    let clear = |_o: [f64; 3], _t: [f64; 3]| Some(ObjectCell::Clear);
    let now = || 0.0;
    let mut guard = 0;
    while !pass.done && guard < 1000 {
        pass.step(&vs, 1.8, &ground, &clear, 1e9, &now);
        guard += 1;
    }
    // Coarse levels covered every cell; the 8 m level only queued cells within 1000 m (126 of 300).
    let coarse = 75 + 150;
    assert_eq!(pass.tested, coarse + 126, "tested {}", pass.tested);
}

#[test]
fn glass_panes_are_named_and_never_mislabelled_as_canopy() {
    // One pane, 5 %: "through glass", no canopy percentage.
    let c = combine(
        LosVerdict::Clear,
        ObjectVerdict::Clear {
            concealment: 0.05,
            glass_panes: 1,
        },
    );
    assert_eq!(format_combined(&c, 8.0), "LoS clear · 8 m · through glass");
    // A pane and a hedge: both named, the percentage is the combined concealment.
    let c = combine(
        LosVerdict::Clear,
        ObjectVerdict::Clear {
            concealment: 0.46,
            glass_panes: 1,
        },
    );
    assert_eq!(
        format_combined(&c, 40.0),
        "LoS clear · 40 m · through glass · canopy 46 %"
    );
    // Two panes (9.75 %) still read as glass only.
    let c = combine(
        LosVerdict::Clear,
        ObjectVerdict::Clear {
            concealment: 0.0975,
            glass_panes: 2,
        },
    );
    assert_eq!(
        format_combined(&c, 12.0),
        "LoS clear · 12 m · through glass"
    );
}

#[test]
fn provisional_cells_are_retested_after_a_requeue() {
    use std::cell::Cell;
    let vs = synth_viewshed();
    let mut pass = ObjectPass::new(&vs, 1);
    // First pass: everything east of the observer is provisional (its BLAS is still in flight).
    let arrived = Cell::new(false);
    let test = |_obs: [f64; 3], tgt: [f64; 3]| -> Option<ObjectCell> {
        if tgt[0] > vs.obs_x && !arrived.get() {
            Some(ObjectCell::Provisional)
        } else {
            Some(ObjectCell::Clear)
        }
    };
    let ground = |_x: f64, _y: f64| Some(0.0);
    let clock = Cell::new(0.0);
    let now = || {
        clock.set(clock.get() + 0.001);
        clock.get()
    };
    while !pass.step(&vs, 1.8, &ground, &test, 1000.0, &now) || !pass.done {
        if pass.done {
            break;
        }
    }
    assert!(pass.done);
    let before = pass.counts();
    assert!(before.3 > 0, "east half provisional: {before:?}");
    // Nothing to do while the BLAS is still pending — requeue is honest about that too.
    arrived.set(true);
    let queued = pass.requeue_provisional(&vs);
    assert!(queued > 0, "provisional blocks re-queued");
    assert!(!pass.done);
    while !pass.done {
        pass.step(&vs, 1.8, &ground, &test, 1000.0, &now);
    }
    let after = pass.counts();
    assert_eq!(after.3, 0, "no provisional cells remain: {after:?}");
    assert!(after.0 > before.0, "the retested cells became clear");
    // A pass with no provisional cells is left alone.
    assert_eq!(pass.requeue_provisional(&vs), 0);
    assert!(pass.done);
}
