//! The shared dump-generation skeleton: the ONE home for the wire-format conventions every
//! generator must honor (scanner pads, 0.1 m lattice at cell centers, origin normalization,
//! r2 rounding, march order, empty-scanline omission). `synth` (analytic test solids) and
//! `mesh` (real triangle geometry) both march through here, so a convention drift between
//! generators is structurally impossible — they can only differ in intersection math.
//!
//! Conventions mirrored from the Workbench sensor (TBD_BuildingTraceScanner.c:56-57,
//! TBD_BuildingVoxelDump.c WriteMeta): scan min = bbox − 0.6 on all axes, scan max =
//! bbox + (0.6, 1.2, 0.6) — the extra y headroom lets top-down rays start above chimneys —
//! and dims = ceil(span / cell).

use super::types::{DumpMeta, ExcludedCounts, VoxelDump};

pub const CELL: f64 = 0.1;
pub const PAD: f64 = 0.6;

/// Identity fields the caller stamps into the meta line.
pub struct DumpIdent {
    pub slug: String,
    pub resource: String,
}

/// Round to the dumper's 2-decimal wire precision.
pub fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// March all six directions over one generator's line function, local frame; normalize by
/// origin = bounds − PAD (with the scanner's +1.2 y top pad on span).
///
/// `line_fn(axis, a, b)` returns `(fwd, bwd)` entry positions in LOCAL axis coordinates,
/// already in MARCH order: `fwd` ascending along +axis, `bwd` descending along +axis (the
/// order a −axis march visits them). The skeleton normalizes (− origin[axis]), r2-rounds,
/// and inserts non-empty scanlines only. Lines: axis 0 at (fy=a, fz=b) over (iy, iz);
/// axis 1 at (fx=a, fz=b) over (ix, iz); axis 2 at (fx=a, fy=b) over (ix, iy).
pub fn generate_dump<F>(
    ident: DumpIdent,
    bbox_min: [f64; 3],
    bbox_max: [f64; 3],
    mut line_fn: F,
) -> VoxelDump
where
    F: FnMut(usize, f64, f64) -> (Vec<f64>, Vec<f64>),
{
    let origin = [bbox_min[0] - PAD, bbox_min[1] - PAD, bbox_min[2] - PAD];
    let span = [
        bbox_max[0] - bbox_min[0] + 2.0 * PAD,
        bbox_max[1] - bbox_min[1] + PAD + 1.2, // mirror the scanner's +1.2 top pad
        bbox_max[2] - bbox_min[2] + 2.0 * PAD,
    ];
    let dims = [
        (span[0] / CELL).ceil() as usize,
        (span[1] / CELL).ceil() as usize,
        (span[2] / CELL).ceil() as usize,
    ];

    let mut dump = VoxelDump {
        meta: Some(DumpMeta {
            v: super::types::DUMP_VERSION.to_string(),
            slug: ident.slug,
            resource: ident.resource,
            origin,
            cell: CELL,
            dims,
            span,
            bbox_min,
            bbox_max,
            root_yaw_deg: 0.0,
            excluded: ExcludedCounts {
                doors: 0,
                glass: 0,
                furniture: 0,
            },
            tick: 0,
        }),
        ..VoxelDump::default()
    };

    // x lines over (iy, iz); y lines over (ix, iz); z lines over (ix, iy).
    for iy in 0..dims[1] {
        let fy = origin[1] + (iy as f64 + 0.5) * CELL;
        for iz in 0..dims[2] {
            let fz = origin[2] + (iz as f64 + 0.5) * CELL;
            let (fwd, bwd) = line_fn(0, fy, fz);
            if fwd.is_empty() && bwd.is_empty() {
                continue;
            }
            let fwd: Vec<f64> = fwd.iter().map(|v| r2(v - origin[0])).collect();
            let bwd: Vec<f64> = bwd.iter().map(|v| r2(v - origin[0])).collect();
            if !fwd.is_empty() {
                dump.x_pos.insert((iy, iz), fwd);
            }
            if !bwd.is_empty() {
                dump.x_neg.insert((iy, iz), bwd);
            }
        }
    }
    for ix in 0..dims[0] {
        let fx = origin[0] + (ix as f64 + 0.5) * CELL;
        for iz in 0..dims[2] {
            let fz = origin[2] + (iz as f64 + 0.5) * CELL;
            let (up, down) = line_fn(1, fx, fz);
            if up.is_empty() && down.is_empty() {
                continue;
            }
            let up: Vec<f64> = up.iter().map(|v| r2(v - origin[1])).collect();
            let down: Vec<f64> = down.iter().map(|v| r2(v - origin[1])).collect();
            if !up.is_empty() {
                dump.y_up.insert((ix, iz), up);
            }
            if !down.is_empty() {
                dump.y_down.insert((ix, iz), down);
            }
        }
    }
    for ix in 0..dims[0] {
        let fx = origin[0] + (ix as f64 + 0.5) * CELL;
        for iy in 0..dims[1] {
            let fy = origin[1] + (iy as f64 + 0.5) * CELL;
            let (fwd, bwd) = line_fn(2, fx, fy);
            if fwd.is_empty() && bwd.is_empty() {
                continue;
            }
            let fwd: Vec<f64> = fwd.iter().map(|v| r2(v - origin[2])).collect();
            let bwd: Vec<f64> = bwd.iter().map(|v| r2(v - origin[2])).collect();
            if !fwd.is_empty() {
                dump.z_pos.insert((ix, iy), fwd);
            }
            if !bwd.is_empty() {
                dump.z_neg.insert((ix, iy), bwd);
            }
        }
    }
    dump
}
