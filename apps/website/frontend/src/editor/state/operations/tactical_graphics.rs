//! T-936.7 — the document mutators for `tacticalGraphics[]`: draw, select, drag a vertex, delete.
//!
//! ══ ONE gesture, ONE undo step ══════════════════════════════════════════════════════════════
//! Every write here is a single [`super::context::update_environment`] call, which is one
//! `MissionDocCore::update_environment` merge patch (one core transaction) followed by one
//! `mission_history::after_local_edit()` — the `entity.rs:1122-1124` rule, so Ctrl+Z undoes a whole
//! gesture rather than a fragment of one.
//!
//! That is why a vertex DRAG does not touch the document until pointerup.
//! [`tactical_vertex_drag_move`] writes the provisional position into session state only and the
//! canvas re-uploads the lane from it, exactly as the slot drag preview does; the single write
//! happens in [`commit_tactical_vertex_drag`]. Writing per pointermove would compile, look
//! correct, and leave a hundred undo steps behind one drag.
//!
//! ══ Why session state and not `Pending` ═════════════════════════════════════════════════════
//! The multi-click zone draw rides `context.rs`'s `Pending::Zone`, and a tactical draw is the same
//! shape — but `Pending` is `pub(super) enum` in a file T-937.3 owns this wave, and adding a
//! variant there would be a cross-slice edit for no behavioural gain. This module keeps its own
//! [`TG_STATE`] thread-local instead. The trade is real and worth naming: a tactical draw is NOT
//! covered by `cancel_pending`, so [`cancel_tactical_draw`] is wired into the canvas Esc arm
//! explicitly — the same explicit wiring `cancel_zone_draw` needed (T-792) after the zone draw
//! turned out to survive `cancel_pending` for its own reasons.
//!
//! ══ Where the block lives ═══════════════════════════════════════════════════════════════════
//! `meta.environment.tacticalGraphics`, the editor's per-mission settings BAG — the transport every
//! T-936 authored block rides, for the mechanical reason `mission/extensions.rs`'s header gives:
//! it is the only part of `meta` with a read/write pair the editor can drive, and
//! `MissionDocCore::hydrate` loads it back VERBATIM, so an authored block survives Save → reload
//! with no change to `doc/store.rs`.

use serde_json::{json, Value};
use std::cell::RefCell;

use super::context::{bump_doc_tick, read_env_value, update_environment};
use crate::editor::canvas::tactical_graphics::{
    mint_graphic_id, pick_tactical_graphic, pick_tactical_vertex, tactical_graphics_from_env,
    TacticalDraft, TacticalGraphic,
};

/// The per-kind authored-vertex floor, straight from the CORE validator — one function, so the
/// canvas can never complete a graphic `/compiled` would refuse.
///
/// `None` means "not one of `map_engine_core::mission::tactical_graphics::KINDS`", which is how
/// [`begin_tactical_draw`] refuses an invented kind at the ARM rather than letting it reach the
/// document, where it would save 201 and then fail every `/compiled` fetch forever — the
/// discipline `begin_zone_draw` adopted after T-581 measured exactly that for an invented
/// `zone.type`.
#[must_use]
pub fn tactical_min_points(kind: &str) -> Option<usize> {
    map_engine_core::mission::tactical_graphics::min_points(kind)
}

/// A vertex drag between pointerdown and pointerup.
#[derive(Clone, Debug, PartialEq)]
struct VertexDrag {
    id: String,
    index: usize,
    /// The provisional position. `None` until the first pointermove, so a press-and-release with
    /// no travel commits nothing at all.
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

/* ─────────────────────────────── reads ─────────────────────────────── */

/// The authored graphics, as the canvas draws them.
#[must_use]
pub fn tactical_graphics_live() -> Vec<TacticalGraphic> {
    let env = read_env_value("tacticalGraphics").unwrap_or(Value::Null);
    tactical_graphics_from_env(&json!({ "tacticalGraphics": env }))
}

/// Lay the in-flight vertex drag over `rows` — what the lane must show mid-drag.
///
/// The document is untouched until pointerup (see the module header), so this overlay is the only
/// view in which a dragged vertex has moved. Everything else — the pick, the compile, a save — sees
/// the committed geometry, which is what makes the drag cancellable at all. A no-op when no drag is
/// in flight or the drag has not moved yet.
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

/// Is a tactical draw armed?
#[must_use]
pub fn tactical_draw_armed() -> bool {
    TG_STATE.with(|s| s.borrow().draft.is_some())
}

/// Is a vertex drag in flight?
#[must_use]
pub fn tactical_vertex_drag_active() -> bool {
    TG_STATE.with(|s| s.borrow().drag.is_some())
}

/// How many graphics the document carries — the dock's count readout.
#[must_use]
pub fn tactical_graphic_count() -> usize {
    tactical_graphics_live().len()
}

/* ─────────────────────────────── selection ─────────────────────────────── */

/// Select the graphic under a world point, or clear the selection on a miss. Returns what is
/// selected afterwards.
///
/// `tol_m` is the click radius in world metres, unprojected by the caller from
/// `TG_PICK_PX` through the FROZEN press camera — so the target is a constant screen size at every
/// zoom (`pick_connection`'s rule, and the reason this takes metres rather than pixels).
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

/* ─────────────────────────────── the draw ─────────────────────────────── */

/// Arm a multi-click draw for `kind`. Refuses a kind [`tactical_min_points`] does not know.
///
/// **NO CALLER YET, AND THAT IS A NAMED GAP, NOT AN OVERSIGHT.** Everything downstream of the arm
/// is wired and live — `gestures.rs`'s pointerdown appends a vertex per click, its `oncontextmenu`
/// finishes the draw, `commands.rs`'s Esc abandons it — but the ARM itself needs a surface that
/// presses this function, and both candidates are outside this slice's owns:
///
/// * a palette / dock control lives under `editor/panels/`;
/// * a keybinding in `canvas/commands.rs` compiles, but `help_modal.rs`'s
///   `every_binding_has_a_help_entry` fails until that file (also not owned here) grows a matching
///   `Shortcut` row, and `no_two_listeners_claim_the_same_chord` further rules out every code the
///   other window listeners already claim.
///
/// So the tool is complete and unreachable rather than half-built: one call site under `panels/`
/// makes the whole path live, and nothing here has to change when it lands. The `never used`
/// warning this raises is the honest signal of exactly that, which is why it is not silenced with
/// an `#[allow]`.
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

/// One canvas release while a draw is armed: append a vertex and stay armed. Returns the vertex
/// count, or 0 when no draw is in flight.
///
/// Nothing is written until [`complete_tactical_draw`], which is what enforces the per-kind floor —
/// the same split `begin_zone_draw` / `close_zone_polygon` uses so a ring is never handed over
/// short.
pub fn tactical_draw_push_vertex(x: f64, z: f64) -> usize {
    let n = TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        let Some(d) = st.draft.as_mut() else {
            return 0;
        };
        // The schema caps `points` at 128; refusing here keeps the draw from reaching a state
        // `complete_tactical_draw` would have to throw away.
        if d.verts.len() >= map_engine_core::mission::tactical_graphics::MAX_POINTS {
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
///
/// The canvas Esc arm calls this: a tactical draw does not ride `Pending`, so `cancel_pending` is a
/// false negative for it — the same trap T-792 fixed for the zone draw, and the reason this is
/// wired explicitly rather than assumed.
pub fn cancel_tactical_draw() -> bool {
    let cleared = TG_STATE.with(|s| s.borrow_mut().draft.take().is_some());
    if cleared {
        bump_doc_tick();
    }
    cleared
}

/// Commit the in-flight draw as one new graphic — ONE undo step.
///
/// Refuses (and KEEPS the draft) when the vertex count is under the kind's floor, so a premature
/// Enter costs the operator nothing. `false` also means "nothing was in flight".
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
    let id = mint_graphic_id(&rows, &draft.kind);
    let points: Vec<Value> = draft.verts.iter().map(|(x, z)| json!([*x, *z])).collect();
    let mut next = rows;
    next.push(json!({"id": id, "kind": draft.kind, "points": points}));

    write_rows(next);
    TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.draft = None;
        st.selected = Some(id);
    });
    true
}

/* ─────────────────────────────── the vertex drag ─────────────────────────────── */

/// Arm a vertex drag if an AUTHORED vertex sits within `tol_m` of the press point. Selecting the
/// graphic is part of arming: a drag that did not visibly select what it is about to change would
/// leave the operator editing an unmarked line.
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

/// Update the provisional vertex position. **Writes nothing to the document** — see the module
/// header. Returns whether a drag was in flight.
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
///
/// A drag that never moved (no pointermove between down and up) commits NOTHING and leaves no undo
/// step: a click on a vertex is a selection, not an edit, and filing an identity edit would make
/// Ctrl+Z appear to do nothing.
pub fn commit_tactical_vertex_drag() -> bool {
    let Some((id, index, x, z)) = TG_STATE.with(|s| {
        let mut st = s.borrow_mut();
        let d = st.drag.take()?;
        let (x, z) = d.at?;
        Some((d.id, d.index, x, z))
    }) else {
        // Either nothing was in flight, or it never moved. Both are no-ops, but the drag slot has
        // already been taken above in the second case, which is the whole point.
        return false;
    };

    let Some(mut rows) = read_env_value("tacticalGraphics").and_then(|v| v.as_array().cloned())
    else {
        return false;
    };
    let Some(row) = rows
        .iter_mut()
        .find(|r| r.get("id").and_then(Value::as_str) == Some(id.as_str()))
    else {
        // The graphic went away under the drag (an undo, a delete from a panel). Dropping the
        // commit is correct: re-adding the row would resurrect a deleted graphic.
        return false;
    };
    let Some(points) = row.get_mut("points").and_then(Value::as_array_mut) else {
        return false;
    };
    let Some(slot) = points.get_mut(index) else {
        return false;
    };
    *slot = json!([x, z]);

    write_rows(rows);
    true
}

/// Abandon the drag; the vertex snaps back to its committed position. Returns whether a drag was
/// in flight.
pub fn cancel_tactical_vertex_drag() -> bool {
    let had = TG_STATE.with(|s| s.borrow_mut().drag.take().is_some());
    if had {
        bump_doc_tick();
    }
    had
}

/* ─────────────────────────────── delete ─────────────────────────────── */

/// Delete one graphic by id — ONE undo step. Returns whether a row was removed.
pub fn delete_tactical_graphic(id: &str) -> bool {
    let Some(rows) = read_env_value("tacticalGraphics").and_then(|v| v.as_array().cloned()) else {
        return false;
    };
    let next: Vec<Value> = rows
        .into_iter()
        .filter(|r| r.get("id").and_then(Value::as_str) != Some(id))
        .collect();
    // Nothing matched ⇒ no write, so a stale selection cannot file an empty undo step.
    let removed = !next
        .iter()
        .any(|r| r.get("id").and_then(Value::as_str) == Some(id));
    if !removed {
        return false;
    }
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

/* ─────────────────────────────── shared ─────────────────────────────── */

/// The ONE write. A merge patch over `meta.environment` carrying the whole array, because
/// `update_environment` is RFC-7386-shaped: a nested array is REPLACED, never merged element-wise,
/// which is exactly the semantics an ordered vertex list needs.
///
/// An empty result writes `null` rather than `[]` — `copy_authored_blocks` treats `null` as a
/// CLEARED key and skips it, so deleting the last graphic returns the document to byte-identical
/// pre-T-936.7 output instead of leaving an empty array on the wire. (`tactical_graphics::parse`
/// refuses `[]` for the same reason: an empty list is omitted rather than authored.)
fn write_rows(rows: Vec<Value>) {
    let value = if rows.is_empty() {
        Value::Null
    } else {
        Value::Array(rows)
    };
    update_environment(json!({ "tacticalGraphics": value }).to_string());
}

// NO `#[cfg(test)]` MODULE HERE, DELIBERATELY. This whole file is wasm-only (`operations.rs`
// carries `#![cfg(target_arch = "wasm32")]`), so a test module here is compiled by nothing on the
// native runner and would report a green over code it never examined. The pure halves that DO have
// arithmetic worth pinning — `TacticalDraft::needed`/`hint` and `mint_graphic_id` — live in
// `canvas/tactical_graphics.rs`, where `cargo test -p website-frontend` runs them for real.
