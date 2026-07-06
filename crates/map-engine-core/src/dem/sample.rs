//! DEM sampling math — **Class R** (bit-identical to the TS reference
//! `apps/website/frontend/src/features/tactical-map/dem/sampleElevation.ts` and the parity oracle
//! `packages/tbd-schema/scripts/lib/dem-sample.mjs`).
//!
//! Arithmetic is `f64` in the same operation order as the JS, cast `as f32` at the same store
//! boundary as the JS `Float32Array` write, so buffer outputs compare `memcmp`-equal.

/// `uint16`-linear sample → meters ASL (Bohemia Terrain Creation Tool encoding).
/// Mirror of `uint16ToMeters` (`sampleElevation.ts:9`): `minM + (u16/65535)*(maxM - minM)`.
#[inline]
#[must_use]
pub fn uint16_to_meters(u16v: f64, min_m: f64, max_m: f64) -> f64 {
    min_m + (u16v / 65535.0) * (max_m - min_m)
}

/// Vectorized meters cache: a row-major `uint16` raster → `f32` meters. Mirror of
/// `buildMetersCache` (`DemTexture.ts:74`) — `out[i] = uint16ToMeters(raster[i], min, max)` stored
/// into a `Float32Array`. This is the Phase 0 boundary-proof kernel and the Phase 1 `dem::png`
/// meters-cache core (the Everon raster is 6400² = 40,960,000 samples → 163,840,000 bytes).
#[must_use]
pub fn meters_cache(raster: &[u16], min_m: f64, max_m: f64) -> Vec<f32> {
    raster
        .iter()
        .map(|&u| uint16_to_meters(f64::from(u), min_m, max_m) as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Everon height range — packages/map-assets/everon/manifest.json.
    const MIN_M: f64 = -204.78;
    const MAX_M: f64 = 375.53;

    #[test]
    fn zero_is_exact_min() {
        // u16 = 0 → minM + 0 → exactly minM (no rounding).
        assert_eq!(uint16_to_meters(0.0, MIN_M, MAX_M), MIN_M);
    }

    #[test]
    fn full_scale_is_max_within_epsilon() {
        // u16 = 65535 → minM + (maxM - minM); double-rounding may differ from the maxM literal.
        assert!((uint16_to_meters(65535.0, MIN_M, MAX_M) - MAX_M).abs() < 1e-10);
    }

    #[test]
    fn midpoint() {
        let mid = uint16_to_meters(32767.5, MIN_M, MAX_M);
        assert!((mid - (MIN_M + (MAX_M - MIN_M) / 2.0)).abs() < 1e-9);
    }

    #[test]
    fn meters_cache_matches_scalar_and_stores_f32() {
        let raster: [u16; 5] = [0, 65535, 12345, 54321, 1];
        let out = meters_cache(&raster, MIN_M, MAX_M);
        assert_eq!(out.len(), raster.len());
        for (i, &u) in raster.iter().enumerate() {
            // The public batch must equal the scalar core cast to f32 at the same boundary.
            assert_eq!(out[i], uint16_to_meters(f64::from(u), MIN_M, MAX_M) as f32);
        }
    }
}
