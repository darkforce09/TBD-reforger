//! Map label specs + importance-distance declutter (T-152.1 / T-144 G8 analogue).

#![forbid(unsafe_code)]

/// World-space map label (meters). Higher [`LabelSpec::importance`] wins collisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelSpec {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub importance: u16,
    pub text: String,
}

/// Screen-space minimum separation in CSS pixels at `deck_zoom = 0` (spec L4).
pub const MIN_LABEL_PX: f64 = 48.0;

/// World-meter minimum distance between label anchors at `deck_zoom`.
///
/// `d_min = MIN_LABEL_PX · 2^(−deck_zoom)` — same meter-per-pixel scale as the map engine.
#[must_use]
pub fn min_label_distance_m(deck_zoom: f64) -> f64 {
    MIN_LABEL_PX * 2f64.powf(-deck_zoom)
}

/// Stable sort key: higher importance first, then lower id.
fn sort_key(l: &LabelSpec) -> (i32, u32) {
    (-i32::from(l.importance), l.id)
}

fn dist_m(a: &LabelSpec, b: &LabelSpec) -> f64 {
    let dx = f64::from(a.x - b.x);
    let dy = f64::from(a.y - b.y);
    dx.hypot(dy)
}

/// Importance-distance declutter (T-144 G8 analogue).
///
/// Returns the draw set such that for every pair `(i,j)` both drawn:
/// `dist(i,j) ≥ d_min` **or** `importance(i) ≠ importance(j)` with the higher-importance
/// label kept (ties broken by lower `id` via sort order — lower-importance never both drawn
/// inside `d_min`).
///
/// Empty / blank texts are dropped.
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
            // Inside d_min: only keep if cand is strictly more important than k.
            // Because we iterate highest-importance first, a later cand should lose to an
            // already-kept higher (or equal-via-id-order) neighbour.
            cand.importance > k.importance
        });
        // Re-check: if any kept neighbour is within d_min and ≥ importance, skip.
        let blocked = keep
            .iter()
            .any(|k| dist_m(&cand, k) < d_min && k.importance >= cand.importance);
        if ok && !blocked {
            keep.push(cand);
        }
    }
    keep
}

/// G4 invariant: every pair in `drawn` satisfies dist≥d_min OR unequal importance with
/// the higher-importance label present (both may only co-exist when dist≥d_min).
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
                // Allowed only if both somehow kept — still violates "drop lower".
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lab(id: u32, x: i32, y: i32, imp: u16, text: &str) -> LabelSpec {
        LabelSpec {
            id,
            x,
            y,
            importance: imp,
            text: text.to_string(),
        }
    }

    #[test]
    fn empty_input_yields_empty() {
        assert!(declutter(&[], 0.0).is_empty());
    }

    #[test]
    fn blank_text_dropped() {
        let out = declutter(&[lab(1, 0, 0, 10, "  ")], 0.0);
        assert!(out.is_empty());
    }

    #[test]
    fn far_apart_both_kept() {
        let z = 0.0;
        let d = min_label_distance_m(z);
        let a = lab(1, 0, 0, 1, "A");
        let b = lab(2, (d as i32) + 10, 0, 1, "B");
        let out = declutter(&[a, b], z);
        assert_eq!(out.len(), 2);
        assert!(declutter_invariant_holds(&out, z));
    }

    #[test]
    fn close_pair_keeps_higher_importance() {
        let z = 0.0;
        let low = lab(1, 0, 0, 1, "low");
        let high = lab(2, 1, 0, 99, "high");
        let out = declutter(&[low, high], z);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "high");
        assert!(declutter_invariant_holds(&out, z));
    }

    #[test]
    fn g4_invariant_on_randomish_fixture() {
        let z = -2.0;
        let labels = vec![
            lab(1, 100, 100, 50, "A"),
            lab(2, 105, 100, 40, "B"),
            lab(3, 5000, 5000, 10, "C"),
            lab(4, 5010, 5000, 90, "D"),
            lab(5, 8000, 100, 5, "E"),
        ];
        let out = declutter(&labels, z);
        assert!(declutter_invariant_holds(&out, z));
        assert!(out.iter().any(|l| l.text == "A" || l.text == "D"));
        // B is near A with lower importance → dropped
        assert!(!out.iter().any(|l| l.text == "B"));
        // C near D with lower importance → dropped
        assert!(!out.iter().any(|l| l.text == "C"));
    }

    #[test]
    fn min_distance_scales_with_zoom() {
        assert!((min_label_distance_m(0.0) - 48.0).abs() < 1e-9);
        assert!((min_label_distance_m(-1.0) - 96.0).abs() < 1e-9);
        assert!((min_label_distance_m(1.0) - 24.0).abs() < 1e-9);
    }
}
