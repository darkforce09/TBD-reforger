//! Role: peaks.
//! Position: `environment/locations` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::terrain::dem::manifest::DemManifest;

/// Local-max search window (px) — ≈18 m on Everon 2 m/px grid.
pub const PEAK_WINDOW_PX: usize = 9;

/// Minimum prominence above the window minimum (m).
pub const PEAK_PROMINENCE_M: f64 = 15.0;

/// Max labels after declutter (Everon cap).
pub const PEAK_LABEL_MAX: usize = 48;

/// Canonical height label screen pitch px value.
pub const HEIGHT_LABEL_SCREEN_PITCH_PX: f64 = 150.0;

/// Canonical height label min sep m value.
pub const HEIGHT_LABEL_MIN_SEP_M: f64 = HEIGHT_LABEL_SCREEN_PITCH_PX;

/// Canonical peak min value m value.
pub const PEAK_MIN_VALUE_M: i32 = 80;

/// Canonical height label min zoom value.
pub const HEIGHT_LABEL_MIN_ZOOM: f64 = -2.0;

/// See [`HEIGHT_LABEL_MIN_ZOOM`].
pub const HEIGHT_LABEL_MAX_ZOOM: f64 = 3.0;

/// Height label kind (peak vs optional contour index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeightLabelKind {
    /// Peak.
    Peak,

    /// Contour.
    Contour,
}

/// One height marker anchor in world meters.
#[derive(Clone, Debug, PartialEq)]
pub struct HeightLabel {
    /// X.
    pub x: f64,

    /// Y.
    pub y: f64,

    /// Value m.
    pub value_m: i32,

    /// Kind.
    pub kind: HeightLabelKind,

    /// Name.
    pub name: Option<String>,
}

/// Because the declutter separation scales as `2^(−deck_zoom)` while the camera scales as `2^(+deck_zoom)`, the kept labels sit a **fixed** `PITCH_PX` apart on screen at every zoom — i.e. the on-screen label density is constant (≈1 per `PITCH×PITCH` px), the Eden-parity behaviour.
#[must_use]
pub fn height_label_screen_sep_world_m(deck_zoom: f64) -> f64 {
    HEIGHT_LABEL_SCREEN_PITCH_PX * 2f64.powf(-deck_zoom)
}

/// Height label min sep m.
#[must_use]
pub fn height_label_min_sep_m(deck_zoom: f64) -> f64 {
    height_label_screen_sep_world_m(deck_zoom)
}

/// Should draw height label.
#[must_use]
pub fn should_draw_height_label(deck_zoom: f64) -> bool {
    (HEIGHT_LABEL_MIN_ZOOM..=HEIGHT_LABEL_MAX_ZOOM).contains(&deck_zoom)
}

/// Pixel center → world (x, z) meters.
#[must_use]
pub fn pixel_to_world(px: usize, py: usize, m: &DemManifest) -> (f64, f64) {
    let w = m.width_px.saturating_sub(1).max(1) as f64;
    let h = m.height_px.saturating_sub(1).max(1) as f64;
    let mut u = px as f64 / w;
    let mut v = py as f64 / h;
    if m.flip_x {
        u = 1.0 - u;
    }
    if m.flip_z {
        v = 1.0 - v;
    }
    let x = m.min_x + u * (m.max_x - m.min_x);
    let z = m.min_y + v * (m.max_y - m.min_y);
    (x, z)
}

fn elev_at(meters: &[f32], width: usize, px: usize, py: usize) -> f64 {
    f64::from(meters[py * width + px])
}

/// Find peaks.
#[must_use]
pub fn find_peaks(
    meters: &[f32],
    width: usize,
    height: usize,
    m: &DemManifest,
) -> Vec<HeightLabel> {
    if width == 0 || height == 0 || meters.len() < width * height {
        return Vec::new();
    }
    let r = PEAK_WINDOW_PX / 2;
    let mut out = Vec::new();
    let mut global_px = 0usize;
    let mut global_py = 0usize;
    let mut global_e = f64::NEG_INFINITY;
    for py in 0..height {
        for px in 0..width {
            let e = elev_at(meters, width, px, py);
            if e > global_e {
                global_e = e;
                global_px = px;
                global_py = py;
            }
        }
    }
    for py in r..height.saturating_sub(r) {
        for px in r..width.saturating_sub(r) {
            let center = elev_at(meters, width, px, py);
            if center <= 0.0 {
                continue;
            }
            let mut is_max = true;
            let mut win_min = f64::MAX;
            for dy in 0..PEAK_WINDOW_PX {
                for dx in 0..PEAK_WINDOW_PX {
                    let nx = px + dx - r;
                    let ny = py + dy - r;
                    let e = elev_at(meters, width, nx, ny);
                    if e < win_min {
                        win_min = e;
                    }
                    if (dx != r || dy != r) && e > center {
                        is_max = false;
                    }
                }
            }
            if !is_max {
                continue;
            }
            if center - win_min < PEAK_PROMINENCE_M {
                continue;
            }
            let value_m = center.round() as i32;

            if value_m < PEAK_MIN_VALUE_M {
                continue;
            }
            let (x, y) = pixel_to_world(px, py, m);
            out.push(HeightLabel {
                x,
                y,
                value_m,
                kind: HeightLabelKind::Peak,
                name: None,
            });
        }
    }

    let g_val = global_e.round() as i32;
    if global_e > 0.0 && g_val >= PEAK_MIN_VALUE_M {
        let (gx, gy) = pixel_to_world(global_px, global_py, m);
        let already = out
            .iter()
            .any(|p| (p.x - gx).abs() < 1.0 && (p.y - gy).abs() < 1.0 && p.value_m == g_val);
        if !already {
            out.push(HeightLabel {
                x: gx,
                y: gy,
                value_m: g_val,
                kind: HeightLabelKind::Peak,
                name: None,
            });
        }
    }
    out
}

fn dist_m(a: &HeightLabel, b: &HeightLabel) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx.hypot(dy)
}

/// Importance-distance greedy declutter: sort by `value_m` desc; keep iff dist ≥ sep to all kept.
#[must_use]
pub fn declutter_height_labels(labels: &[HeightLabel], deck_zoom: f64) -> Vec<HeightLabel> {
    if !should_draw_height_label(deck_zoom) {
        return Vec::new();
    }
    let sep = height_label_min_sep_m(deck_zoom);
    let mut candidates: Vec<HeightLabel> = labels.to_vec();
    candidates.sort_by_key(|c| std::cmp::Reverse(c.value_m));
    let mut keep: Vec<HeightLabel> = Vec::new();
    for cand in candidates {
        if keep.len() >= PEAK_LABEL_MAX {
            break;
        }
        if keep.iter().all(|k| dist_m(&cand, k) >= sep) {
            keep.push(cand);
        }
    }
    keep
}

/// G4: every kept pair satisfies dist ≥ sep.
#[must_use]
pub fn declutter_invariant_holds(labels: &[HeightLabel], deck_zoom: f64) -> bool {
    let sep = height_label_min_sep_m(deck_zoom);
    for (i, a) in labels.iter().enumerate() {
        for b in labels.iter().skip(i + 1) {
            if dist_m(a, b) < sep {
                return false;
            }
        }
    }
    labels.len() <= PEAK_LABEL_MAX
}

/// Convert height labels to `LabelSpec` for the text lane (importance = elevation).
#[must_use]
pub fn height_labels_to_specs(
    labels: &[HeightLabel],
) -> Vec<crate::symbology::labels::declutter::LabelSpec> {
    labels
        .iter()
        .enumerate()
        .map(|(i, l)| crate::symbology::labels::declutter::LabelSpec {
            id: i as u32,
            x: l.x.round() as i32,
            y: l.y.round() as i32,
            importance: l.value_m.clamp(0, i32::from(u16::MAX)) as u16,
            text: match &l.name {
                Some(n) => format!("{n} - {} m", l.value_m),
                None => l.value_m.to_string(),
            },
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/peaks_tests.rs"]
mod tests;
