use super::*;
use crate::blueprint::types::{DUMP_VERSION, ExcludedCounts, VoxelDump};
use crate::blueprint::{slabs, synth};

fn analyzed(d: &VoxelDump) -> (VerticalScan, DumpMeta, Params) {
    let m = d.meta().clone();
    let p = Params {
        min_floor_y: -0.5 - m.origin[1],
        ..Default::default()
    };
    let v = slabs::analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
    (v, m, p)
}

#[test]
fn gable_box_grid_min_biases_low() {
    let d = synth::gable_box(6.0, 4.0, 2.6, 4.2, 0.15);
    let (v, m, p) = analyzed(&d);
    let g = build(&v, &m, &p).expect("gable emits a roof");
    assert!(
        (g.cell_size_m - 0.3).abs() < 1e-9,
        "k = 3 at the 0.1 m dump cell"
    );
    assert_eq!(g.nx, v.nx.div_ceil(3));
    assert_eq!(g.nz, v.nz.div_ceil(3));

    let ridge_local = m.origin[1] + v.ridge;
    let eave_local = m.origin[1] + v.eave;
    let mid_z = (m.bbox_min[2] + m.bbox_max[2]) * 0.5;
    let slope = (ridge_local - eave_local) / ((m.bbox_max[2] - m.bbox_min[2]) * 0.5);
    let mut max_h = f64::NEG_INFINITY;
    for cx in 0..g.nx {
        for cz in 0..g.nz {
            let Some(h) = g.heights_m[cx * g.nz + cz] else {
                continue;
            };
            max_h = max_h.max(h);
            // Local frame + bounded band.
            assert!(
                h > eave_local - 0.4 && h < ridge_local + 0.011,
                "({cx},{cz}) h {h} outside [{eave_local}, {ridge_local}]"
            );
            // Min aggregation ⇒ at or below the analytic plane at the block center.
            let zc = g.origin[1] + (cz as f64 + 0.5) * g.cell_size_m;
            let analytic_center = ridge_local - slope * (zc - mid_z).abs();
            assert!(
                h <= analytic_center + 0.06,
                "({cx},{cz}) h {h} above analytic center {analytic_center}"
            );
            // 2-dp emit.
            assert!((h * 100.0 - (h * 100.0).round()).abs() < 1e-6);
        }
    }
    // The ridge row survives min-bias to within slope·cell + quantization.
    assert!(
        max_h > ridge_local - 0.45,
        "ridge row lost: max {max_h} vs ridge {ridge_local}"
    );
    // The PAD air ring (0.6 m ≥ one coarse block) nulls the whole border.
    for cx in 0..g.nx {
        assert!(g.heights_m[cx * g.nz].is_none());
        assert!(g.heights_m[cx * g.nz + g.nz - 1].is_none());
    }
    for cz in 0..g.nz {
        assert!(g.heights_m[cz].is_none());
        assert!(g.heights_m[(g.nx - 1) * g.nz + cz].is_none());
    }
}

#[test]
fn box_room_flat_roof_covers_plate() {
    let d = synth::box_room(6.0, 4.0, 2.6, 0.15);
    let (v, m, p) = analyzed(&d);
    let g = build(&v, &m, &p).expect("flat box emits a roof");
    let covered: Vec<f64> = g.heights_m.iter().flatten().copied().collect();
    assert!(!covered.is_empty());
    let (lo, hi) = covered
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &h| {
            (a.min(h), b.max(h))
        });
    // Flat top near local 2.6, and flat means flat.
    assert!(lo > 2.3 && hi < 2.9, "flat roof band, got [{lo}, {hi}]");
    assert!(hi - lo < 0.15, "flat roof spread, got {}", hi - lo);
}

/// Micro-grid unit for the three guards: coverage nulling, the ground-clutter drop, and
/// erosion (which here removes everything ⇒ `build` returns `None`).
#[test]
fn coverage_ground_filter_and_erosion_guards() {
    let (nx, nz) = (8usize, 8usize);
    let mut top = vec![None; nx * nz];
    for ix in 0..8 {
        for iz in 0..8 {
            let h = match (ix < 4, iz < 4) {
                (true, true) => Some(0.3),  // stoop — below floor + 1.2
                (false, true) => Some(3.0), // roof block, punctured below
                _ => Some(3.0),             // roof
            };
            top[ix * nz + iz] = h;
        }
    }
    top[5 * nz + 2] = None; // puncture the (1,0) coarse block: 15/16 coverage
    let vert = VerticalScan {
        slabs: vec![0.0],
        floors: vec![0.0],
        eave: 3.0,
        ridge: 3.0,
        chimney: None,
        top,
        top_slope: vec![0.0; nx * nz],
        nx,
        nz,
    };
    let meta = DumpMeta {
        v: DUMP_VERSION.to_string(),
        slug: "micro".to_string(),
        resource: "synthetic://micro".to_string(),
        origin: [0.0, 0.0, 0.0],
        cell: 0.1,
        dims: [nx, 40, nz],
        span: [0.8, 4.0, 0.8],
        bbox_min: [0.0, 0.0, 0.0],
        bbox_max: [0.8, 3.0, 0.8],
        root_yaw_deg: 0.0,
        excluded: ExcludedCounts {
            doors: 0,
            glass: 0,
            furniture: 0,
        },
        tick: 0,
    };
    // Pin the pitch: this micro-grid is designed around 4-fine-cell blocks.
    let p = Params {
        roof_cell_m: 0.4,
        ..Default::default()
    };
    let g = build(&vert, &meta, &p).expect("two roof blocks survive");
    assert_eq!((g.nx, g.nz), (2, 2));
    assert_eq!(g.heights_m[0], None, "stoop dropped by the ground filter");
    assert_eq!(g.heights_m[2], None, "punctured block dropped by coverage");
    assert_eq!(g.heights_m[1], Some(3.0));
    assert_eq!(g.heights_m[3], Some(3.0));

    let pe = Params {
        roof_erode_cells: 1,
        ..Default::default()
    };
    assert!(
        build(&vert, &meta, &pe).is_none(),
        "one erosion pass clears the 2×2 survivors ⇒ no grid at all"
    );
}
