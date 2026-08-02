//! DEM sampling math — **Class R** (bit-identical to the TS reference
//! `apps/website/frontend/src/features/tactical-map/dem/sampleElevation.ts` and the parity oracle
//! `packages/tbd-schema/scripts/lib/dem-sample.mjs`).
//!
//! Arithmetic is `f64` in the same operation order as the JS, cast `as f32` at the same store
//! boundary as the JS `Float32Array` write, so buffer outputs compare `memcmp`-equal. `bilinear_sample`
//! is generic over the raster element type: the JS reads every sample as an f64 regardless of the
//! backing `Float64Array`(uint16 anchor path) or `Float32Array`(runtime meters), and `u16 → f64`
//! and `f32 → f64` are both exact, so a generic `Into<f64>` accessor reproduces the JS exactly.

/// DEM raster geometry + encoding — the fields of a `TerrainManifest.dem` the sampler needs.
/// Kept as plain scalars so `map-engine-core` stays serde-free; the wasm shim / backend map their
/// manifest onto this.
#[derive(Clone, Copy, Debug)]
pub struct DemManifest {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub width_px: usize,
    pub height_px: usize,
    pub flip_x: bool,
    pub flip_z: bool,
    pub height_min_m: f64,
    pub height_max_m: f64,
}

/// Continuous pixel coordinate on the heightmap (mirror of the `worldToPixel` return).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelCoord {
    pub u: f64,
    pub v: f64,
    pub px: f64,
    pub py: f64,
}

/// `uint16`-linear sample → meters ASL (Bohemia Terrain Creation Tool encoding).
/// Mirror of `uint16ToMeters` (`sampleElevation.ts:9`): `minM + (u16/65535)*(maxM - minM)`.
#[inline]
#[must_use]
pub fn uint16_to_meters(u16v: f64, min_m: f64, max_m: f64) -> f64 {
    min_m + (u16v / 65535.0) * (max_m - min_m)
}

/// Vectorized meters cache: a row-major `uint16` raster → `f32` meters. Mirror of
/// `buildMetersCache` (`DemTexture.ts:74`) — `out[i] = uint16ToMeters(raster[i], min, max)` stored
/// into a `Float32Array`. The Phase 0 boundary-proof kernel and the Phase 1 `dem::png` meters-cache
/// core (the Everon raster is 6400² = 40,960,000 samples → 163,840,000 bytes).
#[must_use]
pub fn meters_cache(raster: &[u16], min_m: f64, max_m: f64) -> Vec<f32> {
    raster
        .iter()
        .map(|&u| uint16_to_meters(f64::from(u), min_m, max_m) as f32)
        .collect()
}

/// World meters (x, z) → continuous pixel coords. Mirror of `worldToPixel` (`sampleElevation.ts:17`).
#[must_use]
pub fn world_to_pixel(x: f64, z: f64, m: &DemManifest) -> PixelCoord {
    let w_m = m.max_x - m.min_x;
    let h_m = m.max_y - m.min_y;
    let mut u = (x - m.min_x) / w_m;
    let mut v = (z - m.min_y) / h_m;
    if m.flip_x {
        u = 1.0 - u;
    }
    if m.flip_z {
        v = 1.0 - v;
    }
    PixelCoord {
        u,
        v,
        px: u * (m.width_px as f64 - 1.0),
        py: v * (m.height_px as f64 - 1.0),
    }
}

/// Bilinear sample of a row-major `width × height` raster. Mirror of `bilinearSample`
/// (`sampleElevation.ts:39`) — generic over the element type (`u16` or `f32`), read as `f64`.
/// Caller guarantees `px ∈ [0, width-1]`, `py ∈ [0, height-1]` (see `sample_elevation_meters`).
#[must_use]
pub fn bilinear_sample<T>(raster: &[T], width: usize, height: usize, px: f64, py: f64) -> f64
where
    T: Copy + Into<f64>,
{
    let x0 = px.floor();
    let y0 = py.floor();
    let x0u = x0 as usize;
    let y0u = y0 as usize;
    let x1u = (x0u + 1).min(width - 1);
    let y1u = (y0u + 1).min(height - 1);
    let fx = px - x0;
    let fy = py - y0;
    let at = |y: usize, xx: usize| -> f64 { raster[y * width + xx].into() };
    let v00 = at(y0u, x0u);
    let v10 = at(y0u, x1u);
    let v01 = at(y1u, x0u);
    let v11 = at(y1u, x1u);
    let top = v00 * (1.0 - fx) + v10 * fx;
    let bot = v01 * (1.0 - fx) + v11 * fx;
    top * (1.0 - fy) + bot * fy
}

/// Bilinear on the `uint16` grid, then convert to meters. Mirror of `sampleElevationMeters`
/// (`sampleElevation.ts:67`). `None` on out-of-bounds (the TS throws; the runtime
/// `DemController.sampleElevation` clamps first so it never does).
#[must_use]
pub fn sample_elevation_meters<T>(
    x: f64,
    z: f64,
    m: &DemManifest,
    raster: &[T],
    width: usize,
    height: usize,
) -> Option<f64>
where
    T: Copy + Into<f64>,
{
    let pc = world_to_pixel(x, z, m);
    if pc.px < 0.0 || pc.py < 0.0 || pc.px > width as f64 - 1.0 || pc.py > height as f64 - 1.0 {
        return None;
    }
    let u16v = bilinear_sample(raster, width, height, pc.px, pc.py);
    Some(uint16_to_meters(u16v, m.height_min_m, m.height_max_m))
}

/// Bilinear sample on the **f32 meters cache** (runtime DEM). Mirror of `bilinearSample` on the
/// meters `Float32Array` — no second `uint16_to_meters` pass (that path is for raw u16 rasters).
#[must_use]
pub fn sample_elevation_from_meters_cache(
    x: f64,
    z: f64,
    m: &DemManifest,
    meters: &[f32],
    width: usize,
    height: usize,
) -> Option<f64> {
    let pc = world_to_pixel(x, z, m);
    if pc.px < 0.0 || pc.py < 0.0 || pc.px > width as f64 - 1.0 || pc.py > height as f64 - 1.0 {
        return None;
    }
    Some(bilinear_sample(meters, width, height, pc.px, pc.py))
}

// ── T-643 — segment sampler (the terrain profile between two world points) ───────────────────────
//
// The reusable half of Line of Sight. T-643 (point-to-point ray) and T-644 (viewshed raster, wave
// 110) both need the SAME thing: given two world points, walk the ground between them and read the
// DEM at a bounded step. That walk lives HERE in the core — not in `los_tool` — so the viewshed can
// call it for each of its rays without re-implementing (or drifting from) the sampling policy. The
// occlusion/sight-line math is `los_tool`'s (frontend); this module only produces the profile.
//
// SAMPLER INJECTION (why a closure, not a raster): the point-elevation lookup is passed IN as
// `elev_at(x, y) -> Option<meters>`. That is the one seam that keeps the same walk honest across two
// very different callers WITHOUT this core taking a dependency it should not:
//   * the golden tests + any raw-DEM caller pass a raster-backed closure —
//     `|x, y| sample_elevation_meters(x, y, manifest, raster, w, h)` (the 6400² uint16 path the
//     ticket names) — so the profile is proven bit-for-bit against a controlled raster;
//   * the live editor (and T-644) pass the DEM handle it actually holds — the 8 m downsampled
//     `DemVectorGrid` via `sample_grid_meters` — so no second full-res raster has to be retained
//     just for LoS. Same step policy, same endpoint rule, same off-coverage semantics either way.
// The `manifest` is still a parameter (the ticket's `sample_segment(manifest, from, to, step_m)`
// signature): it bounds the walk to the DEM's world coverage, so a segment leaving the map stops
// contributing samples rather than asking the closure for points it cannot answer.

/// One sample along a terrain profile: `dist_m` is the along-segment distance from `from` (0 at the
/// observer end), `elev_m` is the ground elevation there in metres ASL. Only points the sampler
/// could answer (inside DEM coverage) are emitted — an off-coverage stretch simply has no samples,
/// the honest gap (matching the CUR-Z / ruler-slope em-dash policy) rather than a fabricated 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfileSample {
    /// Along-segment distance from `from`, metres.
    pub dist_m: f64,
    /// Ground elevation at this point, metres ASL.
    pub elev_m: f64,
}

/// Walk the ground from `from` to `to` at ~`step_m` spacing and read the DEM at each step, returning
/// the terrain profile `[(dist_m, elev_m)]`. The reusable segment sampler both LoS (T-643) and the
/// viewshed (T-644) march over.
///
/// **Step policy** — the number of intervals is `ceil(total / step_m)` (at least 1), then the ACTUAL
/// spacing is `total / n` so the endpoints land exactly and the interior samples are evenly spread
/// (a fixed-`step` loop would drop or double the far endpoint). Distances are therefore strictly
/// monotone increasing and the last sample's `dist_m` equals the full segment length. Callers pass
/// `step_m = min(grid spacing, 8 m)` (the ticket's rule) so no ridge between two samples is missed.
///
/// **Endpoints** — both `from` and `to` are sampled (so the observer's and target's own ground are
/// in the profile). If EITHER endpoint (or any interior point) is off DEM coverage — the injected
/// `elev_at` returns `None`, or the point falls outside the manifest's world box — that sample is
/// omitted; the returned vector is the covered subset in order. A fully off-coverage segment yields
/// an empty vector.
///
/// **Degenerate** — a zero-length segment (`from == to` within `1e-9` m) yields a single sample at
/// `dist_m = 0` (if that point is covered), never a divide-by-zero.
///
/// `elev_at(x, y)` is the point-elevation lookup (see the module note): raster-backed in tests, grid
/// backed in the live editor. `manifest` bounds the walk to DEM coverage.
#[must_use]
pub fn sample_segment<F>(
    manifest: &DemManifest,
    from: (f64, f64),
    to: (f64, f64),
    step_m: f64,
    elev_at: F,
) -> Vec<ProfileSample>
where
    F: Fn(f64, f64) -> Option<f64>,
{
    let (fx, fy) = from;
    let (tx, ty) = to;
    let dx = tx - fx;
    let dy = ty - fy;
    let total = (dx * dx + dy * dy).sqrt();

    // Zero-length segment (from == to): a SINGLE sample at dist 0 (if covered) — the two endpoints
    // are the same point, so emitting both would be a duplicate. Handled before the interval walk so
    // there is no divide-by-zero and no coincident pair.
    if total <= 1e-9 {
        if in_coverage(manifest, fx, fy)
            && let Some(elev_m) = elev_at(fx, fy)
        {
            return vec![ProfileSample {
                dist_m: 0.0,
                elev_m,
            }];
        }
        return Vec::new();
    }

    // Interval count: at least one; a non-finite/≤0 step degenerates to a single interval so the
    // walk still visits both (distinct) endpoints rather than looping forever or dividing by zero.
    let n: usize = if !step_m.is_finite() || step_m <= 0.0 {
        1
    } else {
        (total / step_m).ceil().max(1.0) as usize
    };

    let mut out = Vec::with_capacity(n + 1);
    for i in 0..=n {
        // Parameter t in [0, 1]; t*total is the exact along-segment distance (endpoints exact).
        let t = i as f64 / n as f64;
        let x = fx + dx * t;
        let y = fy + dy * t;
        // Bound to DEM coverage first (the manifest's world box), then ask the sampler. Either gate
        // failing drops this sample — no fabricated elevation off the map.
        if in_coverage(manifest, x, y)
            && let Some(elev_m) = elev_at(x, y)
        {
            out.push(ProfileSample {
                dist_m: t * total,
                elev_m,
            });
        }
    }
    out
}

/// True when world `(x, y)` lies inside the manifest's world coverage box `[min_x, max_x] ×
/// [min_y, max_y]` (inclusive). The world-box guard for [`sample_segment`] — kept separate from the
/// pixel-domain guard the point samplers do so an off-map stretch is dropped before the injected
/// sampler (which may be grid- or raster-backed) is even consulted.
#[inline]
#[must_use]
pub fn in_coverage(m: &DemManifest, x: f64, y: f64) -> bool {
    x >= m.min_x && x <= m.max_x && y >= m.min_y && y <= m.max_y
}

// ── T-644 — viewshed raster (the radial variant the LoS ray's sampler feeds) ─────────────────────
//
// Line of Sight, T-643, answered ONE ray: observer → target, clear or blocked. T-644 answers the
// whole disc: pick an observer, and for EVERY cell within a radius decide whether the observer can
// see it. The compute is a radial ray-march — cast rays out from the observer at a fine angular
// step and, along each ray, track the highest sight-line ANGLE seen so far; a cell is visible iff its
// own elevation angle (from the observer's eye) clears every closer cell on that ray. This is the
// classic "reference-plane / running max-angle" viewshed, and it reuses the SAME injected point
// sampler seam as `sample_segment` (raster-backed in tests, the 8 m `DemVectorGrid` in the editor),
// so the two LoS tools share one sampling policy and never drift.
//
// WHY RADIAL, NOT PER-CELL `occlusion()`: running `occlusion()` (a full segment walk) for each of
// the ~N² cells in the disc is O(N³) — a 2000 m radius at 8 m cells is a 500-cell radius, ~785k
// cells, each walking up to 500 samples ⇒ ~400M sampler calls, far over the ~100 ms budget. The
// radial march visits each ray's samples once (O(rays × steps)) and carries the occluding horizon
// forward as a single running max angle, so it is O(N) in the disc area. Rays are spaced so adjacent
// rays are ≤½ cell apart at the RIM and each ray steps ½ cell (a 2× oversample over the ticket's
// ≤1-cell floor) so NO interior cell is skipped between rays and no ridge between two along-ray
// samples is missed; each sample's result is splatted to the nearest raster cell (a cell may be hit
// by several samples — a Visible verdict from any ray wins, and the running-max march makes rays
// through a cell agree on open terrain; the documented 8 m-grid caveat below covers the rest).
//
// THE WAVE-109 ANCHOR FIX (binding constraint 1): T-643's `occlusion()` anchors the observer eye at
// the profile's FIRST COVERED sample. For a single ray whose observer end is off coverage but whose
// head descends, that seeds the sight line too LOW and reports a false BLOCKED. Here the eye is
// anchored at the OBSERVER's OWN ground elevation + eye height, passed in as `observer_ground_m`
// (the caller reads it at the true observer point). If the observer point itself is off coverage the
// whole raster is `Unknown` (there is no honest eye to cast from). Cells the ray reaches that are
// off coverage are marked `Unknown` (not visible, not blocked) and do NOT advance the occluding
// horizon — an unknown gap can neither reveal nor hide what is beyond it.
//
// THE 8 m-GRID CAVEAT (binding constraint 2): the live grid is the 8 m box-averaged `DemVectorGrid`,
// which is systematically OPTIMISTIC on knife crests versus the raw 2 m raster (a box average lowers
// a sharp ridge, so a sight line grazes over a crest the real terrain would block). This viewshed
// inherits that caveat verbatim — it is a PLANNER'S visibility, not a survey guarantee; do not read
// a `Visible` cell as "provably in the clear" on a razor ridge.

/// The visibility class of one viewshed cell (T-644). Three states, never a fake binary: `Unknown`
/// is a first-class verdict (off coverage), the em-dash policy the LoS ray already uses for an
/// un-judgeable sight — an off-coverage cell is rendered as hidden (constraint 1), never as visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    /// The observer's eye clears every closer cell on the ray to here — the cell is seen.
    Visible,
    /// A closer ridge rises above the sight line to here — the cell is in dead ground.
    Hidden,
    /// The cell (or the observer) is off DEM coverage, so visibility cannot be judged. Rendered as
    /// hidden (a possibly-different alpha; see `los_tool`), NEVER as a fabricated `Visible`.
    Unknown,
}

/// A computed viewshed: a `cols × rows` row-major grid of [`Visibility`] over the world rect
/// `[min_x, min_y]..[max_x, max_y]`, plus the observer world point it was cast from. The raster
/// dimensions match the DEM grid the compute marched (8 m cells in the live editor), so the frontend
/// can turn it straight into an RGBA texture over the same world rect.
#[derive(Clone, Debug, PartialEq)]
pub struct Viewshed {
    pub cols: usize,
    pub rows: usize,
    /// Row-major, `cols * rows` entries.
    pub cells: Vec<Visibility>,
    /// World-space rect the raster covers (cell centres span the inclusive endpoints).
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    /// The observer world point (for the overlay's observer dot + re-compute keying).
    pub obs_x: f64,
    pub obs_y: f64,
}

impl Viewshed {
    /// The visibility at cell `(col, row)`, or [`Visibility::Unknown`] out of bounds.
    #[must_use]
    pub fn at(&self, col: usize, row: usize) -> Visibility {
        if col >= self.cols || row >= self.rows {
            return Visibility::Unknown;
        }
        self.cells[row * self.cols + col]
    }

    /// Count of cells in each class — `(visible, hidden, unknown)`. Sums to `cols * rows`. The
    /// coverage-completeness check for the radial test (every in-radius cell is classified).
    #[must_use]
    pub fn class_counts(&self) -> (usize, usize, usize) {
        let (mut v, mut h, mut u) = (0usize, 0usize, 0usize);
        for c in &self.cells {
            match c {
                Visibility::Visible => v += 1,
                Visibility::Hidden => h += 1,
                Visibility::Unknown => u += 1,
            }
        }
        (v, h, u)
    }
}

/// Parameters for [`compute_viewshed`]. Grouped in a struct so a preset (a taller observer eye, a
/// different radius) is a field change, not a churny argument list, and so the radial-step and cell
/// policy are documented in one place.
#[derive(Clone, Copy, Debug)]
pub struct ViewshedParams {
    /// Observer world point.
    pub obs_x: f64,
    pub obs_y: f64,
    /// Observer GROUND elevation at that point, metres ASL — the wave-109 anchor. `None` ⇒ the
    /// observer is off coverage and the whole raster is `Unknown` (no honest eye to cast from).
    pub observer_ground_m: Option<f64>,
    /// Eye height above the observer's ground, metres (the LoS `EYE_HEIGHT_OBSERVER_M`).
    pub eye_height_m: f64,
    /// Sight radius, metres (default 2000; adjustable).
    pub radius_m: f64,
    /// Raster cell size, metres — the DEM grid spacing (8 m live). Drives both the raster dims and
    /// the ray step (one cell per march step) + angular step (≤1 cell apart at the rim).
    pub cell_m: f64,
}

/// Default sight radius, metres (T-644). The compute runs ONCE per observer placement, so this is a
/// planning horizon, not a per-frame cost; 2000 m is a rifle-to-vehicle spotting reach on Everon and
/// keeps the disc under the ~100 ms budget at 8 m cells (see the module perf note + the reported
/// measurement in the ticket).
pub const VIEWSHED_DEFAULT_RADIUS_M: f64 = 2000.0;

/// Compute a viewshed raster by radial ray-march from an observer (T-644). Returns a `cols × rows`
/// [`Visibility`] grid over the world rect around the observer, clamped to the manifest's coverage
/// box; cells off coverage are [`Visibility::Unknown`]. `elev_at(x, y) -> Option<meters>` is the
/// SAME injected point sampler [`sample_segment`] uses (raster-backed in tests, the 8 m grid live).
///
/// The march (see the module note): rays at `angular_step ≈ ½·cell_m / radius_m` (adjacent rays ≤½
/// cell apart at the rim) each stepping ½ `cell_m` outward — a 2× oversample of the ticket's ≤1-cell
/// floor, so no interior disc cell is left unsplatted. Along a ray a running MAX elevation-angle is
/// carried; a sampled cell is `Visible` iff its own angle (from the observer eye) is `≥` that running
/// max (a strict monotone horizon), else `Hidden`. The observer eye is anchored at `observer_ground_m
/// + eye_height_m` — the observer's TRUE elevation (constraint 1), never the first covered sample.
/// The observer's own cell is always `Visible` (you can see your own feet).
///
/// Budget: O(rays × steps) sampler calls — for the 2000 m / 8 m default ≈ 3140 rays × 500 steps
/// (2× oversample), measured well inside ~100 ms with an in-RAM grid sampler.
#[must_use]
pub fn compute_viewshed<F>(manifest: &DemManifest, p: ViewshedParams, elev_at: F) -> Viewshed
where
    F: Fn(f64, f64) -> Option<f64>,
{
    // Raster rect: the radius disc's bounding box, CLAMPED to the manifest coverage box so the raster
    // never allocates cells that can only ever be Unknown off the map. Cell centres are laid on an
    // 8 m lattice aligned to the clamped min corner.
    let cell = if p.cell_m.is_finite() && p.cell_m > 0.0 {
        p.cell_m
    } else {
        8.0
    };
    let radius = if p.radius_m.is_finite() && p.radius_m > 0.0 {
        p.radius_m
    } else {
        VIEWSHED_DEFAULT_RADIUS_M
    };
    let min_x = (p.obs_x - radius).max(manifest.min_x);
    let min_y = (p.obs_y - radius).max(manifest.min_y);
    let max_x = (p.obs_x + radius).min(manifest.max_x);
    let max_y = (p.obs_y + radius).min(manifest.max_y);

    // Dims from the clamped rect at one cell spacing (at least 1×1). `cols-1` spans [min,max], so the
    // last column centre lands on `max_x` — the same inclusive-endpoint convention as `DemVectorGrid`.
    let span_x = (max_x - min_x).max(0.0);
    let span_y = (max_y - min_y).max(0.0);
    let cols = ((span_x / cell).round() as usize) + 1;
    let rows = ((span_y / cell).round() as usize) + 1;
    let n = cols.saturating_mul(rows);

    // Observer off coverage → the whole raster is Unknown (constraint 1: no honest eye to cast from,
    // so nothing is faked visible).
    let Some(obs_ground) = p.observer_ground_m else {
        return Viewshed {
            cols,
            rows,
            cells: vec![Visibility::Unknown; n],
            min_x,
            min_y,
            max_x,
            max_y,
            obs_x: p.obs_x,
            obs_y: p.obs_y,
        };
    };
    let eye_z = obs_ground + p.eye_height_m;

    // Start every cell Hidden; the radial march promotes the ones the observer can see to Visible and
    // marks off-coverage reaches Unknown. A cell no ray ever reaches (only the bbox corners OUTSIDE
    // the radius disc, once the rays+steps are dense enough to leave no interior gap — see below)
    // stays Hidden — correct: those corners are beyond the sight radius, genuine dead ground.
    let mut cells = vec![Visibility::Hidden; n];

    // Helper: world (x,y) → nearest raster cell index, if inside the raster.
    let idx_of = |x: f64, y: f64| -> Option<usize> {
        let c = ((x - min_x) / cell).round();
        let r = ((y - min_y) / cell).round();
        if c < 0.0 || r < 0.0 {
            return None;
        }
        let (c, r) = (c as usize, r as usize);
        if c >= cols || r >= rows {
            return None;
        }
        Some(r * cols + c)
    };

    // The observer's own cell is Visible (constraint: you always see your own position). Guard both
    // the raster-bounds and coverage — the observer is in coverage by construction (obs_ground is
    // Some), but the bbox clamp could in principle drop it if radius is 0; the `.max(1)` dims keep at
    // least the observer cell.
    if let Some(oi) = idx_of(p.obs_x, p.obs_y) {
        cells[oi] = Visibility::Visible;
    }

    // Angular + step density so NO interior cell of the disc is missed by the radial splat (the
    // artifact that would otherwise leave flat-terrain cells stuck at the Hidden sentinel between
    // rays). The ticket's floor is "adjacent rays ≤1 cell apart at the rim" (arc = radius·dθ ≤ cell ⇒
    // dθ ≤ cell/radius); we halve BOTH the angular step and the along-ray step (OVERSAMPLE = 2.0) so
    // adjacent rays are ≤½ cell apart at the rim and each ray advances ½ cell per step. With rays and
    // steps both at half-cell, every cell centre in the disc lies within ~½ cell of some sample and
    // is splatted (nearest-cell rounding then lands on it). Cost stays O(rays × steps) — ~4× the
    // 1-cell march, measured well under the ~100 ms budget (see `viewshed_perf_default_radius_is_reported`).
    const OVERSAMPLE: f64 = 2.0;
    let ray_count = ((2.0 * std::f64::consts::PI) / (cell / radius) * OVERSAMPLE)
        .ceil()
        .max(1.0) as usize;
    let d_theta = (2.0 * std::f64::consts::PI) / ray_count as f64;
    let step_m = cell / OVERSAMPLE;
    // Steps along a ray: half-cell each, out to the radius.
    let steps = (radius / step_m).floor().max(1.0) as usize;

    for ri in 0..ray_count {
        let theta = ri as f64 * d_theta;
        let (dx, dy) = (theta.cos(), theta.sin());
        // Running MAX elevation angle (tangent of the vertical angle from the observer eye) that the
        // terrain has risen to along this ray. A cell is visible only if it clears this horizon. NEG
        // infinity so the first covered cell (nearest) is always visible (nothing occludes it yet).
        let mut max_angle = f64::NEG_INFINITY;
        for s in 1..=steps {
            let dist = s as f64 * step_m;
            if dist > radius {
                break;
            }
            let wx = p.obs_x + dx * dist;
            let wy = p.obs_y + dy * dist;
            // Off the manifest coverage box → Unknown at that cell; do NOT advance the horizon (an
            // unknown gap neither reveals nor hides what lies beyond it — constraint 1's honesty).
            if !in_coverage(manifest, wx, wy) {
                if let Some(i) = idx_of(wx, wy)
                    && cells[i] != Visibility::Visible
                {
                    cells[i] = Visibility::Unknown;
                }
                continue;
            }
            let Some(ground) = elev_at(wx, wy) else {
                if let Some(i) = idx_of(wx, wy)
                    && cells[i] != Visibility::Visible
                {
                    cells[i] = Visibility::Unknown;
                }
                continue;
            };
            // Elevation angle of THIS cell's ground from the observer eye: (ground − eye_z)/dist.
            // Using the ground (not ground+target-eye) is the conservative viewshed convention — the
            // observer sees the GROUND at the cell; a standing target there would be even easier to
            // see. dist > 0 here (s ≥ 1), so no divide-by-zero.
            let angle = (ground - eye_z) / dist;
            let visible = angle >= max_angle;
            if let Some(i) = idx_of(wx, wy) {
                // A cell hit by several rays: once Visible, stays Visible (any ray that sees it wins;
                // the horizon march makes rays agree on open terrain). An Unknown from an earlier ray
                // is overwritten by a real Hidden/Visible verdict from this in-coverage sample.
                if visible {
                    cells[i] = Visibility::Visible;
                } else if cells[i] != Visibility::Visible {
                    cells[i] = Visibility::Hidden;
                }
            }
            // Advance the occluding horizon: a taller ridge here shadows everything farther on the ray.
            if angle > max_angle {
                max_angle = angle;
            }
        }
    }

    Viewshed {
        cols,
        rows,
        cells,
        min_x,
        min_y,
        max_x,
        max_y,
        obs_x: p.obs_x,
        obs_y: p.obs_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Everon height range — packages/map-assets/everon/manifest.json.
    const MIN_M: f64 = -204.78;
    const MAX_M: f64 = 375.53;

    fn everon(width_px: usize, height_px: usize) -> DemManifest {
        DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 12800.0,
            max_y: 12800.0,
            width_px,
            height_px,
            flip_x: false,
            flip_z: false,
            height_min_m: MIN_M,
            height_max_m: MAX_M,
        }
    }

    #[test]
    fn zero_is_exact_min() {
        assert_eq!(uint16_to_meters(0.0, MIN_M, MAX_M), MIN_M);
    }

    #[test]
    fn full_scale_is_max_within_epsilon() {
        assert!((uint16_to_meters(65535.0, MIN_M, MAX_M) - MAX_M).abs() < 1e-10);
    }

    #[test]
    fn meters_cache_matches_scalar_and_stores_f32() {
        let raster: [u16; 5] = [0, 65535, 12345, 54321, 1];
        let out = meters_cache(&raster, MIN_M, MAX_M);
        assert_eq!(out.len(), raster.len());
        for (i, &u) in raster.iter().enumerate() {
            assert_eq!(out[i], uint16_to_meters(f64::from(u), MIN_M, MAX_M) as f32);
        }
    }

    #[test]
    fn world_to_pixel_endpoints() {
        let m = everon(6400, 6400);
        let a = world_to_pixel(0.0, 0.0, &m);
        assert_eq!((a.px, a.py), (0.0, 0.0));
        let b = world_to_pixel(12800.0, 12800.0, &m);
        assert_eq!((b.px, b.py), (6399.0, 6399.0));
    }

    #[test]
    fn world_to_pixel_axis_flip() {
        let mut m = everon(6400, 6400);
        m.flip_x = true;
        m.flip_z = true;
        let a = world_to_pixel(0.0, 0.0, &m);
        assert_eq!((a.px, a.py), (6399.0, 6399.0));
    }

    #[test]
    fn bilinear_2x2_center_is_mean() {
        // Synthetic 2×2 (mirrors sampleElevation.test.ts): corners 0,100,200,300; center = 150.
        let raster: [f32; 4] = [0.0, 100.0, 200.0, 300.0];
        let v = bilinear_sample(&raster, 2, 2, 0.5, 0.5);
        assert!((v - 150.0).abs() < 1e-9);
    }

    #[test]
    fn bilinear_u16_and_f32_agree_when_exact() {
        let u: [u16; 4] = [0, 100, 200, 300];
        let f: [f32; 4] = [0.0, 100.0, 200.0, 300.0];
        for (px, py) in [(0.0, 0.0), (0.25, 0.75), (0.9, 0.1)] {
            assert_eq!(
                bilinear_sample(&u, 2, 2, px, py),
                bilinear_sample(&f, 2, 2, px, py)
            );
        }
    }

    #[test]
    fn sample_elevation_out_of_bounds_is_none() {
        let m = everon(6400, 6400);
        let raster = vec![0u16; 64]; // tiny stand-in; the OOB check fires before any read
        assert!(sample_elevation_meters(-1.0, 0.0, &m, &raster, 6400, 6400).is_none());
    }

    // ── T-643 — segment sampler (the terrain profile the LoS ray + T-644 viewshed walk) ───────────

    /// A small synthetic manifest whose world box is `[0, span] × [0, span]` over a `w × h` raster,
    /// meters range `[0, 100]` (so a u16 maps linearly to metres and a flat closure is easy to reason
    /// about). Separate from `everon()` so the coverage box is tiny and easy to leave in a test.
    fn synth(span: f64, w: usize, h: usize) -> DemManifest {
        DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: span,
            max_y: span,
            width_px: w,
            height_px: h,
            flip_x: false,
            flip_z: false,
            height_min_m: 0.0,
            height_max_m: 100.0,
        }
    }

    /// Distances must be strictly increasing and the last one must equal the segment length exactly
    /// (endpoints land on the ends — the whole point of dividing the span into `n` equal intervals).
    #[test]
    fn segment_distances_are_monotone_and_end_exact() {
        let m = synth(1000.0, 100, 100);
        // A flat sampler (elevation 0 everywhere inside coverage) so every sample is emitted and the
        // distance sequence itself is under test.
        let prof = sample_segment(&m, (0.0, 0.0), (300.0, 400.0), 8.0, |_, _| Some(0.0));
        assert!(prof.len() >= 2, "a real segment yields ≥2 samples");
        // Strictly monotone increasing distances.
        for w in prof.windows(2) {
            assert!(
                w[1].dist_m > w[0].dist_m,
                "distances must strictly increase: {} then {}",
                w[0].dist_m,
                w[1].dist_m
            );
        }
        // First is 0, last is the full 3-4-5 length (500 m) to float tolerance.
        assert!((prof.first().unwrap().dist_m - 0.0).abs() < 1e-9);
        assert!(
            (prof.last().unwrap().dist_m - 500.0).abs() < 1e-9,
            "last dist must equal the segment length (500 m), got {}",
            prof.last().unwrap().dist_m
        );
    }

    /// BOTH endpoints are sampled (the observer's and target's own ground are in the profile), and
    /// the interval count is `ceil(total / step)` so no gap exceeds the step.
    #[test]
    fn segment_includes_both_endpoints_and_respects_step() {
        let m = synth(1000.0, 100, 100);
        // 100 m segment, 8 m step → ceil(100/8)=13 intervals → 14 samples, spacing 100/13 ≈ 7.69 m.
        let prof = sample_segment(&m, (10.0, 10.0), (110.0, 10.0), 8.0, |_, _| Some(0.0));
        assert_eq!(prof.len(), 14, "ceil(100/8)=13 intervals → 14 samples");
        assert!(
            (prof[0].dist_m).abs() < 1e-9,
            "first sample at the observer end (0 m)"
        );
        assert!(
            (prof.last().unwrap().dist_m - 100.0).abs() < 1e-9,
            "last sample at the target end (100 m)"
        );
        // No inter-sample gap exceeds the requested step (the miss-nothing guarantee).
        for w in prof.windows(2) {
            assert!(
                w[1].dist_m - w[0].dist_m <= 8.0 + 1e-9,
                "gap {} exceeds the 8 m step",
                w[1].dist_m - w[0].dist_m
            );
        }
    }

    /// The profile reads the REAL elevation the sampler returns, in order — not a constant. A ramp
    /// closure (elevation == x) must come back as an increasing elevation along an eastward segment.
    #[test]
    fn segment_reads_sampler_elevations_in_order() {
        let m = synth(1000.0, 100, 100);
        // elevation == x metres; segment east along y=0 from x=0 to x=100.
        let prof = sample_segment(&m, (0.0, 0.0), (100.0, 0.0), 25.0, |x, _| Some(x));
        // 25 m step over 100 m → 4 intervals → 5 samples at x = 0,25,50,75,100.
        let elevs: Vec<f64> = prof.iter().map(|s| s.elev_m).collect();
        assert_eq!(elevs, vec![0.0, 25.0, 50.0, 75.0, 100.0]);
        // And the distance of each equals x here (segment is axis-aligned east).
        for s in &prof {
            assert!((s.dist_m - s.elev_m).abs() < 1e-9);
        }
    }

    /// Off-coverage handling: a sample whose point leaves the manifest world box, OR whose sampler
    /// returns `None`, is OMITTED — never a fabricated 0. The covered subset comes back in order.
    #[test]
    fn segment_drops_off_coverage_samples() {
        let m = synth(100.0, 50, 50); // coverage box [0,100]²
        // Segment runs from inside coverage (x=50) OUT past the east edge (x=200). Samples with
        // x > 100 are off the world box and must be dropped even though the closure would answer.
        let prof = sample_segment(&m, (50.0, 50.0), (200.0, 50.0), 10.0, |_, _| Some(7.0));
        assert!(
            !prof.is_empty(),
            "the in-coverage head must still yield samples"
        );
        // Every emitted sample's along-distance maps to an x within [0,100] (dist from x=50).
        for s in &prof {
            let x = 50.0 + s.dist_m; // eastward, so world-x = start-x + distance
            assert!(
                x <= 100.0 + 1e-9,
                "sample at world-x {x} is outside coverage"
            );
            assert_eq!(s.elev_m, 7.0);
        }
        // A sampler that returns None everywhere → empty profile (no fake zeros), even in coverage.
        let none_prof = sample_segment(&m, (10.0, 10.0), (90.0, 10.0), 10.0, |_, _| None);
        assert!(
            none_prof.is_empty(),
            "None sampler yields no samples, never 0 m"
        );
        // A fully off-coverage segment (entirely east of the box) → empty.
        let gone = sample_segment(&m, (150.0, 10.0), (300.0, 10.0), 10.0, |_, _| Some(1.0));
        assert!(gone.is_empty(), "a segment outside coverage yields nothing");
    }

    /// A zero-length segment yields a single sample at dist 0 (if covered) — no divide-by-zero, no
    /// duplicate/empty. A degenerate step (0 / NaN) still walks the endpoints (one interval).
    #[test]
    fn segment_degenerate_length_and_step() {
        let m = synth(1000.0, 100, 100);
        // Zero-length (from == to): one sample at 0.
        let point = sample_segment(&m, (42.0, 42.0), (42.0, 42.0), 8.0, |_, _| Some(3.0));
        assert_eq!(point.len(), 1);
        assert!((point[0].dist_m).abs() < 1e-9);
        assert_eq!(point[0].elev_m, 3.0);
        // A zero-length point OFF coverage → empty (no fabricated sample).
        let off = sample_segment(&m, (2000.0, 2000.0), (2000.0, 2000.0), 8.0, |_, _| {
            Some(3.0)
        });
        assert!(off.is_empty());
        // Degenerate step (0) on a real segment → one interval → exactly the two endpoints.
        let two = sample_segment(&m, (0.0, 0.0), (100.0, 0.0), 0.0, |_, _| Some(1.0));
        assert_eq!(
            two.len(),
            2,
            "a 0 step degenerates to one interval (both ends)"
        );
        assert!((two[0].dist_m).abs() < 1e-9 && (two[1].dist_m - 100.0).abs() < 1e-9);
    }

    /// The sampler-injection seam works with the RASTER path the ticket names (the 6400²-style uint16
    /// DEM): a segment sampled through `sample_elevation_meters` returns the raster's real metres.
    /// This is the exact closure the LoS goldens + T-644 use — proving `sample_segment` composes with
    /// the existing point sampler, not just synthetic closures.
    #[test]
    fn segment_composes_with_the_raster_point_sampler() {
        // 2×2 raster, corners 0..300 (u16); manifest maps the 2 px across [0, 100] m, meters [0,300].
        let raster: [u16; 4] = [0, 100, 200, 300];
        let m = DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 100.0,
            max_y: 100.0,
            width_px: 2,
            height_px: 2,
            flip_x: false,
            flip_z: false,
            height_min_m: 0.0,
            height_max_m: 300.0,
        };
        // Raster-backed closure — the golden/T-644 form.
        let prof = sample_segment(&m, (0.0, 0.0), (100.0, 0.0), 50.0, |x, y| {
            sample_elevation_meters(x, y, &m, &raster, 2, 2)
        });
        // 50 m step over 100 m → 3 samples at x=0,50,100 → px 0,0.5,1 on the top row.
        // Top row values 0 and 100 (u16), meters range [0,300] with 65535-scale means the u16
        // *values* 0/100 → tiny metres; the point sampler bilinear-interpolates the u16 then converts.
        assert_eq!(prof.len(), 3);
        // Endpoints exact, monotone x → monotone metres (top row 0 → 100 in u16 is increasing).
        assert!((prof[0].dist_m).abs() < 1e-9);
        assert!((prof[2].dist_m - 100.0).abs() < 1e-9);
        assert!(
            prof[0].elev_m <= prof[1].elev_m && prof[1].elev_m <= prof[2].elev_m,
            "metres increase along the raster's increasing top row"
        );
    }

    // ── T-644 — viewshed raster (radial ray-march) ────────────────────────────────────────────────

    /// A big flat-terrain manifest for the viewshed goldens (coverage box [0, span]², meters range
    /// wide enough for the synthetic ridges). Separate from `synth` only in the metres range.
    fn flat_world(span: f64) -> DemManifest {
        DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: span,
            max_y: span,
            width_px: 1,
            height_px: 1,
            flip_x: false,
            flip_z: false,
            height_min_m: 0.0,
            height_max_m: 500.0,
        }
    }

    fn params(
        obs_x: f64,
        obs_y: f64,
        ground: Option<f64>,
        radius: f64,
        cell: f64,
    ) -> ViewshedParams {
        ViewshedParams {
            obs_x,
            obs_y,
            observer_ground_m: ground,
            eye_height_m: 1.8,
            radius_m: radius,
            cell_m: cell,
        }
    }

    /// Class counts over only the cells whose CENTRE lies within `radius` of the observer — the
    /// disc, not the square bbox. Cells in the bbox CORNERS beyond the radius are legitimately Hidden
    /// (dead ground past the sight horizon), so a "flat terrain hides nothing" claim is about the
    /// disc, not the enclosing rectangle. Returns `(visible, hidden, unknown)` in-radius.
    fn disc_counts(vs: &Viewshed, radius: f64) -> (usize, usize, usize) {
        let (mut v, mut h, mut u) = (0usize, 0usize, 0usize);
        for r in 0..vs.rows {
            for c in 0..vs.cols {
                let x = vs.min_x + c as f64 * 8.0;
                let y = vs.min_y + r as f64 * 8.0;
                if ((x - vs.obs_x).powi(2) + (y - vs.obs_y).powi(2)).sqrt() > radius {
                    continue;
                }
                match vs.at(c, r) {
                    Visibility::Visible => v += 1,
                    Visibility::Hidden => h += 1,
                    Visibility::Unknown => u += 1,
                }
            }
        }
        (v, h, u)
    }

    /// RADIAL COVERAGE: every cell within the radius disc is classified (never left in a limbo
    /// state), the raster dims match the clamped bbox at the cell spacing, and the class counts sum
    /// to `cols*rows`. On flat ground fully inside coverage there are NO Unknown cells.
    #[test]
    fn viewshed_radial_coverage_classifies_every_cell() {
        let m = flat_world(4000.0);
        // Observer well inside coverage so the whole 200 m disc is on the map.
        let vs = compute_viewshed(
            &m,
            params(1000.0, 1000.0, Some(50.0), 200.0, 8.0),
            |_, _| Some(50.0),
        );
        // Dims: a 400 m span (±200) at 8 m → 50 intervals → 51 cells each axis.
        assert_eq!(vs.cols, 51, "±radius / cell + 1 columns");
        assert_eq!(vs.rows, 51);
        assert_eq!(vs.cells.len(), vs.cols * vs.rows);
        // EVERY raster cell carries a definite class (the coverage-completeness guarantee): the sum
        // of the three classes equals the raster size, so no cell is left in a limbo state.
        let (v, h, u) = vs.class_counts();
        assert_eq!(v + h + u, vs.cols * vs.rows, "every cell is classified");
        assert_eq!(u, 0, "fully in-coverage flat disc has no Unknown cells");
        assert!(v > 0, "flat terrain has visible cells");
        // WITHIN THE RADIUS DISC, flat terrain hides nothing — every in-radius cell is Visible. (The
        // bbox CORNERS beyond the radius are Hidden by design: dead ground past the sight horizon, so
        // the whole-raster `h` above is nonzero even on a flat plain.)
        let (dv, dh, du) = disc_counts(&vs, 200.0);
        assert_eq!(dh, 0, "flat terrain hides nothing WITHIN the sight radius");
        assert_eq!(du, 0, "in-coverage disc has no Unknown cells");
        assert!(dv > 0);
    }

    /// SYMMETRY / FLAT-TERRAIN SANITY: on perfectly flat ground every reached cell is Visible — a
    /// viewshed that hid anything on the flat would be miscounting the horizon. Also confirms the
    /// observer's own cell is Visible.
    #[test]
    fn viewshed_flat_terrain_all_visible() {
        let m = flat_world(4000.0);
        let vs = compute_viewshed(
            &m,
            params(2000.0, 2000.0, Some(100.0), 400.0, 8.0),
            |_, _| Some(100.0),
        );
        // Within the sight disc, flat terrain hides nothing and leaves nothing unknown.
        let (_, hidden, unknown) = disc_counts(&vs, 400.0);
        assert_eq!(
            hidden, 0,
            "flat terrain: nothing is hidden within the radius"
        );
        assert_eq!(
            unknown, 0,
            "flat terrain fully in coverage: nothing unknown"
        );
        // Observer cell is Visible.
        let oc = ((vs.obs_x - vs.min_x) / 8.0).round() as usize;
        let orr = ((vs.obs_y - vs.min_y) / 8.0).round() as usize;
        assert_eq!(
            vs.at(oc, orr),
            Visibility::Visible,
            "observer sees its own cell"
        );
    }

    /// RIDGE-SHADOW GOLDEN: a wall ridge east of the observer casts a shadow — cells just IN FRONT of
    /// the wall (between observer and wall) are Visible; cells BEHIND the wall (farther east, lower)
    /// are Hidden (dead ground). The observer is low; the wall is tall; the ground behind is back at
    /// observer height, so only the wall's shadow — not distance — hides those cells.
    #[test]
    fn viewshed_ridge_casts_a_shadow() {
        let m = flat_world(4000.0);
        let obs = (1000.0, 1000.0);
        // A tall N–S wall at x=1200 (200 m east of the observer). Ground is 10 m everywhere except a
        // 1-cell-thick 200 m wall. Observer eye ≈ 11.8 m; the wall towers over the sight line, so
        // everything east of it and below its crest sits in dead ground.
        let elev = |x: f64, _y: f64| -> Option<f64> {
            if (x - 1200.0).abs() < 4.0 {
                Some(200.0) // the wall
            } else {
                Some(10.0) // flat plain
            }
        };
        let vs = compute_viewshed(&m, params(obs.0, obs.1, Some(10.0), 600.0, 8.0), elev);

        // A cell IN FRONT of the wall (x≈1100, same row as observer) is Visible.
        let front_c = ((1100.0 - vs.min_x) / 8.0).round() as usize;
        let row = ((obs.1 - vs.min_y) / 8.0).round() as usize;
        assert_eq!(
            vs.at(front_c, row),
            Visibility::Visible,
            "ground between observer and the wall is visible"
        );
        // A cell BEHIND the wall (x≈1400, same row) is Hidden — the wall shadows it.
        let behind_c = ((1400.0 - vs.min_x) / 8.0).round() as usize;
        assert_eq!(
            vs.at(behind_c, row),
            Visibility::Hidden,
            "ground behind the wall is in dead ground"
        );
    }

    /// OBSERVER-ELEVATION ANCHOR — THE WAVE-109 REFUTATION CASE AS A GOLDEN. T-643's `occlusion()`
    /// anchors the eye at the FIRST COVERED sample; for a ray whose near stretch is off coverage and
    /// whose covered head DESCENDS, that seeds the sight line too low and reports a false BLOCKED.
    /// Here the eye is anchored at the OBSERVER'S OWN elevation (a HIGH observer), so a lower covered
    /// cell out along the ray reads Visible — never a false Hidden from a mis-anchored eye.
    ///
    /// Setup: observer on a 300 m hill; the terrain immediately around drops into an off-coverage gap
    /// (the manifest box starts east of the observer's near field is simulated by a sampler that
    /// returns None for a near annulus), then resumes as lower (100 m) covered ground farther out.
    /// The descending sight line from the TRUE 300 m eye clears the 100 m ground ⇒ Visible. If the
    /// eye were anchored at that first covered 100 m sample (T-643's bug), the ground beyond would sit
    /// at/above the (now flat, low) line and mis-read Hidden.
    #[test]
    fn viewshed_anchors_eye_at_observer_not_first_covered() {
        let m = flat_world(6000.0);
        let obs = (3000.0, 3000.0);
        // Observer ground 300 m (eye ≈ 301.8). A near annulus (16..80 m from the observer) returns
        // None (a coverage hole), then ground resumes at 100 m farther out. Descending line from 300 m
        // clears the 100 m ground.
        let elev = move |x: f64, y: f64| -> Option<f64> {
            let d = ((x - obs.0).powi(2) + (y - obs.1).powi(2)).sqrt();
            if d < 1.0 {
                Some(300.0) // the observer's own cell
            } else if (16.0..80.0).contains(&d) {
                None // off-coverage hole near the observer
            } else {
                Some(100.0) // lower ground beyond
            }
        };
        let vs = compute_viewshed(&m, params(obs.0, obs.1, Some(300.0), 400.0, 8.0), elev);

        // A far cell east on the observer's row (x≈3200, d≈200 m, ground 100 m) MUST be Visible —
        // the descending line from the true 300 m eye clears it. The false-BLOCK bug would hide it.
        let far_c = ((3200.0 - vs.min_x) / 8.0).round() as usize;
        let row = ((obs.1 - vs.min_y) / 8.0).round() as usize;
        assert_eq!(
            vs.at(far_c, row),
            Visibility::Visible,
            "wave-109: a high observer's descending line sees lower ground past a coverage hole \
             (eye anchored at the OBSERVER, not the first covered sample)"
        );
        // The off-coverage hole itself is Unknown, never faked visible (constraint 1).
        let hole_c = ((obs.0 + 40.0 - vs.min_x) / 8.0).round() as usize;
        assert_eq!(
            vs.at(hole_c, row),
            Visibility::Unknown,
            "an off-coverage cell is Unknown (rendered hidden), never a fabricated Visible"
        );
    }

    /// Observer OFF coverage → the whole raster is Unknown (no honest eye to cast from). Never a disc
    /// of fake-visible cells.
    #[test]
    fn viewshed_observer_off_coverage_is_all_unknown() {
        let m = flat_world(1000.0);
        // Observer at (2000,2000) is outside the [0,1000]² box → observer_ground_m None.
        let vs = compute_viewshed(&m, params(2000.0, 2000.0, None, 200.0, 8.0), |_, _| {
            Some(50.0)
        });
        let (v, h, u) = vs.class_counts();
        assert_eq!(v, 0, "off-coverage observer: nothing visible");
        assert_eq!(h, 0, "off-coverage observer: nothing hidden");
        assert_eq!(u, vs.cols * vs.rows, "off-coverage observer: all Unknown");
    }

    /// FIRE THE RIDGE-SHADOW RULE (perturb / fail / restore). The shadow rule genuinely discriminates:
    /// with the wall present, a cell behind it is Hidden and asserting it Visible fails; REMOVING the
    /// wall (flat plain) restores Visible for that same cell. A viewshed that ignored terrain (all
    /// Visible always) would pass the perturbed Visible assertion — so this proves the running-horizon
    /// occlusion is load-bearing, not incidental.
    #[test]
    fn viewshed_ridge_shadow_rule_fires() {
        let m = flat_world(4000.0);
        let obs = (1000.0, 1000.0);
        let behind_c_of = |vs: &Viewshed| ((1400.0 - vs.min_x) / 8.0).round() as usize;
        let row_of = |vs: &Viewshed| ((obs.1 - vs.min_y) / 8.0).round() as usize;

        // Baseline: WITH the wall, the cell behind it is Hidden.
        let walled = compute_viewshed(&m, params(obs.0, obs.1, Some(10.0), 600.0, 8.0), |x, _| {
            if (x - 1200.0).abs() < 4.0 {
                Some(200.0)
            } else {
                Some(10.0)
            }
        });
        let (bc, br) = (behind_c_of(&walled), row_of(&walled));
        assert_eq!(
            walled.at(bc, br),
            Visibility::Hidden,
            "baseline: the wall shadows the ground behind it"
        );
        // Perturb: CLAIM it is visible. That is FALSE behind the wall, so the equality must NOT hold —
        // the rule fires (a terrain-blind viewshed would have left it Visible and passed this).
        assert_ne!(
            walled.at(bc, br),
            Visibility::Visible,
            "the cell behind the wall MUST be hidden — if Visible, the viewshed is ignoring terrain"
        );
        // Restore: REMOVE the wall (flat plain) → that same cell is Visible again.
        let flat = compute_viewshed(&m, params(obs.0, obs.1, Some(10.0), 600.0, 8.0), |_, _| {
            Some(10.0)
        });
        assert_eq!(
            flat.at(bc, br),
            Visibility::Visible,
            "removing the wall restores a clear sight to the cell behind where it stood"
        );
        // The verdict genuinely VARIES with the terrain (not a constant).
        assert_ne!(
            walled.at(bc, br),
            flat.at(bc, br),
            "the viewshed verdict must depend on the terrain"
        );
    }

    /// The viewshed composes with the RASTER point sampler the ticket names (same seam as
    /// `sample_segment`), not just synthetic closures.
    #[test]
    fn viewshed_composes_with_the_raster_point_sampler() {
        // 4×4 raster, flat u16 value 100 across a [0,300] m box, meters [0,300] → a flat ~0.46 m plain.
        let raster = [100u16; 16];
        let m = DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 300.0,
            max_y: 300.0,
            width_px: 4,
            height_px: 4,
            flip_x: false,
            flip_z: false,
            height_min_m: 0.0,
            height_max_m: 300.0,
        };
        let ground = uint16_to_meters(100.0, 0.0, 300.0);
        let vs = compute_viewshed(
            &m,
            params(150.0, 150.0, Some(ground), 100.0, 8.0),
            |x, y| sample_elevation_meters(x, y, &m, &raster, 4, 4),
        );
        // In-radius (disc) counts: a flat raster hides nothing within the sight radius.
        let (v, h, _u) = disc_counts(&vs, 100.0);
        assert!(v > 0, "raster-backed flat plain yields visible cells");
        assert_eq!(h, 0, "a flat raster hides nothing within the radius");
    }

    /// PERF MEASUREMENT (reported, NOT asserted — the ticket's rule). Times the default 2000 m /
    /// 8 m-cell disc with an in-RAM ramp sampler (the shape of the live 8 m grid read) and prints the
    /// elapsed ms + the disc size. Under `cargo test -- --nocapture` this is the number the ticket
    /// asks to report; it never fails the suite (CI timing is not a contract).
    #[test]
    fn viewshed_perf_default_radius_is_reported() {
        // A full Everon-scale coverage box so the 2000 m disc is entirely on the map.
        let m = DemManifest {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 12_800.0,
            max_y: 12_800.0,
            width_px: 6400,
            height_px: 6400,
            flip_x: false,
            flip_z: false,
            height_min_m: -204.78,
            height_max_m: 375.53,
        };
        // A cheap ramp sampler (elevation grows with x) — arithmetic only, like a grid lookup.
        let elev = |x: f64, _y: f64| -> Option<f64> { Some(x * 0.01) };
        let p = params(6400.0, 6400.0, Some(64.0), VIEWSHED_DEFAULT_RADIUS_M, 8.0);
        let t0 = std::time::Instant::now();
        let vs = compute_viewshed(&m, p, elev);
        let dt = t0.elapsed();
        let (v, h, u) = vs.class_counts();
        eprintln!(
            "T-644 viewshed perf: {:.2} ms | {}×{} raster ({} cells) | visible {} hidden {} unknown {}",
            dt.as_secs_f64() * 1000.0,
            vs.cols,
            vs.rows,
            vs.cols * vs.rows,
            v,
            h,
            u
        );
        // Sanity only (not a timing assert): the disc was computed and classified.
        assert_eq!(v + h + u, vs.cols * vs.rows);
        assert!(
            vs.cols >= 500 && vs.rows >= 500,
            "2000 m / 8 m ≈ 501-cell radius disc"
        );
    }
}
