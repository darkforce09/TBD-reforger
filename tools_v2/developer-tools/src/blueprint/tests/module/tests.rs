use super::*;

pub(crate) fn fixture(name: &str) -> std::path::PathBuf {
    // Compile-time root, not the cwd walk: see `root::test_repo_root` for the race.
    crate::repository_paths::test_repo_root()
        .join("tools_v2/developer-tools/test_fixtures/blueprint")
        .join(name)
}

/// The floors-and-walls fixture end to end: partial mezzanine plate (void stays void),
/// the under-roof knee wall surviving via the roof-clipped persistence denominator, and
/// the attic band self-synthesizing with gable ends and no plate.
#[test]
fn gable_mezzanine_bands_plate_and_knee_wall() {
    let d = synth::gable_mezzanine();
    let m = d.meta().clone();
    let p = Params {
        min_floor_y: -0.5 - m.origin[1],
        ..Default::default()
    };
    let vert = slabs::analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
    assert_eq!(
        vert.floors.len(),
        2,
        "ground + mezzanine: {:?}",
        vert.floors
    );

    let mut dbg: Vec<walls::BandDebug> = Vec::new();
    let bands = build_bands(&d, &vert, Algo::Segments, &p, Some(&mut dbg));
    assert_eq!(bands.len(), 3, "two floors + attic");
    assert_eq!(dbg.len(), 3);

    // Attic: [band1_hi .. ridge], no plate products, gable ends on both x extremes.
    let attic = &bands[2];
    assert!(attic.is_attic);
    assert!((attic.band_hi - vert.ridge).abs() < 1e-9);
    assert!(attic.plate_heights.is_empty() && attic.footprint.is_empty());
    let gable_ends = attic
        .walls
        .walls
        .iter()
        .filter(|w| (w.start[0] - w.end[0]).abs() < 1e-9)
        .count();
    assert!(
        gable_ends >= 2,
        "gable ends missing: {:?}",
        attic.walls.walls
    );

    // Mezzanine plate covers roughly the west half — and only that.
    assert!(bands[1].plate_cells > 0);
    assert!(
        bands[1].plate_cells * 3 < bands[0].plate_cells * 2,
        "mezzanine plate must stay partial: {} vs ground {}",
        bands[1].plate_cells,
        bands[0].plate_cells
    );

    // The knee wall (x-running; solid z = 0.325 → normalized 0.925 after the 0.6 m dump
    // PAD) survives in band 1: only ~6/16 whole-window rows, but ≥ need against its
    // roof-clipped denominator.
    let knee = bands[1]
        .walls
        .walls
        .iter()
        .find(|w| {
            (w.start[1] - w.end[1]).abs() < 1e-9
                && (w.start[1] - 0.925).abs() < 0.15
                && w.start[0].min(w.end[0]) > 3.4
        })
        .unwrap_or_else(|| panic!("knee wall missing: {:?}", bands[1].walls.walls));
    assert!(
        (w_len(knee) - 1.8).abs() < 0.4,
        "knee wall run length: {:?}",
        knee
    );
    let knee_cluster = dbg[1]
        .clusters
        .iter()
        .find(|c| c.axis == "x-running" && (c.center - 0.925).abs() < 0.15 && c.fixed > 34)
        .expect("knee cluster recorded");
    assert_eq!(knee_cluster.verdict, "accepted");
    assert!(
        knee_cluster.rows_avail < 16 && knee_cluster.rows_seen >= knee_cluster.need,
        "roof clipping engaged: {knee_cluster:?}"
    );
}

fn w_len(w: &types::WallSeg) -> f64 {
    ((w.end[0] - w.start[0]).powi(2) + (w.end[1] - w.start[1]).powi(2)).sqrt()
}

/// Full pipeline on the real FarmHouse dump == the committed golden. Any heuristic change
/// that shifts the output must re-bless the golden deliberately.
#[test]
fn farmhouse_dump_matches_golden_blueprint() {
    let bp = interpret_one(
        &fixture("FarmHouse_E_1L01_Wood_voxels.jsonl.gz"),
        Algo::Segments,
        &Params::default(),
        None,
    )
    .expect("interpret fixture dump");
    let got = serde_json::to_value(&bp).expect("serialize");
    let golden: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixture("FarmHouse_E_1L01_Wood_blueprint.golden.json"))
            .expect("read golden"),
    )
    .expect("parse golden");
    // The writer strips null optionals; mirror that for the comparison.
    let mut got = got;
    if got.get("modelMesh").is_some_and(serde_json::Value::is_null) {
        got.as_object_mut().expect("obj").remove("modelMesh");
    }
    assert_eq!(
        got, golden,
        "pipeline drifted from the blessed golden — re-bless on purpose"
    );
}

/// The acceptance instrument, pinned: replay the committed 400-pair engine oracle through
/// the golden blueprint + the golden `.bvh` sidecar. **400/400** since `evaluate_los` moved
/// onto the BVH raycaster (step 3, measured 2026-09-01). 2.5D: 260/400 pre-roof
/// (every miss the unmodeled roof) → 384 roof heightfield → 387 attic band + above-roof
/// wall cap, where the 13 misses were all model-clear/engine-blocked roof-margin leans.
/// Same instrument as `bvh::tests::farmhouse_bvh_sidecar_parity_is_pinned` by construction
/// (`Bvh::first_hit` ⇔ `Bvh::any_hit` on existence) — kept as the blueprint-lane pin so an
/// attribution or annotation bug that flips `is_clear` fails HERE, on the shipping path.
#[test]
fn farmhouse_golden_parity_is_pinned() {
    #[derive(serde::Deserialize)]
    struct ParityFile {
        pairs: Vec<(f64, f64, f64, f64, f64, f64, bool)>,
    }
    let bp: website_map_engine::world::architecture::blueprint::structure::BuildingBlueprint =
        serde_json::from_str(
            &std::fs::read_to_string(fixture("FarmHouse_E_1L01_Wood_blueprint.golden.json"))
                .expect("read golden"),
        )
        .expect("parse golden");
    let sidecar = website_map_engine::spatial::bvh::sidecar::BvhSidecar::parse(
        &std::fs::read(fixture("FarmHouse_E_1L01_Wood.bvh.golden")).expect("read sidecar"),
    )
    .expect("parse golden sidecar");
    let oracle: ParityFile = serde_json::from_str(
        &std::fs::read_to_string(fixture("FarmHouse_E_1L01_Wood_parity.json"))
            .expect("read parity"),
    )
    .expect("parse parity");
    assert_eq!(oracle.pairs.len(), 400);
    let mut agree = 0usize;
    let mut model_blocked_engine_clear = 0usize;
    for &(ox, oy, oz, tx, ty, tz, engine_clear) in &oracle.pairs {
        let model_clear = bp
            .evaluate_los(&sidecar, [ox, oy, oz], [tx, ty, tz])
            .is_clear;
        if model_clear == engine_clear {
            agree += 1;
        } else if !model_clear && engine_clear {
            model_blocked_engine_clear += 1;
        }
    }
    assert_eq!(agree, 400, "parity drifted (was 100%)");
    assert_eq!(
        model_blocked_engine_clear, 0,
        "phantom geometry blocks rays the engine clears"
    );
}
