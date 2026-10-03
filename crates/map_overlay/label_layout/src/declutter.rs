//! Label declutter: which of a set of map labels draw at a zoom.
//!
//! **Role:** keeps the labels whose anchors stand at least [`min_label_distance_m`] apart at a
//! zoom, the more important label winning a collision ([`declutter`]), and checks a drawn set
//! against that rule ([`declutter_invariant_holds`]).
//! **Position:** `label_layout`; [`crate::text_packing`] declutters before it packs, and the
//! map engine's place-name packers build the [`LabelSpec`]s.
//! **Signals & state:** none; pure functions over their arguments.
//! **Invariants:** blank labels never draw; candidates are ranked by importance, then by the
//! lower [`LabelId`], so the drawn set is deterministic; no two drawn labels stand closer than
//! the minimum distance.

use crate::label_ids::LabelId;

/// World-space map label (meters). Higher [`LabelSpec::importance`] wins collisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelSpec {
    /// The caller's stable id; the declutter tie-break.
    pub id: LabelId,

    /// Anchor east coordinate in whole world metres.
    pub x: i32,

    /// Anchor north coordinate in whole world metres (the world Z axis).
    pub y: i32,

    /// Collision priority: the higher value keeps its label; equal values keep the lower `id`.
    pub importance: u16,

    /// The label text; a blank or whitespace-only text never draws.
    pub text: String,
}

/// Screen-space minimum separation in CSS pixels at `deck_zoom = 0`.
pub const MIN_LABEL_PX: f64 = 48.0;

/// World-meter minimum distance between label anchors at `deck_zoom`.
#[must_use]
pub fn min_label_distance_m(deck_zoom: f64) -> f64 {
    MIN_LABEL_PX * 2f64.powf(-deck_zoom)
}

fn sort_key(l: &LabelSpec) -> (i32, LabelId) {
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

/// The declutter invariant: every pair in `drawn` satisfies dist≥d_min OR unequal importance with the higher-importance label present (both may only co-exist when dist≥d_min).
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
