//! Role: phrase the object half of a sight line and fold it together with the terrain half.
//! Position: `editing/tools/line_of_sight` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: a shot whose objects could not be judged reads "objects not loaded"; a verdict a proxy box decided reads "provisional". Nothing here ever reads clear because geometry was missing.

use super::object_wash::GLASS_CONCEALMENT;
use super::projection::ProjectedShot;
use super::terrain_verdict::{LosVerdict, format_distance};

/// The object layer's verdict for one shot.
#[derive(Clone, Debug, PartialEq)]
pub enum ObjectVerdict {
    /// No occluder reachable (pre-mount, the host mid-settle, or a chunk on the segment is not
    /// resident with nothing else blocking) — the honest "can't tell".
    NotLoaded,
    /// Nothing opaque on the segment; `concealment` is the glass / foliage fold.
    /// Nothing terminal on the segment. `concealment` is the combined foliage + glass
    /// concealment (`1 − Π(1 − cᵢ)`); `glass_panes` counts the panes crossed so the readout can
    /// say "through glass" rather than mislabel a 5 % pane as canopy.
    Clear { concealment: f64, glass_panes: u32 },
    /// An object stops the ray at `dist_m` along the ground run.
    Blocked {
        dist_m: f64,
        label: String,
        kind: String,
    },
    /// A proxy box (descriptor or BLAS still loading) stopped the ray.
    Provisional { dist_m: f64, label: String },
}

impl ObjectVerdict {
    /// The along-run distance at which objects stop the ray (blocked or provisional).
    #[must_use]
    pub fn block_dist(&self) -> Option<f64> {
        match self {
            ObjectVerdict::Blocked { dist_m, .. } | ObjectVerdict::Provisional { dist_m, .. } => {
                Some(*dist_m)
            }
            _ => None,
        }
    }
}

/// Terrain ∧ objects.
#[derive(Clone, Debug, PartialEq)]
pub struct CombinedVerdict {
    pub terrain: LosVerdict,
    pub objects: ObjectVerdict,
}

#[must_use]
pub fn combine(terrain: LosVerdict, objects: ObjectVerdict) -> CombinedVerdict {
    CombinedVerdict { terrain, objects }
}

/// The nearest block along the run — terrain or object — if any.
#[must_use]
pub fn first_block_dist(c: &CombinedVerdict) -> Option<f64> {
    let t = match c.terrain {
        LosVerdict::Blocked {
            blocking_dist_m, ..
        } => Some(blocking_dist_m),
        _ => None,
    };
    match (t, c.objects.block_dist()) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// What the overlay styles the shot as: blocked when anything blocks (the object block is
/// represented as a terrain-shaped `Blocked` at its distance), unknown when the terrain profile
/// is unknown, else clear. `format_combined` reads the real pair; this is only for CSS classes.
#[must_use]
pub fn styling_verdict(c: &CombinedVerdict) -> LosVerdict {
    if let LosVerdict::Unknown = c.terrain {
        return LosVerdict::Unknown;
    }
    match first_block_dist(c) {
        Some(d) => LosVerdict::Blocked {
            blocking_dist_m: d,
            blocking_elev_m: match c.terrain {
                LosVerdict::Blocked {
                    blocking_elev_m, ..
                } => blocking_elev_m,
                _ => 0.0,
            },
        },
        None => LosVerdict::Clear,
    }
}

/// The styling verdict of a projected shot (terrain + its object layer).
#[must_use]
pub fn styling_of(shot: &ProjectedShot) -> LosVerdict {
    styling_verdict(&CombinedVerdict {
        terrain: shot.verdict,
        objects: shot.objects.clone(),
    })
}

/// The one-line header:
///   * both clear      → `LoS clear · 640 m` (+ ` · canopy 43 %` when concealed ≥ 0.5 %)
///   * clear, no layer → `LoS clear · 1.24 km · objects not loaded`
///   * terrain nearer  → `LoS blocked at 412 m — terrain`
///   * object nearer   → `LoS blocked at 96 m — FarmHouse_E_1L01_Wood (building)`
///   * proxy nearer    → `LoS provisional at 96 m — Barn_01 (geometry loading)`
///   * no profile      → `LoS —`
#[must_use]
pub fn format_combined(c: &CombinedVerdict, total_m: f64) -> String {
    if let LosVerdict::Unknown = c.terrain {
        return "LoS —".to_string();
    }
    let terrain_d = match c.terrain {
        LosVerdict::Blocked {
            blocking_dist_m, ..
        } => Some(blocking_dist_m),
        _ => None,
    };
    let object_d = c.objects.block_dist();
    let terrain_wins = match (terrain_d, object_d) {
        (Some(t), Some(o)) => t <= o,
        (Some(_), None) => true,
        _ => false,
    };
    if terrain_wins {
        return format!(
            "LoS blocked at {} — terrain",
            format_distance(terrain_d.unwrap_or(0.0))
        );
    }
    match &c.objects {
        ObjectVerdict::Blocked {
            dist_m,
            label,
            kind,
        } => format!(
            "LoS blocked at {} — {label} ({kind})",
            format_distance(*dist_m)
        ),
        ObjectVerdict::Provisional { dist_m, label } => format!(
            "LoS provisional at {} — {label} (geometry loading)",
            format_distance(*dist_m)
        ),
        ObjectVerdict::Clear {
            concealment,
            glass_panes,
        } => {
            let mut s = format!("LoS clear · {}", format_distance(total_m));
            let glass_only = *glass_panes > 0
                && (*concealment - (1.0 - GLASS_CONCEALMENT.powi(*glass_panes as i32))).abs()
                    < 0.01;
            if *glass_panes > 0 {
                s.push_str(" · through glass");
            }
            if *concealment >= 0.005 && !glass_only {
                s.push_str(&format!(" · canopy {:.0} %", concealment * 100.0));
            }
            s
        }
        ObjectVerdict::NotLoaded => {
            format!(
                "LoS clear · {} · objects not loaded",
                format_distance(total_m)
            )
        }
    }
}

/// Attach the object verdict to a projected shot and move the blocking marker to the nearest
/// block of the pair.
pub fn apply_objects(shot: &mut ProjectedShot, objects: ObjectVerdict) {
    shot.objects = objects;
    let combined = CombinedVerdict {
        terrain: shot.verdict,
        objects: shot.objects.clone(),
    };
    shot.block_px = match first_block_dist(&combined) {
        Some(d) if shot.total_m > 0.0 => {
            let t = (d / shot.total_m).clamp(0.0, 1.0);
            Some((
                shot.obs_px + (shot.tgt_px - shot.obs_px) * t,
                shot.obs_py + (shot.tgt_py - shot.obs_py) * t,
            ))
        }
        _ => None,
    };
}

#[cfg(test)]
#[path = "tests/object_verdict.rs"]
mod tests;
