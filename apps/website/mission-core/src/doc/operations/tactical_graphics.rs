//! Role: tactical graphics.
//! Position: `doc/operations` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde_json::{Value, json};

/// The per-kind authored-vertex floor, straight from the CORE validator — one function, so the canvas can never complete a graphic `/compiled` would refuse.
#[must_use]
pub fn tactical_min_points(kind: &str) -> Option<usize> {
    crate::mission::tactical_graphics::min_points(kind)
}

/// An in-flight multi-click draw.
#[derive(Clone, Debug, PartialEq)]
pub struct TacticalDraft {
    /// The armed kind — one of `crate::mission::tactical_graphics::KINDS`.
    pub kind: String,

    /// Vertices placed so far, in click order.
    pub verts: Vec<(f64, f64)>,
}

impl TacticalDraft {
    /// Delegates to the CORE validator's own floor rather than restating it, so the canvas can never complete a graphic `/compiled` would refuse — the two numbers are one function.
    #[must_use]
    pub fn needed(&self) -> usize {
        crate::mission::tactical_graphics::min_points(&self.kind)
            .unwrap_or(2)
            .saturating_sub(self.verts.len())
    }
}

/// A stable id unique within `rows`: `tg_{kind initials}_{n}`, `n` the first free index.
#[must_use]
pub fn mint_graphic_id(rows: &[Value], kind: &str) -> String {
    let stem: String = kind.split('_').filter_map(|w| w.chars().next()).collect();
    for n in 1..=u32::MAX {
        let candidate = format!("tg_{stem}_{n}");
        let taken = rows
            .iter()
            .any(|r| r.get("id").and_then(Value::as_str) == Some(candidate.as_str()));
        if !taken {
            return candidate;
        }
    }

    format!("tg_{stem}")
}

/// Document operation over explicit authored state.
pub fn complete_tactical_draw(rows: Vec<Value>, draft: &TacticalDraft) -> (Vec<Value>, String) {
    let id = mint_graphic_id(&rows, &draft.kind);
    let points: Vec<Value> = draft.verts.iter().map(|(x, z)| json!([*x, *z])).collect();
    let mut next = rows;
    next.push(json!({"id": id, "kind": draft.kind, "points": points}));
    (next, id)
}

/// Document operation over explicit authored state.
pub fn commit_tactical_vertex_drag(
    rows: &mut [Value],
    id: &str,
    index: usize,
    x: f64,
    z: f64,
) -> bool {
    let Some(row) = rows
        .iter_mut()
        .find(|r| r.get("id").and_then(Value::as_str) == Some(id))
    else {
        return false;
    };
    let Some(points) = row.get_mut("points").and_then(Value::as_array_mut) else {
        return false;
    };
    let Some(slot) = points.get_mut(index) else {
        return false;
    };
    *slot = json!([x, z]);
    true
}

/// Document operation over explicit authored state.
pub fn delete_tactical_graphic(rows: Vec<Value>, id: &str) -> Option<Vec<Value>> {
    let before = rows.len();
    let next: Vec<Value> = rows
        .into_iter()
        .filter(|r| r.get("id").and_then(Value::as_str) != Some(id))
        .collect();

    let removed = next.len() < before;
    if !removed {
        return None;
    }
    Some(next)
}

/// Document operation over explicit authored state.
pub fn environment_patch(rows: Vec<Value>) -> String {
    let value = if rows.is_empty() {
        Value::Null
    } else {
        Value::Array(rows)
    };
    json!({ "tacticalGraphics": value }).to_string()
}
