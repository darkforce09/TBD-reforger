//! Role: sampler.
//! Position: `spatial/terrain_los` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::terrain::dem::manifest::DemManifest;
use crate::terrain::dem::sampling::in_coverage;

/// One sample along a terrain profile: `dist_m` is the along-segment distance from `from` (0 at the observer end), `elev_m` is the ground elevation there in metres ASL. Only points the sampler could answer (inside DEM coverage) are emitted — an off-coverage stretch simply has no samples, the honest gap (matching the CUR-Z / ruler-slope em-dash policy) rather than a fabricated 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfileSample {
    /// Along-segment distance from `from`, metres.
    pub dist_m: f64,

    /// Ground elevation at this point, metres ASL.
    pub elev_m: f64,
}

/// **Endpoints** — both `from` and `to` are sampled (so the observer's and target's own ground are in the profile). If EITHER endpoint (or any interior point) is off DEM coverage — the injected `elev_at` returns `None`, or the point falls outside the manifest's world box — that sample is omitted; the returned vector is the covered subset in order. A fully off-coverage segment yields an empty vector.
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

    let n: usize = if !step_m.is_finite() || step_m <= 0.0 {
        1
    } else {
        (total / step_m).ceil().max(1.0) as usize
    };

    let mut out = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = i as f64 / n as f64;
        let x = fx + dx * t;
        let y = fy + dy * t;

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
