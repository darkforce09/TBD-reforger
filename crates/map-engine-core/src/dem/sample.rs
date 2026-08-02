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
}
