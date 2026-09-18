use super::*;
use crate::blueprint::{plate, slabs, synth, walls};

#[test]
fn box_room_blueprint_passes_the_schema_contract() {
    let d = synth::box_room(6.0, 4.0, 2.6, 0.15);
    let m = d.meta().clone();
    let p = Params {
        min_floor_y: -0.5 - m.origin[1],
        ..Default::default()
    };
    let v = slabs::analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
    let lo = v.floors[0];
    let hi = v.eave.max(lo + p.top_band_min_m);
    let bw = walls::extract_band(&d, &v, lo, hi, walls::Algo::Segments, &p, None);
    let (pg, plate_heights) = plate::floor_plate(&d.y_down, v.nx, v.nz, lo, &p);
    let plate_cells = pg.count();
    let traced = crate::blueprint::rings::trace(&pg, m.cell, p.plate_min_ring_area_m2);
    let (footprint, floor_polygons) = traced.contract();
    let bp = assemble(
        &d,
        &v,
        vec![BandProducts {
            band_lo: lo,
            band_hi: hi,
            walls: bw,
            footprint,
            floor_polygons,
            plate_heights,
            plate_cells,
            is_attic: false,
        }],
        &p,
    );

    assert_eq!(bp.levels.len(), 1);
    assert_eq!(bp.levels[0].walls.len(), 4);
    // Local frame: west wall (z-running, constant x) centerline near x = 0.075.
    let west_x = bp.levels[0]
        .walls
        .iter()
        .filter(|w| (w.start[0] - w.end[0]).abs() < 1e-9)
        .map(|w| w.start[0])
        .fold(f64::MAX, f64::min);
    assert!(
        (west_x - 0.075).abs() < 0.06,
        "local-frame west centerline, got {west_x}"
    );

    let root = crate::repository_paths::test_repo_root();
    let schema = root.join("packages/tbd-schema/schema/building-blueprint.schema.json");
    let tmp = std::env::temp_dir().join("tbd_bp_synth_schema_test.json");
    validate_and_write(&bp, &schema, &tmp).expect("schema-valid blueprint");
    let _ = std::fs::remove_file(tmp);
}
