//! Role: tactical graphics.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use serde_json::{json, Value};
use std::cell::RefCell;

/// Expose website mission core :: doc :: operations :: tactical graphics :: tactical min points at this domain boundary.
pub use website_map_engine::data::store::operations::tactical_graphics::tactical_min_points;

use super::context::{bump_doc_tick, read_env_value, update_environment};
use crate::editor::canvas::tactical_graphics::pick_tactical_graphic;
use crate::editor::canvas::tactical_graphics::pick_tactical_vertex;
use crate::editor::canvas::tactical_graphics::tactical_graphics_from_env;
use crate::editor::canvas::tactical_graphics::TacticalDraft;
use crate::editor::canvas::tactical_graphics::TacticalGraphic;

#[derive(Clone, Debug, PartialEq)]
struct VertexDrag {
    id: String,
    index: usize,

    at: Option<(f64, f64)>,
}

#[derive(Default)]
struct TgState {
    selected: Option<String>,
    draft: Option<TacticalDraft>,
    drag: Option<VertexDrag>,
}

thread_local! {
    /// Session state: the selection, the in-flight draw and the in-flight vertex drag. None of it
    /// is document state and none of it is persisted — the same footing as the ruler chain and the
    /// LoS capture.
    static TG_STATE: RefCell<TgState> = RefCell::new(TgState::default());
}

/// The authored graphics, as the canvas draws them.
#[must_use]
pub fn tactical_graphics_live() -> Vec<TacticalGraphic> {
    let env = read_env_value("tacticalGraphics").unwrap_or(Value::Null);
    tactical_graphics_from_env(&json!({ "tacticalGraphics": env }))
}

/// Lay the in-flight vertex drag over `rows` — what the lane must show mid-drag.
pub fn apply_tactical_drag_preview(rows: &mut [TacticalGraphic]) {
    let Some((id, index, x, z)) = TG_STATE.with(|s| {
        let st = s.borrow();
        let d = st.drag.as_ref()?;
        let (x, z) = d.at?;
        Some((d.id.clone(), d.index, x, z))
    }) else {
        return;
    };
    if let Some(g) = rows.iter_mut().find(|g| g.id == id) {
        if let Some(p) = g.points.get_mut(index) {
            *p = [x, z];
        }
    }
}

/// The selected graphic id, if any.
#[must_use]
pub fn selected_tactical_graphic() -> Option<String> {
    TG_STATE.with(|s| s.borrow().selected.clone())
}

/// The in-flight draw, for a live hint.
#[must_use]
pub fn tactical_draft() -> Option<TacticalDraft> {
    TG_STATE.with(|s| s.borrow().draft.clone())
}

/// Is a tactical draw armed?.
#[must_use]
pub fn tactical_draw_armed() -> bool {
    TG_STATE.with(|s| s.borrow().draft.is_some())
}

/// Is a vertex drag in flight?.
#[must_use]
pub fn tactical_vertex_drag_active() -> bool {
    TG_STATE.with(|s| s.borrow().drag.is_some())
}

/// How many graphics the document carries — the dock's count readout.
#[must_use]
pub fn tactical_graphic_count() -> usize {
    tactical_graphics_live().len()
}

/// Select the graphic under a world point, or clear the selection on a miss. Returns what is selected afterwards.
pub fn select_tactical_graphic_at(wx: f64, wy: f64, tol_m: f64) -> Option<String> {
    let hit = pick_tactical_graphic(&tactical_graphics_live(), wx, wy, tol_m);
    let changed = TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        let changed = st.selected != hit;
        st.selected.clone_from(&hit);
        changed
    });
    if changed {
        bump_doc_tick();
    }
    hit
}

/// Drop the tactical selection. Returns whether anything was selected.
pub fn clear_tactical_selection() -> bool {
    let had = TG_STATE.with(|s| s.borrow_mut().selected.take().is_some());
    if had {
        bump_doc_tick();
    }
    had
}

/// Arm a multi-click draw for `kind`. Refuses a kind [`tactical_min_points`] does not know.
pub fn begin_tactical_draw(kind: &str) -> bool {
    if tactical_min_points(kind).is_none() {
        return false;
    }
    TG_STATE.with(|s| {
        s.borrow_mut().draft = Some(TacticalDraft {
            kind: kind.to_string(),
            verts: Vec::new(),
        });
    });
    bump_doc_tick();
    true
}

/// One canvas release while a draw is armed: append a vertex and stay armed. Returns the vertex count, or 0 when no draw is in flight.
pub fn tactical_draw_push_vertex(x: f64, z: f64) -> usize {
    let n = TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        let Some(d) = st.draft.as_mut() else {
            return 0;
        };

        if d.verts.len() >= website_map_engine::data::scenario::tactical_graphics::MAX_POINTS {
            return d.verts.len();
        }
        d.verts.push((x, z));
        d.verts.len()
    });
    if n > 0 {
        bump_doc_tick();
    }
    n
}

/// Drop the last placed vertex (the Undo-vertex control). Returns the remaining count.
pub fn tactical_draw_pop_vertex() -> usize {
    let n = TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.draft.as_mut().map_or(0, |d| {
            d.verts.pop();
            d.verts.len()
        })
    });
    bump_doc_tick();
    n
}

/// Abandon the in-flight draw without writing anything. Returns whether a draw was abandoned.
pub fn cancel_tactical_draw() -> bool {
    let cleared = TG_STATE.with(|s| s.borrow_mut().draft.take().is_some());
    if cleared {
        bump_doc_tick();
    }
    cleared
}

/// Commit the in-flight draw as one new graphic — ONE undo step.
pub fn complete_tactical_draw() -> bool {
    let Some(draft) = tactical_draft() else {
        return false;
    };
    if draft.needed() > 0 {
        return false;
    }
    let rows = read_env_value("tacticalGraphics")
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let (next, id) =
        website_map_engine::data::store::operations::tactical_graphics::complete_tactical_draw(
            rows, &draft,
        );

    write_rows(next);
    TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.draft = None;
        st.selected = Some(id);
    });
    true
}

/// Arm a vertex drag if an AUTHORED vertex sits within `tol_m` of the press point. Selecting the graphic is part of arming: a drag that did not visibly select what it is about to change would leave the operator editing an unmarked line.
pub fn begin_tactical_vertex_drag(wx: f64, wy: f64, tol_m: f64) -> bool {
    let Some((id, index)) = pick_tactical_vertex(&tactical_graphics_live(), wx, wy, tol_m) else {
        return false;
    };
    TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.selected = Some(id.clone());
        st.drag = Some(VertexDrag {
            id,
            index,
            at: None,
        });
    });
    bump_doc_tick();
    true
}

/// Update the provisional vertex position. **Writes nothing to the document** — see the module header. Returns whether a drag was in flight.
pub fn tactical_vertex_drag_move(wx: f64, wy: f64) -> bool {
    TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        let Some(d) = st.drag.as_mut() else {
            return false;
        };
        d.at = Some((wx, wy));
        true
    })
}

/// Commit the drag — ONE `update_environment`, therefore ONE undo step for the whole gesture.
pub fn commit_tactical_vertex_drag() -> bool {
    let Some((id, index, x, z)) = TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        let d = st.drag.take()?;
        let (x, z) = d.at?;
        Some((d.id, d.index, x, z))
    }) else {
        return false;
    };

    let Some(mut rows) = read_env_value("tacticalGraphics").and_then(|v| v.as_array().cloned())
    else {
        return false;
    };
    if !website_map_engine::data::store::operations::tactical_graphics::commit_tactical_vertex_drag(
        &mut rows, &id, index, x, z,
    ) {
        return false;
    }

    write_rows(rows);
    true
}

/// Abandon the drag; the vertex snaps back to its committed position. Returns whether a drag was in flight.
pub fn cancel_tactical_vertex_drag() -> bool {
    let had = TG_STATE.with(|s| s.borrow_mut().drag.take().is_some());
    if had {
        bump_doc_tick();
    }
    had
}

/// Delete one graphic by id — ONE undo step. Returns whether a row was removed.
pub fn delete_tactical_graphic(id: &str) -> bool {
    let Some(rows) = read_env_value("tacticalGraphics").and_then(|v| v.as_array().cloned()) else {
        return false;
    };
    let Some(next) =
        website_map_engine::data::store::operations::tactical_graphics::delete_tactical_graphic(
            rows, id,
        )
    else {
        return false;
    };
    write_rows(next);
    TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.selected.as_deref() == Some(id) {
            st.selected = None;
        }
    });
    true
}

/// Delete the selected graphic, if any — the canvas Delete arm.
pub fn delete_selected_tactical_graphic() -> bool {
    let Some(id) = selected_tactical_graphic() else {
        return false;
    };
    delete_tactical_graphic(&id)
}

fn write_rows(rows: Vec<Value>) {
    update_environment(
        website_map_engine::data::store::operations::tactical_graphics::environment_patch(rows),
    );
}
