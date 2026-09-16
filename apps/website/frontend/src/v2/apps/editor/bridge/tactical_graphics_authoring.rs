//! Role: authoring the control measures — arm a multi-click draw, take and drop vertices, select
//! and drag an authored vertex, and delete a finished graphic.
//! Position: `editor/bridge` in the frontend editor shell.
//! Signals & state: the draw arm, the selection and the in-flight vertex drag are the document
//! operations' own process state; this module reads and writes the `tacticalGraphics` environment
//! key and nudges the reactive document tick every gesture the canvas must repaint for.
//! Invariants: an in-flight draw and an in-flight vertex drag write NOTHING to the document — they
//! nudge the tick, and the canvas lays the provisional position over the committed rows. Only a
//! closed draw, a committed drag and a delete reach the environment, each as ONE write, so a
//! gesture the operator experienced as one act is one Ctrl+Z. Picking runs over the same `Vec` the
//! canvas draws, so what is visible and what a click can find are one set by construction.

use serde_json::{json, Value};

use super::tactical_graphics::{
    pick_tactical_graphic, pick_tactical_vertex, tactical_graphics_from_env, TacticalDraft,
    TacticalGraphic,
};
use crate::v2::apps::editor::state::editor_context::{
    bump_doc_tick, read_env_value, update_environment,
};
use website_map_engine::data::store::operations::tactical_graphics as graphics_ops;

/// How many points each kind of control measure needs before it can close.
pub use website_map_engine::data::store::operations::tactical_graphics::tactical_min_points;

/// The authored graphics, as the canvas draws them.
#[must_use]
pub fn tactical_graphics_live() -> Vec<TacticalGraphic> {
    let env = read_env_value("tacticalGraphics").unwrap_or(Value::Null);
    tactical_graphics_from_env(&json!({ "tacticalGraphics": env }))
}

/// Lay the in-flight vertex drag over `rows` — what the lane must show mid-drag.
pub fn apply_tactical_drag_preview(rows: &mut [TacticalGraphic]) {
    let Some((id, index, x, z)) = graphics_ops::tactical_vertex_drag_preview() else {
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
    graphics_ops::selected_tactical_graphic()
}

/// The in-flight draw, for a live hint.
#[must_use]
pub fn tactical_draft() -> Option<TacticalDraft> {
    graphics_ops::tactical_draft()
}

/// Is a tactical draw armed?
#[must_use]
pub fn tactical_draw_armed() -> bool {
    graphics_ops::tactical_draw_armed()
}

/// Is a vertex drag in flight?
#[must_use]
pub fn tactical_vertex_drag_active() -> bool {
    graphics_ops::tactical_vertex_drag_active()
}

/// How many graphics the document carries — the dock's count readout.
#[must_use]
pub fn tactical_graphic_count() -> usize {
    tactical_graphics_live().len()
}

/// Select the graphic under a world point, or clear the selection on a miss. Returns what is
/// selected afterwards.
pub fn select_tactical_graphic_at(wx: f64, wy: f64, tol_m: f64) -> Option<String> {
    let hit = pick_tactical_graphic(&tactical_graphics_live(), wx, wy, tol_m);
    let changed = graphics_ops::select_tactical_graphic(hit.clone());
    if changed {
        bump_doc_tick();
    }
    hit
}

/// Drop the tactical selection. Returns whether anything was selected.
pub fn clear_tactical_selection() -> bool {
    let had = graphics_ops::clear_tactical_selection();
    if had {
        bump_doc_tick();
    }
    had
}

/// Arm a multi-click draw for `kind`. Refuses a kind [`tactical_min_points`] does not know.
pub fn begin_tactical_draw(kind: &str) -> bool {
    if !graphics_ops::begin_tactical_draw(kind) {
        return false;
    }
    bump_doc_tick();
    true
}

/// One canvas release while a draw is armed: append a vertex and stay armed. Returns the vertex
/// count, or 0 when no draw is in flight.
pub fn tactical_draw_push_vertex(x: f64, z: f64) -> usize {
    let n = graphics_ops::push_tactical_draw_vertex(x, z);
    if n > 0 {
        bump_doc_tick();
    }
    n
}

/// Drop the last placed vertex (the undo-vertex control). Returns the remaining count.
pub fn tactical_draw_pop_vertex() -> usize {
    let n = graphics_ops::pop_tactical_draw_vertex();
    bump_doc_tick();
    n
}

/// Abandon the in-flight draw without writing anything. Returns whether a draw was abandoned.
pub fn cancel_tactical_draw() -> bool {
    let cleared = graphics_ops::cancel_tactical_draw();
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
    let (next, id) = graphics_ops::complete_tactical_draw(rows, &draft);

    write_rows(next);
    graphics_ops::finish_tactical_draw(id);
    true
}

/// Arm a vertex drag if an AUTHORED vertex sits within `tol_m` of the press point. Selecting the
/// graphic is part of arming: a drag that did not visibly select what it is about to change would
/// leave the operator editing an unmarked line.
pub fn begin_tactical_vertex_drag(wx: f64, wy: f64, tol_m: f64) -> bool {
    let hit = pick_tactical_vertex(&tactical_graphics_live(), wx, wy, tol_m);
    if !graphics_ops::begin_tactical_vertex_drag(hit) {
        return false;
    }
    bump_doc_tick();
    true
}

/// Update the provisional vertex position. **Writes nothing to the document** — see the module
/// header. Returns whether a drag was in flight.
pub fn tactical_vertex_drag_move(wx: f64, wy: f64) -> bool {
    graphics_ops::move_tactical_vertex_drag(wx, wy)
}

/// Commit the drag — ONE environment write, therefore ONE undo step for the whole gesture.
pub fn commit_tactical_vertex_drag() -> bool {
    let Some((id, index, x, z)) = graphics_ops::take_tactical_vertex_drag() else {
        return false;
    };

    let Some(mut rows) = read_env_value("tacticalGraphics").and_then(|v| v.as_array().cloned())
    else {
        return false;
    };
    if !graphics_ops::commit_tactical_vertex_drag(&mut rows, &id, index, x, z) {
        return false;
    }

    write_rows(rows);
    true
}

/// Abandon the drag; the vertex snaps back to its committed position. Returns whether a drag was in
/// flight.
pub fn cancel_tactical_vertex_drag() -> bool {
    let had = graphics_ops::cancel_tactical_vertex_drag();
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
    let Some(next) = graphics_ops::delete_tactical_graphic(rows, id) else {
        return false;
    };
    write_rows(next);
    graphics_ops::forget_deleted_tactical_graphic(id);
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
    update_environment(graphics_ops::environment_patch(rows));
}
