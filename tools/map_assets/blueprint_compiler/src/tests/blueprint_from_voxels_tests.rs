use super::*;
use crate::test_fixtures::fixture;
use crate::voxel_processing::synthetic_fixtures;

/// The floors-and-walls fixture end to end: partial mezzanine plate (void stays void),
/// the under-roof knee wall surviving via the roof-clipped persistence denominator, and
/// the attic band self-synthesizing with gable ends and no plate.
#[test]
fn gable_mezzanine_bands_plate_and_knee_wall() {
    let d = synthetic_fixtures::gable_mezzanine();
    let m = d.metadata().clone();
    let p = AnalysisParameters {
        min_floor_y: -0.5 - m.origin[1],
        ..Default::default()
    };
    let vert = vertical_slabs::analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
    assert_eq!(
        vert.floors.len(),
        2,
        "ground + mezzanine: {:?}",
        vert.floors
    );

    let mut dbg: Vec<wall_extraction::BandDebug> = Vec::new();
    let bands = build_bands(&d, &vert, WallAlgorithm::Segments, &p, Some(&mut dbg));
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

fn w_len(w: &voxel_types::WallSegment) -> f64 {
    ((w.end[0] - w.start[0]).powi(2) + (w.end[1] - w.start[1]).powi(2)).sqrt()
}

/// Full pipeline on the real FarmHouse dump == the committed golden. Any heuristic change
/// that shifts the output must re-bless the golden deliberately.
#[test]
fn farmhouse_dump_matches_golden_blueprint() {
    let bp = interpret_one(
        &fixture("FarmHouse_E_1L01_Wood_voxels.jsonl.gz"),
        WallAlgorithm::Segments,
        &AnalysisParameters::default(),
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
