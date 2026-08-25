//! T-934.7 — transform commands of the old `state/operations.rs`: rotate / align /
//! space / orient / pattern and the shared `commit_positions` batch.
//! Split from `operations.rs`; the façade re-exports keep paths stable.

use crate::editor::state::history as mission_history;
use map_engine_core::doc::{EntityTransformPatch, MissionDocCore};

#[allow(unused_imports)]
use super::{attrs::*, cargo::*, compositions::*, context::*, entity::*};

/// T-648 XFORM-SHIFT-001 — rotate the whole selection to FACE the cursor `(cx, cy)` (world metres),
/// each entity about its OWN position, quantised to the rotation ladder rung `rung`
/// ([`crate::editor::mission_editor::transform`]). This is the commit end of the Shift+drag gesture and the
/// widget rotate ring.
///
/// Returns whether anything rotated (nothing selected, or every entity sitting exactly under the
/// cursor, is a no-op — [`crate::editor::mission_editor::transform::bearing_to_face`] returns `None` for a
/// degenerate aim and that entity is left untouched).
///
/// **Undo (T-732):** commits through [`MissionDocCore::rotate_entities`] — one LOCAL txn — so a
/// multi-selection rotate is **one** Ctrl+Z, matching the single-entity case. The whole gesture still
/// fires **one** history/persist tail (`after_local_edit` once below).
pub fn rotate_selection_to_face(cx: f64, cy: f64, rung: usize) -> bool {
    if !cx.is_finite() || !cy.is_finite() {
        return false;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        if sel.is_empty() {
            return false;
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let soa = core.materialize();
        let veh_root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok();
        let terrain = veh_root
            .as_ref()
            .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
            .unwrap_or_default();
        let tb = map_engine_core::mission::compile::terrain_bounds(&terrain);
        let mut items: Vec<(String, bool, f64)> = Vec::new();
        for id in &sel {
            if let Some(row) = soa.ids.iter().position(|s| s == id) {
                let (sx, sy) = (f64::from(soa.xs[row]), f64::from(soa.ys[row]));
                if let Some(bearing) =
                    crate::editor::mission_editor::transform::bearing_to_face(sx, sy, cx, cy)
                {
                    let deg = crate::editor::mission_editor::transform::snap_rotate(bearing, rung);
                    items.push((id.clone(), true, deg));
                }
                continue;
            }
            let Some(pos) = veh_root
                .as_ref()
                .and_then(|r| r.get("vehiclesById")?.get(id)?.get("position").cloned())
            else {
                continue;
            };
            let (Some(vx), Some(vy)) = (
                pos.get("x").and_then(serde_json::Value::as_f64),
                pos.get("y").and_then(serde_json::Value::as_f64),
            ) else {
                continue;
            };
            if let Some(bearing) =
                crate::editor::mission_editor::transform::bearing_to_face(vx, vy, cx, cy)
            {
                let deg = crate::editor::mission_editor::transform::snap_rotate(bearing, rung);
                items.push((id.clone(), false, deg));
            }
        }
        core.rotate_entities(&items, tb[2], tb[3]) > 0
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/* ═══════════════════════════════ T-645 — Placement helpers ═══════════════════════════════════════ */
//
// The wasm wiring for the Placement Tools. Every entry point here:
//   1. reads the LIVE selection's positions (slots off the materialized SoA, vehicles off
//      `small_maps_json` — the exact two sources `rotate_selection_to_face` reads),
//   2. computes target positions/yaws with the DOM-free pure math in `crate::editor::tools::place_helpers`
//      (natively golden-tested),
//   3. CONFIRMS via `confirm_with_message` when the op moves MORE THAN 10 entities, and
//   4. commits through [`MissionDocCore::update_entity_transforms`] / [`MissionDocCore::rotate_entities`]
//      (T-732 — one LOCAL txn = one undo step), then fires ONE `after_local_edit` history/persist tail.
//
// ── UNDO (T-732) ─────────────────────────────────────────────────────────────────────────────────
// Pattern / align / space / orient / Shift-rotate all share the atomic batch API. A 50-entity
// circular apply is ONE Ctrl+Z. The >10 confirm says so out loud (aligned with T-693's merge toast).

/// One selected entity resolved to its kind + current world position, for the placement math.
struct SelPos {
    id: String,
    /// `true` = slot; `false` = vehicle (both commit via the T-732 atomic batch APIs).
    is_slot: bool,
    x: f64,
    y: f64,
    /// The VEHICLE z a reposition preserves, read exact off `vehiclesById`.
    ///
    /// wave-127 F-5 — for a SLOT this is the f32 SoA column, so [`commit_positions`] must NOT commit
    /// it: widening `f32` back to `f64` would rewrite an authored z as a slightly different number on
    /// every align/space/pattern. The slot's z is resolved from the raw row ([`slot_z`]) at commit
    /// time instead. It is still carried here because the placement math and the callers read it.
    z: f64,
}

/// Resolve the live selection into `(SelPos list, terrain [x0,y0,w,h])`. Slots come off the
/// materialized SoA (widening the `f32` columns to f64 — the `Math.fround` store boundary); vehicles
/// come off `small_maps_json` `vehiclesById`. Ids in the selection that are neither (objects, or a
/// stale id) are dropped — placement acts on the transformable slot+vehicle set, the same scope as
/// `rotate_selection_to_face`. Returns an empty list when nothing resolves.
fn resolve_selection_positions(core: &MissionDocCore, sel: &[String]) -> (Vec<SelPos>, [f64; 4]) {
    let soa = core.materialize();
    let veh_root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok();
    let tb = terrain_bounds_of(core);
    let mut out = Vec::with_capacity(sel.len());
    for id in sel {
        if let Some(row) = soa.ids.iter().position(|s| s == id) {
            out.push(SelPos {
                id: id.clone(),
                is_slot: true,
                x: f64::from(soa.xs[row]),
                y: f64::from(soa.ys[row]),
                z: f64::from(soa.zs[row]),
            });
            continue;
        }
        if let Some(pos) = veh_root
            .as_ref()
            .and_then(|r| r.get("vehiclesById")?.get(id)?.get("position").cloned())
        {
            let (Some(vx), Some(vy)) = (
                pos.get("x").and_then(serde_json::Value::as_f64),
                pos.get("y").and_then(serde_json::Value::as_f64),
            ) else {
                continue;
            };
            out.push(SelPos {
                id: id.clone(),
                is_slot: false,
                x: vx,
                y: vy,
                z: pos
                    .get("z")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
            });
        }
    }
    (out, tb)
}

/// Ask the operator to confirm a bulk rearrangement when it moves more than the destructive
/// threshold (`place_helpers::needs_confirm`). Returns `true` to proceed. Below the threshold there
/// is no prompt (returns `true`). The wasm build shows a real `window().confirm(...)`; a native build
/// (no `window`) proceeds — the confirm is a UI guard, not a correctness gate, and the native path is
/// test-only. `verb` names the op in the prompt ("apply the Circular pattern to").
///
/// T-732 — the prompt states the honest one-step undo (atomic batch API), matching T-693's merge
/// toast form. Loadout bulk ops still N-step and use [`confirm_bulk_n_step`] instead.
#[cfg(target_arch = "wasm32")]
fn confirm_bulk(n: usize, verb: &str) -> bool {
    if !crate::editor::tools::place_helpers::needs_confirm(n) {
        return true;
    }
    let msg = format!("This will {verb} {n} entities. Continue? (Ctrl+Z undoes the whole op.)");
    web_sys::window()
        .and_then(|w| w.confirm_with_message(&msg).ok())
        .unwrap_or(false)
}

/// Confirm for bulk ops that are still N undo steps (loadout apply/remove — no atomic batch yet).
#[cfg(target_arch = "wasm32")]
pub(super) fn confirm_bulk_n_step(n: usize, verb: &str) -> bool {
    if !crate::editor::tools::place_helpers::needs_confirm(n) {
        return true;
    }
    let msg = format!("This will {verb} {n} entities. Continue?");
    web_sys::window()
        .and_then(|w| w.confirm_with_message(&msg).ok())
        .unwrap_or(false)
}

/// Commit a set of target positions (index-aligned with `entities`) through
/// [`MissionDocCore::update_entity_transforms`] — **one** LOCAL txn = **one** undo step (T-732).
/// Slots: x/y clamped to `[0,w]×[0,h]`, authored z carried through. Vehicles: z + existing heading
/// preserved (a move does not re-orient). Returns whether anything committed.
///
/// **wave-127 F-5 — a placement command no longer flattens an authored slot z.** Every slot write
/// here is an x/y write; the sticky z is resolved via [`slot_z`] / [`keep_z_rows`] and passed in so
/// the mutator cannot terrain-follow to `pz = 0.0`.
///
/// The rows are read ONCE for the whole batch, not per entity: this commits `k` entities and
/// [`raw_slot_rows`] is an O(document) JSON parse. `keep_z_rows` is asked with the FIRST moved slot's
/// write shape (x and y set, z absent).
fn commit_positions(
    core: &MissionDocCore,
    entities: &[SelPos],
    targets: &[crate::editor::tools::place_helpers::Pt],
    tb: [f64; 4],
) -> bool {
    let z_rows = entities
        .iter()
        .zip(targets.iter())
        .find(|(e, t)| e.is_slot && (e.x != t.x || e.y != t.y))
        .and_then(|(_, t)| keep_z_rows(core, Some(t.x), Some(t.y), None));
    let mut patches: Vec<EntityTransformPatch> = Vec::new();
    for (e, t) in entities.iter().zip(targets.iter()) {
        if e.x == t.x && e.y == t.y {
            continue;
        }
        if e.is_slot {
            let z = z_rows.as_ref().and_then(|rows| slot_z(rows, &e.id));
            patches.push(EntityTransformPatch {
                id: e.id.clone(),
                is_slot: true,
                x: Some(t.x),
                y: Some(t.y),
                z,
                rotation: None,
            });
        } else {
            let heading = vehicle_heading_of(core, &e.id).unwrap_or(0.0);
            patches.push(EntityTransformPatch {
                id: e.id.clone(),
                is_slot: false,
                x: Some(t.x.clamp(0.0, tb[2])),
                y: Some(t.y.clamp(0.0, tb[3])),
                z: Some(e.z),
                rotation: Some(heading),
            });
        }
    }
    core.update_entity_transforms(&patches, tb[2], tb[3]) > 0
}

/// Read a vehicle's current heading (rotation) off `small_maps_json`; `None` if absent/unplaced.
fn vehicle_heading_of(core: &MissionDocCore, id: &str) -> Option<f64> {
    let root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok()?;
    root.get("vehiclesById")?
        .get(id)?
        .get("position")?
        .get("rotation")?
        .as_f64()
}

/// T-645 (PLACE-PATTERN-001) — apply a placement PATTERN to the live selection, LIVE (in place). The
/// pattern re-arranges the selection's positions; each entity keeps its identity and rotation. `kind`
/// is the pattern selector; `Fill Area` seeds its deterministic scatter from the selection ids
/// (`place_helpers::seed_from_ids`), so the same selection scatters the same way (reproducible).
///
/// Confirms when moving > 10 entities. Returns whether anything moved (a selection of `< 2`, or a
/// pattern that lands every entity where it already is, moves nothing). **Undo: `k` steps for `k`
/// Undo (T-732): one step for the whole selection.**
pub fn apply_pattern_to_selection(kind: crate::editor::tools::place_helpers::PatternKind) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let (entities, tb) = resolve_selection_positions(core, &sel);
        if entities.len() < 2 {
            return false;
        }
        let src: Vec<crate::editor::tools::place_helpers::Pt> = entities
            .iter()
            .map(|e| crate::editor::tools::place_helpers::Pt::new(e.x, e.y))
            .collect();
        let targets = match kind {
            crate::editor::tools::place_helpers::PatternKind::Circular => {
                crate::editor::tools::place_helpers::pattern_circular(&src)
            }
            crate::editor::tools::place_helpers::PatternKind::Line => {
                crate::editor::tools::place_helpers::pattern_line(&src)
            }
            crate::editor::tools::place_helpers::PatternKind::Grid => {
                crate::editor::tools::place_helpers::pattern_grid(&src)
            }
            crate::editor::tools::place_helpers::PatternKind::FillArea => {
                let ids: Vec<String> = entities.iter().map(|e| e.id.clone()).collect();
                let seed = crate::editor::tools::place_helpers::seed_from_ids(&ids);
                crate::editor::tools::place_helpers::pattern_fill_area(&src, seed)
            }
        };
        #[cfg(target_arch = "wasm32")]
        if !confirm_bulk(
            entities.len(),
            &format!("apply the {} pattern to", kind.label()),
        ) {
            return false;
        }
        commit_positions(core, &entities, &targets, tb)
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-645 (PLACE-ALIGN-001) — align the live selection to one of the six edges/centres
/// (`place_helpers::AlignEdge`). Confirms when moving > 10. Returns whether anything moved. Undo:
/// one step for the whole selection (T-732).
pub fn align_selection(edge: crate::editor::tools::place_helpers::AlignEdge) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let (entities, tb) = resolve_selection_positions(core, &sel);
        if entities.len() < 2 {
            return false;
        }
        let src: Vec<crate::editor::tools::place_helpers::Pt> = entities
            .iter()
            .map(|e| crate::editor::tools::place_helpers::Pt::new(e.x, e.y))
            .collect();
        let targets = crate::editor::tools::place_helpers::align_edge(&src, edge);
        #[cfg(target_arch = "wasm32")]
        if !confirm_bulk(entities.len(), "align") {
            return false;
        }
        commit_positions(core, &entities, &targets, tb)
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-645 (PLACE-SPACE-001) — space the live selection equally along one of the three axes
/// (`place_helpers::SpaceAxis`). Confirms when moving > 10. Returns whether anything moved. Undo:
/// one step for the whole selection (T-732).
pub fn space_selection(axis: crate::editor::tools::place_helpers::SpaceAxis) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let (entities, tb) = resolve_selection_positions(core, &sel);
        if entities.len() < 3 {
            return false; // space-equally needs at least 3 (2 are already "spaced")
        }
        let src: Vec<crate::editor::tools::place_helpers::Pt> = entities
            .iter()
            .map(|e| crate::editor::tools::place_helpers::Pt::new(e.x, e.y))
            .collect();
        let targets = crate::editor::tools::place_helpers::space_equally(&src, axis);
        #[cfg(target_arch = "wasm32")]
        if !confirm_bulk(entities.len(), "space") {
            return false;
        }
        commit_positions(core, &entities, &targets, tb)
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-645 (PLACE-ORIENT-001) — orient the live selection under one of the six commands
/// (`place_helpers::Orient`): N/E/S/W set an absolute yaw; face-centre/face-away turn each entity
/// toward/away from the selection centroid. Rotates IN PLACE (no move) via
/// [`MissionDocCore::rotate_entities`] (T-732 — one LOCAL txn). An entity sitting exactly on the
/// centroid declines a FACE command (`orient_yaw` → `None`) and is left unchanged; cardinals always
/// apply.
///
/// Confirms when re-orienting > 10 entities. Returns whether anything rotated.
/// **Undo (T-732): one step for the whole selection.**
pub fn orient_selection(cmd: crate::editor::tools::place_helpers::Orient) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let (entities, tb) = resolve_selection_positions(core, &sel);
        if entities.is_empty() {
            return false;
        }
        #[cfg(target_arch = "wasm32")]
        if !confirm_bulk(entities.len(), "re-orient") {
            return false;
        }
        let pivot = crate::editor::tools::place_helpers::centroid(
            &entities
                .iter()
                .map(|e| crate::editor::tools::place_helpers::Pt::new(e.x, e.y))
                .collect::<Vec<_>>(),
        );
        let mut items: Vec<(String, bool, f64)> = Vec::new();
        for e in &entities {
            let Some(deg) = crate::editor::tools::place_helpers::orient_yaw(
                cmd,
                crate::editor::tools::place_helpers::Pt::new(e.x, e.y),
                pivot,
            ) else {
                continue; // degenerate face (entity on the centroid) → leave unchanged
            };
            items.push((e.id.clone(), e.is_slot, deg));
        }
        core.rotate_entities(&items, tb[2], tb[3]) > 0
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}
