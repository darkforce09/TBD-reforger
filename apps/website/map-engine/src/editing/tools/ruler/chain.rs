//! Role: the ruler's polyline capture: append, end, and the escalating dismissal.
//! Position: `editing/tools/ruler` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: the chain stores committed vertices only — the rubber-band leg to the cursor is the drawer's, not the chain's. Escape drops the in-progress tail first and the placed points second.

use super::leg::{Leg, RulerPoint, distance_m, format_total};

/// The persistent ruler polyline plus its placement phase. Session-local measurement state, held
/// by a host beside the selection set.
///
/// A chain is a list of committed vertices plus a `drawing` flag. While `drawing`, the NEXT click
/// appends a vertex and a live "rubber-band" leg to the cursor previews the leg-to-be (the overlay
/// draws it from the live cursor; the chain only stores committed vertices, so the preview needs no
/// state here). `double_click` ends the chain (clears `drawing`, keeps the points). `escape`
/// escalates: first it drops the in-progress tail (ends drawing), then a second Esc clears the
/// placed points entirely.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RulerChain {
    /// Committed vertices in click order.
    pub points: Vec<RulerPoint>,
    /// True while the operator is still adding points (the tool is "armed" for the next click).
    pub drawing: bool,
}

impl RulerChain {
    /// A fresh, empty chain (nothing placed, not drawing).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// True when there is nothing to draw or dismiss (no points and not mid-draw).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Commit a click at world `(x, y, z)` — the primary interaction (Decision 3). Appends the
    /// vertex and marks the chain `drawing` (so the next click continues the chain and the overlay
    /// shows the rubber-band leg to the cursor). Starting a fresh chain and extending an existing
    /// one are the same op — the only difference is whether `points` was empty.
    pub fn press(&mut self, x: f64, y: f64, z: Option<f64>) {
        self.points.push(RulerPoint::new(x, y, z));
        self.drawing = true;
    }

    /// End the chain but KEEP it placed (Decision 3 — the double-click contract). Clears `drawing`
    /// so no further click extends it; the points stay on the map until dismissed. A no-op on an
    /// empty chain. A double-click also fires two `press`es first (the browser emits pointer events
    /// before `dblclick`); the host de-dupes the coincident final vertex — see `end_dedup_epsilon`.
    pub fn double_click(&mut self) {
        self.drawing = false;
    }

    /// Escape — the two-step escalating dismissal (Decision 3):
    ///   * while `drawing` → drop the in-progress tail: stop drawing but KEEP the committed points
    ///     (so a mis-aimed final click is undone without losing the whole measure), UNLESS only a
    ///     single lone vertex exists (no leg yet) in which case there is nothing worth keeping and
    ///     the chain clears outright;
    ///   * when NOT drawing (a placed ruler) → clear the placed points entirely.
    ///
    /// Returns `true` if it changed anything (so the host can `preventDefault` only on a real act).
    pub fn escape(&mut self) -> bool {
        if self.is_empty() {
            return false;
        }
        if self.drawing {
            // A single un-legged vertex is not worth "keeping" — first Esc clears it.
            if self.points.len() <= 1 {
                self.points.clear();
            }
            self.drawing = false;
        } else {
            self.points.clear();
        }
        true
    }

    /// Clear everything (tool-switch back to Select is Decision 3's "second Esc equivalent"). Idempotent.
    pub fn clear(&mut self) {
        self.points.clear();
        self.drawing = false;
    }

    /// Drop the LAST committed vertex if it is within `eps` world metres of the point BEFORE it —
    /// the double-click de-dupe. A dblclick emits `pointerdown/up` (→ `press`) then `dblclick`
    /// (→ `double_click`); the two coincident presses would otherwise leave a zero-length final leg.
    /// The host calls this from the `dblclick` handler, before `double_click`, so the kept chain ends
    /// on the real penultimate vertex. Returns `true` if a duplicate tail was removed.
    pub fn dedup_tail(&mut self, eps: f64) -> bool {
        let n = self.points.len();
        if n < 2 {
            return false;
        }
        if distance_m(self.points[n - 2], self.points[n - 1]) <= eps {
            self.points.pop();
            true
        } else {
            false
        }
    }

    /// The committed legs (`points.len() − 1` of them; empty for 0/1 vertices). Each [`Leg`] carries
    /// distance/bearing/Δelev/slope, computed once so the overlay and the status bar agree.
    #[must_use]
    pub fn legs(&self) -> Vec<Leg> {
        self.points
            .windows(2)
            .map(|w| Leg::between(w[0], w[1]))
            .collect()
    }

    /// Total ground distance over all committed legs (metres). 0 for a 0/1-vertex chain.
    #[must_use]
    pub fn total_m(&self) -> f64 {
        self.legs().iter().map(|l| l.dist_m).sum()
    }

    /// The status-bar readout string (Decision 1 — the summary that rides beside CUR/OBJ/SEL), or
    /// `None` when there is nothing to summarise (no legs yet). Shows the running total and the
    /// LAST leg's readout, e.g. `"Σ 1.24 km · last 412 m · 073.2° · +8 m (2%)"`. `None` while a lone
    /// first vertex sits un-legged (the total is meaningless with no leg).
    #[must_use]
    pub fn status_readout(&self) -> Option<String> {
        let legs = self.legs();
        let last = legs.last()?;
        Some(format!(
            "{} · last {}",
            format_total(self.total_m()),
            last.label()
        ))
    }
}

// ── The DOM/SVG overlay (cheapest correct lane at ruler scale — a handful of points) ─────────────
//
// RENDERING LANE (the ticket's open call): the chain is a few points, so a GPU lane (a new wgpu
// pipeline + upload) would be far more machinery than the geometry warrants. The selection MARQUEE
// proves DOM overlay is the house idiom for transient camera-projected geometry at this scale
// (`MapGridRefs` also draws camera-projected labels from a pure geometry layer with no GPU), so the
// ruler draws as ONE absolutely-positioned SVG overlay: a polyline through the projected vertices +
// per-leg mid-line `<text>` labels. Reactive off the same cursor/heartbeat channel the scale bar +

#[cfg(test)]
#[path = "tests/chain.rs"]
mod tests;
