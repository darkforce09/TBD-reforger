//! Role: tactical graphics.
//! Position: `doc/operations` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde_json::{Value, json};
use std::cell::RefCell;

/// The per-kind authored-vertex floor, straight from the CORE validator — one function, so the canvas can never complete a graphic `/compiled` would refuse.
#[must_use]
pub fn tactical_min_points(kind: &str) -> Option<usize> {
    crate::data::scenario::tactical_graphics::min_points(kind)
}

/// An in-flight multi-click draw.
#[derive(Clone, Debug, PartialEq)]
pub struct TacticalDraft {
    /// The armed kind — one of `crate::data::scenario::tactical_graphics::KINDS`.
    pub kind: String,

    /// Vertices placed so far, in click order.
    pub verts: Vec<(f64, f64)>,
}

impl TacticalDraft {
    /// Delegates to the CORE validator's own floor rather than restating it, so the canvas can never complete a graphic `/compiled` would refuse — the two numbers are one function.
    #[must_use]
    pub fn needed(&self) -> usize {
        crate::data::scenario::tactical_graphics::min_points(&self.kind)
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

/// A vertex drag in flight: which authored vertex was picked up, and where it is being held.
///
/// The held position is PROVISIONAL — nothing is written to the document until the drag is taken
/// on release, so a drag that is abandoned costs no undo step and a drag that is committed costs
/// exactly one for the whole gesture.
#[derive(Clone, Debug, PartialEq)]
pub struct TacticalVertexDrag {
    /// The graphic whose vertex is held.
    pub id: String,

    /// Which authored vertex of that graphic, by index.
    pub index: usize,

    /// Where the pointer is holding it, in world metres. `None` until the first move, which is why
    /// a press-and-release with no movement commits nothing.
    pub at: Option<(f64, f64)>,
}

/// The tactical lane's session: what is selected, what is being drawn, and what is being dragged.
/// None of it is document state and none of it is persisted.
#[derive(Default)]
struct TacticalSession {
    /// The selected graphic's id.
    selected: Option<String>,

    /// The multi-click draw in flight.
    draft: Option<TacticalDraft>,

    /// The vertex drag in flight.
    drag: Option<TacticalVertexDrag>,
}

thread_local! {
    static TACTICAL_SESSION: RefCell<TacticalSession> =
        RefCell::new(TacticalSession::default());
}

/// The selected graphic's id, if any.
#[must_use]
pub fn selected_tactical_graphic() -> Option<String> {
    TACTICAL_SESSION.with(|s| s.borrow().selected.clone())
}

/// The draw in flight, for a live hint.
#[must_use]
pub fn tactical_draft() -> Option<TacticalDraft> {
    TACTICAL_SESSION.with(|s| s.borrow().draft.clone())
}

/// Is a draw armed? This is what routes a canvas release to the draw rather than to the select
/// machine.
#[must_use]
pub fn tactical_draw_armed() -> bool {
    TACTICAL_SESSION.with(|s| s.borrow().draft.is_some())
}

/// Is a vertex drag in flight?
#[must_use]
pub fn tactical_vertex_drag_active() -> bool {
    TACTICAL_SESSION.with(|s| s.borrow().drag.is_some())
}

/// Where a moved vertex is being held — `(graphic id, vertex index, x, z)` — so the lane can draw
/// the drag over the authored rows without the document having been written. `None` when no drag
/// is in flight or the drag has not moved yet.
#[must_use]
pub fn tactical_vertex_drag_preview() -> Option<(String, usize, f64, f64)> {
    TACTICAL_SESSION.with(|s| {
        let session = s.borrow();
        let drag = session.drag.as_ref()?;
        let (x, z) = drag.at?;
        Some((drag.id.clone(), drag.index, x, z))
    })
}

/// Select `hit`, or clear the selection when it is `None`. Returns whether the selection changed,
/// which is what tells the host whether anything needs redrawing.
pub fn select_tactical_graphic(hit: Option<String>) -> bool {
    TACTICAL_SESSION.with(|s| {
        let mut session = s.borrow_mut();
        let changed = session.selected != hit;
        session.selected = hit;
        changed
    })
}

/// Drop the selection. Returns whether anything was selected.
pub fn clear_tactical_selection() -> bool {
    TACTICAL_SESSION.with(|s| s.borrow_mut().selected.take().is_some())
}

/// Arm a multi-click draw for `kind`. Refuses a kind [`tactical_min_points`] does not know, so a
/// draw can never be begun for something `/compiled` has no floor for.
pub fn begin_tactical_draw(kind: &str) -> bool {
    if tactical_min_points(kind).is_none() {
        return false;
    }
    TACTICAL_SESSION.with(|s| {
        s.borrow_mut().draft = Some(TacticalDraft {
            kind: kind.to_string(),
            verts: Vec::new(),
        });
    });
    true
}

/// One canvas release while a draw is armed: append a vertex and stay armed. Returns the vertex
/// count — 0 when no draw is in flight. A draw already at the schema's `maxItems` keeps its count
/// rather than growing a vertex the document would refuse.
pub fn push_tactical_draw_vertex(x: f64, z: f64) -> usize {
    TACTICAL_SESSION.with(|s| {
        let mut session = s.borrow_mut();
        let Some(draft) = session.draft.as_mut() else {
            return 0;
        };
        if draft.verts.len() >= crate::data::scenario::tactical_graphics::MAX_POINTS {
            return draft.verts.len();
        }
        draft.verts.push((x, z));
        draft.verts.len()
    })
}

/// Drop the last placed vertex. Returns the vertices remaining, and 0 when no draw is in flight.
pub fn pop_tactical_draw_vertex() -> usize {
    TACTICAL_SESSION.with(|s| {
        let mut session = s.borrow_mut();
        session.draft.as_mut().map_or(0, |draft| {
            draft.verts.pop();
            draft.verts.len()
        })
    })
}

/// Abandon the draw without writing anything. Returns whether a draw was abandoned.
pub fn cancel_tactical_draw() -> bool {
    TACTICAL_SESSION.with(|s| s.borrow_mut().draft.take().is_some())
}

/// Retire the draw that has just been written as `id`, and select what it minted: a graphic the
/// author has only now finished drawing is the one they mean to adjust next.
pub fn finish_tactical_draw(id: String) {
    TACTICAL_SESSION.with(|s| {
        let mut session = s.borrow_mut();
        session.draft = None;
        session.selected = Some(id);
    });
}

/// Arm a vertex drag on `hit` — the `(graphic id, vertex index)` a pick found, or `None` for a
/// press that found no vertex. Selecting the graphic is part of arming: a drag that did not
/// visibly select what it is about to change would leave the operator editing an unmarked line.
/// Returns whether a drag was armed.
pub fn begin_tactical_vertex_drag(hit: Option<(String, usize)>) -> bool {
    let Some((id, index)) = hit else {
        return false;
    };
    TACTICAL_SESSION.with(|s| {
        let mut session = s.borrow_mut();
        session.selected = Some(id.clone());
        session.drag = Some(TacticalVertexDrag {
            id,
            index,
            at: None,
        });
    });
    true
}

/// Update where the drag is holding the vertex. **Writes nothing to the document.** Returns
/// whether a drag was in flight.
pub fn move_tactical_vertex_drag(x: f64, z: f64) -> bool {
    TACTICAL_SESSION.with(|s| {
        let mut session = s.borrow_mut();
        let Some(drag) = session.drag.as_mut() else {
            return false;
        };
        drag.at = Some((x, z));
        true
    })
}

/// Consume the drag and yield what it moved — `(graphic id, vertex index, x, z)` — for the single
/// document write that ends the gesture. `None` when no drag is in flight, or when the drag never
/// moved; either way the drag is consumed, so a release can only ever commit once.
pub fn take_tactical_vertex_drag() -> Option<(String, usize, f64, f64)> {
    TACTICAL_SESSION.with(|s| {
        let drag = s.borrow_mut().drag.take()?;
        let (x, z) = drag.at?;
        Some((drag.id, drag.index, x, z))
    })
}

/// Abandon the drag; the vertex snaps back to its committed position. Returns whether a drag was
/// in flight.
pub fn cancel_tactical_vertex_drag() -> bool {
    TACTICAL_SESSION.with(|s| s.borrow_mut().drag.take().is_some())
}

/// Drop `id` from the selection if it is what was selected, after the row has been removed from
/// the document — a selection pointing at a row that no longer exists is a selection nothing can
/// act on.
pub fn forget_deleted_tactical_graphic(id: &str) {
    TACTICAL_SESSION.with(|s| {
        let mut session = s.borrow_mut();
        if session.selected.as_deref() == Some(id) {
            session.selected = None;
        }
    });
}

#[cfg(test)]
#[path = "tests/tactical_graphics.rs"]
mod tests;
