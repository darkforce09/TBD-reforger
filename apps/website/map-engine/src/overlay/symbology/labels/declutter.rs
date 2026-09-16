//! Role: declutter.
//! Position: `overlay/symbology/labels` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

#![forbid(unsafe_code)]

/// World-space map label (meters). Higher [`LabelSpec::importance`] wins collisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelSpec {
    /// Id.
    pub id: u32,

    /// X.
    pub x: i32,

    /// Y.
    pub y: i32,

    /// Importance.
    pub importance: u16,

    /// Text.
    pub text: String,
}

/// Screen-space minimum separation in CSS pixels at `deck_zoom = 0` (spec L4).
pub const MIN_LABEL_PX: f64 = 48.0;

/// World-meter minimum distance between label anchors at `deck_zoom`.
#[must_use]
pub fn min_label_distance_m(deck_zoom: f64) -> f64 {
    MIN_LABEL_PX * 2f64.powf(-deck_zoom)
}

fn sort_key(l: &LabelSpec) -> (i32, u32) {
    (-i32::from(l.importance), l.id)
}

fn dist_m(a: &LabelSpec, b: &LabelSpec) -> f64 {
    let dx = f64::from(a.x - b.x);
    let dy = f64::from(a.y - b.y);
    dx.hypot(dy)
}

/// Returns the draw set such that for every pair `(i,j)` both drawn: `dist(i,j) ≥ d_min` **or** `importance(i) ≠ importance(j)` with the higher-importance label kept (ties broken by lower `id` via sort order — lower-importance never both drawn inside `d_min`).
#[must_use]
pub fn declutter(labels: &[LabelSpec], deck_zoom: f64) -> Vec<LabelSpec> {
    let d_min = min_label_distance_m(deck_zoom);
    let mut candidates: Vec<LabelSpec> = labels
        .iter()
        .filter(|l| !l.text.trim().is_empty())
        .cloned()
        .collect();
    candidates.sort_by_key(sort_key);

    let mut keep: Vec<LabelSpec> = Vec::with_capacity(candidates.len());
    for cand in candidates {
        let ok = keep.iter().all(|k| {
            if dist_m(&cand, k) >= d_min {
                return true;
            }

            cand.importance > k.importance
        });

        let blocked = keep
            .iter()
            .any(|k| dist_m(&cand, k) < d_min && k.importance >= cand.importance);
        if ok && !blocked {
            keep.push(cand);
        }
    }
    keep
}

/// G4 invariant: every pair in `drawn` satisfies dist≥d_min OR unequal importance with the higher-importance label present (both may only co-exist when dist≥d_min).
#[must_use]
pub fn declutter_invariant_holds(drawn: &[LabelSpec], deck_zoom: f64) -> bool {
    let d_min = min_label_distance_m(deck_zoom);
    for (i, a) in drawn.iter().enumerate() {
        for b in drawn.iter().skip(i + 1) {
            let d = dist_m(a, b);
            if d < d_min && a.importance == b.importance {
                return false;
            }
            if d < d_min && a.importance != b.importance {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
#[path = "tests/declutter_tests.rs"]
mod tests;
