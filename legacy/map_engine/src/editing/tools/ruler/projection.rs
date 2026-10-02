//! Role: project a chain's legs and vertices to screen space for drawing.
//! Position: `editing/tools/ruler` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: a drawn node is keyed by WHERE it is in the world, not by its label text — two legs reading `412 m` on different parts of the map must never share a node.

use super::chain::RulerChain;

//
// LABEL KEYING (wave-107 T-727 trap): the `<For>` over legs is keyed by the leg's WORLD-COORDINATE
// endpoints, NOT by the label text. Wave-107 found a `<For>` keyed on text retained a stale DOM
// position when two legs shared a label string (`"412 m · 073.2°"` twice), because Leptos reused the
// node for the "same" key at the wrong place. Keying on the world coords means a leg's node is tied
// to WHERE it is, so moving/re-placing a vertex re-positions the right node and identical labels on
// different legs never collide.

/// A projected leg ready to draw: screen-pixel endpoints + mid-point (for the label) + the label
/// text and a WORLD-coordinate key (Decision 1 / the T-727 keying fix). Built by [`project_legs`].
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedLeg {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub mid_x: f64,
    pub mid_y: f64,
    pub label: String,
    /// Stable key: the leg's two world endpoints quantised to 0.1 m. Ties the DOM node to WHERE the
    /// leg is, so identical label strings on different legs never share a `<For>` key (T-727).
    pub key: String,
}

/// A key string from a world coordinate pair, quantised to 0.1 m so tiny float noise between frames
/// does not churn the key (which would drop + re-create the node every frame). 0.1 m is far below a
/// visible pixel at any editor zoom, so two genuinely distinct vertices never collide.
#[must_use]
pub fn world_key(ax: f64, ay: f64, bx: f64, by: f64) -> String {
    format!(
        "{}:{}:{}:{}",
        (ax * 10.0).round() as i64,
        (ay * 10.0).round() as i64,
        (bx * 10.0).round() as i64,
        (by * 10.0).round() as i64,
    )
}

/// Project a chain's committed legs to screen space via a world→pixel projector (the live
/// `OrthoCamera::project` on wasm; injected here so this is pure + native-testable). Each leg gets
/// its endpoints, mid-point, label and a world-coordinate key. `project` takes world `(x, y)` and
/// returns screen `(px, py)`.
#[must_use]
pub fn project_legs<F>(chain: &RulerChain, project: F) -> Vec<ProjectedLeg>
where
    F: Fn(f64, f64) -> (f64, f64),
{
    chain
        .legs()
        .iter()
        .map(|leg| {
            let (x1, y1) = project(leg.from.x, leg.from.y);
            let (x2, y2) = project(leg.to.x, leg.to.y);
            let (mx, my) = leg.midpoint();
            let (mid_x, mid_y) = project(mx, my);
            ProjectedLeg {
                x1,
                y1,
                x2,
                y2,
                mid_x,
                mid_y,
                label: leg.label(),
                key: world_key(leg.from.x, leg.from.y, leg.to.x, leg.to.y),
            }
        })
        .collect()
}

/// A projected vertex dot — a small ring at each committed point so the operator sees the exact
/// clicked positions. Keyed by world coordinate (T-727) like the legs.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedVertex {
    pub px: f64,
    pub py: f64,
    pub key: String,
}

/// Project the committed vertices to screen dots (same projector as [`project_legs`]).
#[must_use]
pub fn project_vertices<F>(chain: &RulerChain, project: F) -> Vec<ProjectedVertex>
where
    F: Fn(f64, f64) -> (f64, f64),
{
    chain
        .points
        .iter()
        .map(|p| {
            let (px, py) = project(p.x, p.y);
            ProjectedVertex {
                px,
                py,
                key: world_key(p.x, p.y, p.x, p.y),
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/projection.rs"]
mod tests;
