//! Spot heights: the hilltops found on a terrain's elevation model, and which of them show at a
//! zoom.
//!
//! **Role:** finds the local maxima of the metres raster that stand out from their window
//! ([`find_peaks`]), declutters them highest first at a zoom ([`declutter_height_labels`]) and
//! turns the kept ones into label specs ([`height_labels_to_specs`]).
//! **Position:** reads the raster placement of `terrain_elevation`; [`crate::label_packing`], the
//! map engine's label loader and the developer tools' height label export and checks read it.
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** a peak is a land cell that no cell of its [`PEAK_WINDOW_PX`] window exceeds,
//! rising [`PEAK_PROMINENCE_M`] above the window's lowest cell and reaching [`PEAK_MIN_VALUE_M`],
//! plus the raster's highest cell; kept labels sit [`HEIGHT_LABEL_SCREEN_PITCH_PX`] apart on
//! screen at every zoom of the band, at most [`PEAK_LABEL_MAX`] of them.

use label_layout::declutter::LabelSpec;
use label_layout::label_ids::LabelId;
use terrain_elevation::manifest::DemManifest;

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
    /// A hilltop spot height: every one [`find_peaks`] yields or the labels archive holds, and
    /// every `height-labels.json` row whose `kind` is not `contour`.
    Peak,

    /// A contour index height, from a `height-labels.json` row whose `kind` is `contour`.
    Contour,
}

/// One height marker anchor in world meters.
#[derive(Clone, Debug, PartialEq)]
pub struct HeightLabel {
    /// Marker position along world X, metres.
    pub x: f64,

    /// Marker position along map Y (world Z, as [`pixel_to_world`] returns it), metres.
    pub y: f64,

    /// Elevation the label shows, rounded to whole metres; it is also the label's importance, so
    /// the declutter keeps higher markers first.
    pub value_m: i32,

    /// Whether the marker is a hilltop or a contour index.
    pub kind: HeightLabelKind,

    /// Hill name from `height-labels.json` (`name`); a named label reads `{name} - {value} m`,
    /// an unnamed one only the value. `None` for raster peaks and archive rows.
    pub name: Option<String>,
}

/// Because the declutter separation scales as `2^(−deck_zoom)` while the camera scales as `2^(+deck_zoom)`, the kept labels sit a **fixed** `PITCH_PX` apart on screen at every zoom — i.e. the on-screen label density is constant (≈1 per `PITCH×PITCH` px), the Eden-parity behaviour.
#[must_use]
pub fn height_label_screen_sep_world_m(deck_zoom: f64) -> f64 {
    HEIGHT_LABEL_SCREEN_PITCH_PX * 2f64.powf(-deck_zoom)
}

/// Minimum world distance in metres between two kept height labels at `deck_zoom`, so they sit
/// [`HEIGHT_LABEL_SCREEN_PITCH_PX`] apart on screen ([`height_label_screen_sep_world_m`]).
#[must_use]
pub fn height_label_min_sep_m(deck_zoom: f64) -> f64 {
    height_label_screen_sep_world_m(deck_zoom)
}

/// Whether height labels draw at `deck_zoom`: inside [`HEIGHT_LABEL_MIN_ZOOM`] to
/// [`HEIGHT_LABEL_MAX_ZOOM`], both ends included; false for `NaN`.
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

/// Spot heights of a row-major `width × height` metres raster, placed in world metres through
/// `m`: every land cell no cell of its [`PEAK_WINDOW_PX`] window exceeds, rising
/// [`PEAK_PROMINENCE_M`] above the window's lowest cell and rounding to at least
/// [`PEAK_MIN_VALUE_M`], plus the raster's highest cell when it reaches that height and is not
/// already listed. Cells closer than half a window to the edge are never local maxima; empty
/// when either dimension is 0 or `meters` is shorter than the raster.
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
pub fn height_labels_to_specs(labels: &[HeightLabel]) -> Vec<LabelSpec> {
    labels
        .iter()
        .enumerate()
        .map(|(i, l)| LabelSpec {
            id: LabelId::new(i as u32),
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
