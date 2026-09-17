//! Vertical analysis — the ScanVertical port plus the roof-slope field the live pipeline never
//! had. Input is the `y-` (top-down) scan map: first entry per column = top surface; every entry
//! is a downward-facing horizontal surface (floor plates and landings dominate; walls contribute
//! almost nothing to DOWNWARD entry faces).

use super::params::Params;
use super::types::{ScanMap, VerticalScan};

pub fn analyze(
    y_down: &ScanMap,
    dims: [usize; 3],
    cell: f64,
    span_y: f64,
    p: &Params,
) -> VerticalScan {
    let (nx, nz) = (dims[0], dims[2]);
    let mut top: Vec<Option<f64>> = vec![None; nx * nz];
    let n_bins = (span_y / p.slab_bin_m).ceil() as usize + 1;
    let mut bins = vec![0usize; n_bins];
    let mut top_ys: Vec<f64> = Vec::new();

    for (&(ix, iz), entries) in y_down {
        if ix >= nx || iz >= nz || entries.is_empty() {
            continue;
        }
        let t = entries[0]; // march order: first = topmost
        top[ix * nz + iz] = Some(t);
        top_ys.push(t);
        for &y in entries {
            let bin = (y / p.slab_bin_m) as usize;
            if bin < n_bins {
                bins[bin] += 1;
            }
        }
    }

    if top_ys.is_empty() {
        return VerticalScan {
            slabs: vec![],
            floors: vec![0.0],
            eave: 0.0,
            ridge: 0.0,
            chimney: None,
            top,
            top_slope: vec![0.0; nx * nz],
            nx,
            nz,
        };
    }

    top_ys.sort_by(f64::total_cmp);
    let pct = |p_: usize| top_ys[(top_ys.len() * p_ / 100).min(top_ys.len() - 1)];
    let eave = pct(p.eave_pctile);
    let ridge = pct(p.ridge_pctile);
    let peak = *top_ys.last().expect("non-empty");
    let chimney = (peak >= ridge + p.chimney_margin_m).then_some(peak);

    // Slab peaks: support floor, local max over +/-2 bins, min vertical spacing, under the roof.
    let columns_hit = top_ys.len();
    let support = ((columns_hit as f64 * p.slab_support_frac) as usize).max(p.slab_support_min);
    let spacing_bins = (p.slab_spacing_m / p.slab_bin_m).round() as i64;
    let mut slabs = Vec::new();
    let mut last_bin: i64 = -1000;
    for b in 0..n_bins {
        if bins[b] < support {
            continue;
        }
        let is_peak = (-2i64..=2).all(|w| {
            let nb = b as i64 + w;
            nb < 0 || nb >= n_bins as i64 || bins[nb as usize] <= bins[b]
        });
        if !is_peak || (b as i64 - last_bin) < spacing_bins {
            continue;
        }
        let slab_y = b as f64 * p.slab_bin_m;
        if slab_y > eave + p.slab_above_eave_m {
            continue;
        }
        slabs.push(slab_y);
        last_bin = b as i64;
    }

    // Live Execute's floor filter (drops foundation-skirt and roof-height returns). NOTE the dump
    // is normalized: local y = origin.y + normalized y, so the live "> -0.5 local" bound shifts
    // by -origin.y — the caller passes min_floor_y already shifted.
    let floors: Vec<f64> = slabs
        .iter()
        .copied()
        .filter(|&s| s > p.min_floor_y && s < eave - p.eave_clearance_m)
        .collect();
    let floors = if floors.is_empty() {
        vec![p.min_floor_y.max(0.0)]
    } else {
        floors
    };

    let mut top_slope = vec![0.0; nx * nz];
    for ix in 0..nx {
        for iz in 0..nz {
            let Some(h) = top[ix * nz + iz] else { continue };
            let sample = |jx: i64, jz: i64| -> f64 {
                if jx < 0 || jz < 0 || jx >= nx as i64 || jz >= nz as i64 {
                    return h;
                }
                top[jx as usize * nz + jz as usize].unwrap_or(h)
            };
            let gx = (sample(ix as i64 + 1, iz as i64) - sample(ix as i64 - 1, iz as i64))
                / (2.0 * cell);
            let gz = (sample(ix as i64, iz as i64 + 1) - sample(ix as i64, iz as i64 - 1))
                / (2.0 * cell);
            top_slope[ix * nz + iz] = (gx * gx + gz * gz).sqrt();
        }
    }

    VerticalScan {
        slabs,
        floors,
        eave,
        ridge,
        chimney,
        top,
        top_slope,
        nx,
        nz,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_blueprint::synth;

    #[test]
    fn box_room_yields_single_ground_slab() {
        let d = synth::box_room(6.0, 4.0, 2.6, 0.15);
        let m = d.meta();
        let p = Params {
            min_floor_y: -0.5 - m.origin[1],
            ..Default::default()
        };
        let v = analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
        assert_eq!(
            v.floors.len(),
            1,
            "floors: {:?} slabs: {:?} eave {}",
            v.floors,
            v.slabs,
            v.eave
        );
        let local_floor = m.origin[1] + v.floors[0];
        assert!(
            local_floor.abs() < 0.15,
            "ground slab near local y=0, got {local_floor}"
        );
        assert!(
            (m.origin[1] + v.eave - 2.6).abs() < 0.2,
            "flat roof reads as eave~2.6"
        );
        assert!(v.chimney.is_none());
    }

    #[test]
    fn gable_slope_field_flags_roof_planes() {
        let d = synth::gable_box(6.0, 4.0, 2.6, 4.2, 0.15);
        let m = d.meta();
        let p = Params {
            min_floor_y: -0.5 - m.origin[1],
            ..Default::default()
        };
        let v = analyze(&d.y_down, m.dims, m.cell, m.span[1], &p);
        // Mid-span roof cells slope ~ (ridge-eave)/(depth/2) = 1.6/2.0 = 0.8.
        let (ix, iz) = (m.dims[0] / 2, m.dims[2] / 4);
        let s = v.slope_at(ix, iz);
        assert!(
            s > 0.25 && s < 4.0,
            "roof plane slope in veto band, got {s}"
        );
        assert!(
            v.ridge > v.eave + 1.0,
            "gable ridge above eave: {} vs {}",
            v.ridge,
            v.eave
        );
    }
}
