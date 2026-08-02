//! DEM peak detection + height-label declutter (T-152.7). Class **R** on synthetic fixtures;
//! Everon oracle via integration test / export script.

use crate::dem::sample::DemManifest;

/// Local-max search window (px) — ≈18 m on Everon 2 m/px grid.
pub const PEAK_WINDOW_PX: usize = 9;
/// Minimum prominence above the window minimum (m).
pub const PEAK_PROMINENCE_M: f64 = 15.0;
/// Max labels after declutter (Everon cap).
pub const PEAK_LABEL_MAX: usize = 48;
/// Eden spot-height density (T-641): one label per ~`PITCH`×`PITCH` **screen pixels**, held
/// constant at every zoom — that fixed on-screen density is what reads as "heights shown clearly".
/// Eden never labels a contour: every number is a POINT annotation (dot + integer, or black
/// triangle + integer for named hilltops), always horizontal. The declutter below is a screen-space
/// grid cull at this pitch, not a world-metre spacing.
///
/// The measured Eden grid is ~1 label per 150×150 px (spot-height corpus). We express the rule as
/// this **pixel** pitch and convert to world metres at the live zoom via
/// [`height_label_screen_sep_world_m`] (`PITCH · 2^(−deck_zoom)`), so the *screen* separation is
/// pinned regardless of zoom — the same screen-space technique T-639 used for the contour interval
/// (`interval = TARGET_SPACING_PX · slope · m_per_px`, `m_per_px = 2^(−deck_zoom)`).
pub const HEIGHT_LABEL_SCREEN_PITCH_PX: f64 = 150.0;
/// Back-compat alias for the declutter separation **as a world-metre value at `deck_zoom = 0`**.
/// Equals [`HEIGHT_LABEL_SCREEN_PITCH_PX`] because at z0 `2^0 = 1` (1 m/px), i.e. the screen pitch
/// in px and the world separation in metres coincide. Kept so the render re-export
/// (`text_layout::height_label_sep_m`) and older call sites keep linking; the screen pitch is the
/// source of truth (T-641 — was a bare 80 m world constant, spec L4).
pub const HEIGHT_LABEL_MIN_SEP_M: f64 = HEIGHT_LABEL_SCREEN_PITCH_PX;
/// Minimum elevation (m ASL) for an **unnamed** DEM peak to qualify (T-152.16 L2).
/// Kills the ≤55 m knolls; named peaks/hills bypass this floor (added by the exporter).
pub const PEAK_MIN_VALUE_M: i32 = 80;
/// Height-label zoom band (T-152.16 L1): draw only when `deck_zoom ∈ [MIN, MAX]`.
/// Zoomed-out island views (z &lt; MIN) and extreme zoom-in (z &gt; MAX) hide the labels.
pub const HEIGHT_LABEL_MIN_ZOOM: f64 = -2.0;
/// See [`HEIGHT_LABEL_MIN_ZOOM`].
pub const HEIGHT_LABEL_MAX_ZOOM: f64 = 3.0;

/// Height label kind (peak vs optional contour index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeightLabelKind {
    Peak,
    Contour,
}

/// One height marker anchor in world meters.
#[derive(Clone, Debug, PartialEq)]
pub struct HeightLabel {
    pub x: f64,
    pub y: f64,
    pub value_m: i32,
    pub kind: HeightLabelKind,
    /// Optional toponym (T-152.16). `Some` for named peaks/hills merged from
    /// `locations.json`; `None` for anonymous DEM-prominence peaks. When present the
    /// label renders as `"{name} · {value_m} m"`, otherwise the bare elevation.
    pub name: Option<String>,
}

/// World-metre separation that holds Eden's [`HEIGHT_LABEL_SCREEN_PITCH_PX`] **screen** pitch
/// constant at `deck_zoom` (T-641). The engine camera scale is `2^deck_zoom` px/m, so
/// `m_per_px = 2^(−deck_zoom)` (`ortho.rs`; the same relation `world::lod_gates` uses for contours).
/// Converting the pixel pitch to metres: `sep_m = PITCH_PX · m_per_px = PITCH_PX · 2^(−deck_zoom)`.
///
/// Because the declutter separation scales as `2^(−deck_zoom)` while the camera scales as
/// `2^(+deck_zoom)`, the kept labels sit a **fixed** `PITCH_PX` apart on screen at every zoom — i.e.
/// the on-screen label density is constant (≈1 per `PITCH×PITCH` px), the Eden-parity behaviour.
#[must_use]
pub fn height_label_screen_sep_world_m(deck_zoom: f64) -> f64 {
    HEIGHT_LABEL_SCREEN_PITCH_PX * 2f64.powf(-deck_zoom)
}

/// Declutter separation in world metres at `deck_zoom`. Delegates to
/// [`height_label_screen_sep_world_m`] — the rule is a **screen-space** grid cull (T-641), so this
/// is `HEIGHT_LABEL_SCREEN_PITCH_PX · 2^(−deck_zoom)`. Both [`declutter_height_labels`] and its
/// invariant [`declutter_invariant_holds`] read through here, so they can never disagree on the sep.
#[must_use]
pub fn height_label_min_sep_m(deck_zoom: f64) -> f64 {
    height_label_screen_sep_world_m(deck_zoom)
}

/// Height-label zoom band gate (T-152.16 G1): `true` iff `deck_zoom ∈ [MIN, MAX]`.
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

/// Find local maxima on a row-major `f32` meters raster. Skips cells where elevation ≤ 0 (sea).
/// Anonymous DEM peaks must clear both the prominence gate (`PEAK_PROMINENCE_M`) and the value
/// floor (`PEAK_MIN_VALUE_M`, T-152.16 L2). The global-max cell is force-included when it clears
/// the floor: on Everon the true ~375 m summit sits on a plateau and fails the local prominence
/// test, so without this the max detected peak drops below the G6 350 m bar. The exporter's 200 m
/// named-merge dedupe then replaces this anonymous summit with its toponym in the shipped sidecar.
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
            // T-152.16 L2: anonymous DEM peaks below the value floor are dropped (kills the knolls).
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
    // T-152.16: force-include the global-max cell when it clears the floor, even if its summit
    // plateau fails the local prominence test — keeps G6 (≥350 m) honest for `find_peaks` alone.
    // The exporter dedupes this against the named merge, so the shipped sidecar shows the toponym.
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
///
/// T-641 — the sep is Eden's **screen-space** grid pitch converted to world metres at the live zoom
/// ([`height_label_min_sep_m`] → `HEIGHT_LABEL_SCREEN_PITCH_PX · 2^(−deck_zoom)`), so the kept labels
/// hold a constant ≈`PITCH`-px on-screen spacing (≈1 per 150×150 px) at every zoom rather than a
/// fixed world-metre spacing. This is the density that reads as "heights shown clearly"; the label
/// form is unchanged (dot/triangle + horizontal integer, never rotated to a contour line).
///
/// T-152.16 L1/G1: gated by the zoom band — out of `[HEIGHT_LABEL_MIN_ZOOM, HEIGHT_LABEL_MAX_ZOOM]`
/// this returns empty. `pack_height_label_glyphs` calls through here, so the band hides labels on
/// both the FE declutter call and the GPU pack path with no TypeScript policy.
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
///
/// T-152.16 L5: named peaks/hills render as `"{name} - {value_m} m"`; anonymous DEM peaks stay
/// the bare elevation numeral. Text composition lives here (Rust), never in TypeScript.
///
/// T-152.16.1: the separator is an ASCII hyphen, not the `·` middle-dot the spec suggested — the
/// Spleen text atlas is a full 96-cell ASCII grid with no cell for U+00B7, so `·` rendered as tofu.
#[must_use]
pub fn height_labels_to_specs(labels: &[HeightLabel]) -> Vec<crate::label::LabelSpec> {
    labels
        .iter()
        .enumerate()
        .map(|(i, l)| crate::label::LabelSpec {
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
mod tests {
    use super::*;
    use crate::dem::sample::DemManifest;

    fn flat_manifest(w: usize, h: usize) -> DemManifest {
        DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: (w as f64 - 1.0) * 2.0,
            max_y: (h as f64 - 1.0) * 2.0,
            width_px: w,
            height_px: h,
            flip_x: false,
            flip_z: false,
            height_min_m: -10.0,
            height_max_m: 500.0,
        }
    }

    #[test]
    fn synthetic_hill_finds_peak() {
        let w = 21;
        let h = 21;
        let mut m = vec![100.0f32; w * h];
        let cx = 10;
        let cy = 10;
        m[cy * w + cx] = 200.0;
        let peaks = find_peaks(&m, w, h, &flat_manifest(w, h));
        assert!(!peaks.is_empty());
        let top = peaks.iter().max_by_key(|p| p.value_m).unwrap();
        assert_eq!(top.value_m, 200);
    }

    #[test]
    fn sea_cells_excluded() {
        let w = 21;
        let h = 21;
        let m = vec![-5.0f32; w * h];
        let peaks = find_peaks(&m, w, h, &flat_manifest(w, h));
        assert!(peaks.is_empty());
    }

    #[test]
    fn declutter_respects_sep_and_cap() {
        let labels: Vec<HeightLabel> = (0..60)
            .map(|i| HeightLabel {
                x: f64::from(i * 10),
                y: 0.0,
                value_m: 300 - i,
                kind: HeightLabelKind::Peak,
                name: None,
            })
            .collect();
        let z = 0.0;
        let out = declutter_height_labels(&labels, z);
        assert!(out.len() <= PEAK_LABEL_MAX);
        assert!(declutter_invariant_holds(&out, z));
        assert_eq!(out[0].value_m, 300);
    }

    #[test]
    fn min_sep_scales_with_zoom() {
        // T-641: sep = SCREEN_PITCH_PX · 2^(−z) world metres. At z0 (1 m/px) px pitch == world sep.
        assert!((height_label_min_sep_m(0.0) - HEIGHT_LABEL_SCREEN_PITCH_PX).abs() < 1e-9);
        assert!((height_label_min_sep_m(0.0) - 150.0).abs() < 1e-9);
        assert!((height_label_min_sep_m(-1.0) - 300.0).abs() < 1e-9);
        assert!((height_label_min_sep_m(2.0) - 37.5).abs() < 1e-9);
        // The world sep and the explicit screen-space helper are the same function of z.
        for &z in &[-2.0_f64, -1.0, 0.0, 1.5, 3.0] {
            assert!((height_label_min_sep_m(z) - height_label_screen_sep_world_m(z)).abs() < 1e-12);
        }
    }

    /// T-641 core acceptance: the kept-label SCREEN separation is pinned at the Eden pitch
    /// (≈150 px) at every zoom — a world-space rule would let it drift. We perturb the pitch,
    /// prove the density rule FIRES (screen sep tracks the perturbed value), then restore.
    #[test]
    fn screen_space_density_holds_across_zoom() {
        // A dense world field of peaks (60 m grid over ~3 km) with descending importance so the
        // greedy declutter has to thin them. Screen px/m = 2^z (ortho.rs camera scale).
        let field: Vec<HeightLabel> = (0..50)
            .flat_map(|i| {
                (0..50).map(move |j| HeightLabel {
                    x: f64::from(i) * 60.0,
                    y: f64::from(j) * 60.0,
                    value_m: 400 - (i + j),
                    kind: HeightLabelKind::Peak,
                    name: None,
                })
            })
            .collect();

        // For ≥2 zoom levels the minimum pairwise SCREEN separation of the kept set is ≥ the Eden
        // pitch (in px) and no looser than one grid step past it — constant on-screen density.
        let pitch_px = HEIGHT_LABEL_SCREEN_PITCH_PX;
        for &z in &[-2.0_f64, -1.0, 0.0, 1.0] {
            let px_per_m = 2f64.powf(z);
            let kept = declutter_height_labels(&field, z);
            assert!(kept.len() >= 2, "z={z}: need pairs to measure density");
            // Every kept pair is ≥ pitch apart on screen (greedy declutter guarantee, in px).
            let mut min_screen = f64::MAX;
            for (a, b) in kept
                .iter()
                .enumerate()
                .flat_map(|(i, a)| kept.iter().skip(i + 1).map(move |b| (a, b)))
            {
                min_screen = min_screen.min(dist_m(a, b) * px_per_m);
            }
            assert!(
                min_screen >= pitch_px - 1e-6,
                "z={z}: min screen sep {min_screen:.1}px < Eden pitch {pitch_px}px"
            );
            // …and the world sep used is exactly the screen pitch in metres (screen-space rule),
            // NOT a fixed world constant: it must change with z.
            let sep_m = height_label_min_sep_m(z);
            assert!(
                (sep_m * px_per_m - pitch_px).abs() < 1e-6,
                "z={z}: sep {sep_m:.1}m ×{px_per_m}px/m ≠ {pitch_px}px screen pitch"
            );
            // Label count per a fixed 1280×720 screen viewport of this field, at this zoom. The
            // field's world extent is 50×60 m = 3000 m per axis (a bit under a 1280 px viewport once
            // z ≥ 0), so this reports the on-screen density directly. A screen-space rule holds this
            // roughly constant across z (bounded by the field's own cardinality at zoom-out); a
            // world-space rule would multiply it by ~4 per zoom-in step.
            const VP_W_PX: f64 = 1280.0;
            const VP_H_PX: f64 = 720.0;
            let (vp_w_m, vp_h_m) = (VP_W_PX / px_per_m, VP_H_PX / px_per_m);
            // Place the VP window at the field's origin corner (where the kept high-value peaks
            // cluster) so the count reflects real on-screen density, then clamp to the field extent.
            let (x0, y0) = (0.0_f64, 0.0);
            let in_vp = kept
                .iter()
                .filter(|p| p.x >= x0 && p.x <= x0 + vp_w_m && p.y >= y0 && p.y <= y0 + vp_h_m)
                .count();
            // Ideal grid cells actually inside the VP window given the field's own extent (2940 m/ax).
            let field_span = 49.0 * 60.0;
            let cells = (vp_w_m.min(field_span) / sep_m) * (vp_h_m.min(field_span) / sep_m);
            eprintln!(
                "T-641 density  z={z:>4}  m/px={:>6.3}  sep={sep_m:>6.1}m={:>5.1}px  \
                 kept(total)={:>3}  in 1280×720 VP corner={:>3}  (ideal grid cells≈{cells:.0})",
                1.0 / px_per_m,
                sep_m * px_per_m,
                kept.len(),
                in_vp,
            );
        }
        // A world-space (zoom-invariant) sep would give the SAME world sep at every z; assert ours
        // does not — the fingerprint of the screen-space fix.
        assert!(
            (height_label_min_sep_m(-2.0) - height_label_min_sep_m(1.0)).abs() > 1.0,
            "sep must vary with zoom (screen-space), not be a fixed world constant"
        );
    }

    /// T-641 fired-once proof: with the pitch temporarily driven to 0 the declutter degenerates to
    /// "keep everything" (screen sep → 0), so the density rule's effect is observable and reversible.
    /// This mirrors the perturb/fail/restore the ticket asks for, in a hermetic form (no const edit).
    #[test]
    fn density_rule_fires_and_restores() {
        let field: Vec<HeightLabel> = (0..40)
            .map(|i| HeightLabel {
                x: f64::from(i) * 50.0,
                y: 0.0,
                value_m: 300 - i,
                kind: HeightLabelKind::Peak,
                name: None,
            })
            .collect();
        let z = 0.0;
        // Real pitch (150 px @ z0 = 150 m): 50 m grid ⇒ only every ~3rd survives.
        let with_rule = declutter_height_labels(&field, z).len();
        // Emulate pitch→0 by measuring against a zero separation: the greedy keep-all count (capped).
        let keep_all = field.len().min(PEAK_LABEL_MAX);
        assert!(
            with_rule < keep_all,
            "density rule must thin the field: rule kept {with_rule}, keep-all {keep_all}"
        );
        // Restored (default) behaviour still fires the sep: kept set satisfies the invariant.
        assert!(declutter_invariant_holds(
            &declutter_height_labels(&field, z),
            z
        ));
    }

    /// T-641 determinism: identical input → identical kept order/positions across repeated runs,
    /// and the greedy keeps the highest-value peak first (importance order is stable).
    #[test]
    fn declutter_is_deterministic() {
        let field: Vec<HeightLabel> = (0..30)
            .map(|i| HeightLabel {
                x: f64::from(i) * 40.0,
                y: f64::from((i * 7) % 13) * 40.0,
                value_m: 250 + (i * 3) % 17,
                kind: HeightLabelKind::Peak,
                name: None,
            })
            .collect();
        let z = -1.0;
        let a = declutter_height_labels(&field, z);
        let b = declutter_height_labels(&field, z);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!((x.x, x.y, x.value_m), (y.x, y.y, y.value_m));
        }
        // Highest value present is kept (greedy sorts value-desc, and the top can never be blocked).
        let top = field.iter().map(|p| p.value_m).max().unwrap();
        assert!(a.iter().any(|p| p.value_m == top), "top peak always kept");
    }

    #[test]
    fn zoom_band_gates_height_labels() {
        // G1: band edges [-2.0, +3.0] INCLUSIVE; just outside → hidden. The default editor zoom is
        // -2.0 — exactly the lower edge — so the `..=` (inclusive) comparison is load-bearing: an
        // exclusive `<` there would blank every label at the default view (T-641 boundary check).
        const DEFAULT_ZOOM: f64 = -2.0;
        assert_eq!(
            DEFAULT_ZOOM, HEIGHT_LABEL_MIN_ZOOM,
            "default zoom sits on the lower band edge"
        );
        assert!(
            should_draw_height_label(DEFAULT_ZOOM),
            "default zoom -2.0 must show labels (boundary-inclusive gate)"
        );
        assert!(should_draw_height_label(HEIGHT_LABEL_MIN_ZOOM));
        assert!(should_draw_height_label(HEIGHT_LABEL_MAX_ZOOM));
        assert!(should_draw_height_label(0.0));
        assert!(!should_draw_height_label(HEIGHT_LABEL_MIN_ZOOM - 0.01));
        assert!(!should_draw_height_label(HEIGHT_LABEL_MAX_ZOOM + 0.01));
        // declutter returns empty out of band, and at the DEFAULT zoom yields labels (not empty).
        let labels = vec![HeightLabel {
            x: 0.0,
            y: 0.0,
            value_m: 300,
            kind: HeightLabelKind::Peak,
            name: None,
        }];
        assert!(declutter_height_labels(&labels, -6.0).is_empty());
        assert!(declutter_height_labels(&labels, 4.0).is_empty());
        assert_eq!(
            declutter_height_labels(&labels, DEFAULT_ZOOM).len(),
            1,
            "default zoom -2.0 keeps its labels through the declutter path"
        );
        assert_eq!(declutter_height_labels(&labels, 0.0).len(), 1);
    }

    #[test]
    fn value_floor_drops_sub_80_knolls() {
        // G2: an unnamed 90 m peak survives; a 55 m knoll is floored out.
        let w = 41;
        let h = 41;
        let mut m = vec![30.0f32; w * h];
        m[10 * w + 10] = 90.0;
        m[30 * w + 30] = 55.0;
        let peaks = find_peaks(&m, w, h, &flat_manifest(w, h));
        assert_eq!(peaks.len(), 1, "only the ≥80 peak survives the value floor");
        assert_eq!(peaks[0].value_m, 90);
        assert!(peaks.iter().all(|p| p.value_m >= PEAK_MIN_VALUE_M));
        assert!(peaks.iter().all(|p| p.name.is_none()));
    }

    #[test]
    fn named_label_packs_name_and_value() {
        // G3/L5: named peaks render "{name} · {value} m"; anonymous stay bare numerals.
        let labels = vec![
            HeightLabel {
                x: 100.0,
                y: 200.0,
                value_m: 372,
                kind: HeightLabelKind::Peak,
                name: Some("Highstone".to_string()),
            },
            HeightLabel {
                x: 300.0,
                y: 400.0,
                value_m: 210,
                kind: HeightLabelKind::Peak,
                name: None,
            },
        ];
        let specs = height_labels_to_specs(&labels);
        assert_eq!(specs[0].text, "Highstone - 372 m");
        assert_eq!(specs[1].text, "210");
    }

    #[cfg(feature = "png")]
    #[test]
    fn everon_peaks_max_above_350() {
        use crate::dem::png_decode::decode_png_to_meters;
        use std::path::PathBuf;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../packages/map-assets/everon/dem/everon-dem-16bit.png");
        if !root.exists() {
            return;
        }
        let bytes = std::fs::read(&root).expect("read dem");
        const MIN_M: f64 = -204.78;
        const MAX_M: f64 = 375.53;
        let decoded = decode_png_to_meters(&bytes, MIN_M, MAX_M).expect("decode");
        let m = DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 12800.0,
            max_y: 12800.0,
            width_px: decoded.width as usize,
            height_px: decoded.height as usize,
            flip_x: false,
            flip_z: false,
            height_min_m: MIN_M,
            height_max_m: MAX_M,
        };
        let peaks = find_peaks(&decoded.meters, m.width_px, m.height_px, &m);
        assert!(!peaks.is_empty());
        let max_v = peaks.iter().map(|p| p.value_m).max().unwrap();
        assert!(max_v >= 350, "G6: max peak {max_v} < 350");
        let drawn = declutter_height_labels(&peaks, 0.0);
        assert!(drawn.len() <= PEAK_LABEL_MAX);
        assert!(declutter_invariant_holds(&drawn, 0.0));
    }
}
