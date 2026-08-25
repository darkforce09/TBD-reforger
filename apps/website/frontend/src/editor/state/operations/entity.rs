//! T-934.7 — entity lifecycle half of the old `state/operations.rs`: selection, place /
//! delete / copy / paste, layers, comments, connections, ORBAT + vehicles, zones,
//! triggers, markers and the document index.
//! Split from `operations.rs`; the façade re-exports keep call-site paths stable.

use crate::core::dto::{FactionDoc, FactionRole, FactionVehicle};
use crate::editor::arsenal::asset_catalog::PlacePayload;
use crate::editor::panels::outliner::CommentRow;
use crate::editor::state::history as mission_history;
use leptos::prelude::{GetUntracked, Set};
use map_engine_core::doc::place_character_under_side;
use map_engine_core::doc::{
    apply_faction_library, FactionLibraryInput, FactionLibraryRole, FactionLibraryVehicle,
    MissionDocCore, APPLY_ANCHOR_X, APPLY_ANCHOR_Y, NONE_IDX,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

#[allow(unused_imports)]
use super::{attrs::*, cargo::*, compositions::*, context::*, transform::*};

/// The lazily-minted default layer (React's `ensureDefaultLayer`).
const DEFAULT_LAYER_ID: &str = "layer-1";

const DEFAULT_LAYER_NAME: &str = "Layer 1";

/// T-651 — one comment's editable fields for the editor overlay, or `None` when the id is gone
/// (deleted, or undone away while the panel was open — the overlay then closes itself rather than
/// editing a ghost).
#[must_use]
pub fn read_comment(id: &str) -> Option<CommentDetail> {
    comment_list().into_iter().find(|c| c.id == id)
}

/* ───────────────────────── keyboard actions (T-159.26 — MissionCreatorPage) ───────────────────────── */

thread_local! {
    /// The in-editor copy/paste clipboard (React `clipboardRef`) — raw slot dicts from `slots_json`.
    pub(super) static CLIPBOARD: RefCell<Vec<serde_json::Value>> = const { RefCell::new(Vec::new()) };
}

/// Delete/Backspace — remove the selected entities (React `removeEntities`).
///
/// **Wave 145 F-4 — NOT "in one undoable step", which is what this line used to claim.** The slot
/// removal is one transaction, but the T-672 edge cascade opens one per deleted slot and the T-784
/// comment loop opens one per removed note, so a multi-select or mixed delete costs several Ctrl+Z
/// presses to walk back. Nothing is lost — undo restores all of it — and a single-slot or
/// single-comment delete really is one step. The full argument, and why the batch would have to be
/// minted core-side, is in the KNOWN AND ACCEPTED note on the cascade below.
///
/// **T-784 — the selection can now hold a COMMENT id, so the verb PARTITIONS it.** A comment is a
/// `commentsById` key, not a slot id: handing it to `remove_slots` removed nothing and reported
/// success, which is the T-779 class (an acknowledgement for a write that never landed) and exactly
/// the failure a comment click would otherwise have shipped. Each half goes to the mutator that owns
/// it — [`crate::editor::state::operations::delete_comment`]'s `remove_comment` for the notes, `remove_slots` for
/// everything else — and neither half runs when it is empty, so **Delete over a comment-only
/// selection removes that comment and touches nothing else**. A MIXED selection removes both, which
/// is what selecting both and pressing Delete means.
///
/// The membership question is asked of [`comment_details`], the same `comments_json` read the
/// Outliner rows, the map lane and the map pick are built from — not a prefix test on the id. A
/// `cmt-` prefix is [`mint_comment_id`]'s convention, not a document invariant, and a hydrated
/// mission is free to carry comment ids that were never minted here.
pub fn delete_selection() -> bool {
    let removed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let ids = ctx.selection.borrow().clone();
        if ids.is_empty() {
            return false;
        }
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            let comment_ids: std::collections::HashSet<String> =
                comment_details(core).into_iter().map(|c| c.id).collect();
            let (comments, ids): (Vec<String>, Vec<String>) =
                ids.into_iter().partition(|id| comment_ids.contains(id));
            for id in &comments {
                core.remove_comment(id);
            }
            // T-672 — take each deleted unit's connection edges with it. Without this every delete
            // manufactures `CONN-DANGLING` findings the operator then has to clean up by hand — a
            // delete that half-finishes, which is exactly the failure class the connection graph's
            // warning is about. Deliberately BEFORE `remove_slots` so the cascade reads the id set
            // while the entities still exist.
            //
            // KNOWN AND ACCEPTED: this is one transaction per deleted slot plus one for the slot
            // removal, so a multi-select delete is several undo steps rather than one. Folding the
            // edge cascade into `remove_slots_in_txn` would fix that, but `remove_slots_in_txn` is
            // shared with `remove_editor_layer` and re-shaping it is a core-side change this slice
            // does not need to make to keep the graph honest. The visible cost is extra Ctrl+Z
            // presses; the alternative cost was dangling edges.
            //
            // WAVE 145 F-4 — **the comment loop above joins that acceptance, explicitly.** T-784
            // wrote it as a plain `for` over `core.remove_comment`, and `remove_comment` opens its
            // own transaction per call, so a delete over N notes is N more undo steps on top of the
            // cascade's. It was never added to this paragraph, which left the T-672 note reading as
            // if the cascade were the only multi-step half. It is the same accepted class, for the
            // same reason (the one-txn batch would have to be minted core-side, and `store.rs` is
            // out of this slice), with the same visible cost: extra Ctrl+Z presses, never lost work
            // — undo restores every removed note. A SINGLE-comment delete is genuinely one step.
            for id in &ids {
                let _ = core.remove_connections_touching(id);
            }
            // T-784 — GUARDED, because the comment-only case is the acceptance: with nothing but
            // notes selected there is no slot half, and handing `remove_slots` an empty `Vec` is a
            // pointless transaction opened over a document this delete did not change.
            //
            // WAVE 145 F-2 — what that guard is NOT. It was justified here as preventing "an empty
            // undo step", and that justification is empirically false: yrs skips a transaction that
            // wrote nothing, so `remove_slots(vec![])` leaves `can_undo()` FALSE and mints no undo
            // step at all (probed natively against `MissionDocCore`). The guard stays — a call that
            // provably cannot change anything should not be made, and saying "I removed the slots"
            // over an empty half is the vocabulary this partition exists to avoid — but it is
            // hygiene, not the last thing standing between the operator and a dead Ctrl+Z.
            if !ids.is_empty() {
                core.remove_slots(ids);
            }
        }
        ctx.selection.borrow_mut().clear();
        true
    });
    if removed {
        mission_history::after_local_edit();
    }
    removed
}

/// Spacebar — center the camera on the selection centroid (React `flyTo`, no auto-fly on click).
pub fn center_on_selection() -> bool {
    OPS_CTX.with(|c| {
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
        let mut sx = 0.0f64;
        let mut sy = 0.0f64;
        let mut n = 0.0f64;
        for id in &sel {
            if let Some(row) = soa.ids.iter().position(|s| s == id) {
                sx += f64::from(soa.xs[row]);
                sy += f64::from(soa.ys[row]);
                n += 1.0;
            }
        }
        if n == 0.0 {
            return false;
        }
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_view(sx / n, sy / n, e.zoom()); // keep zoom, center on centroid
            e.on_camera_changed(); // T-172 H5 — slot sizing/cluster gate
            true
        } else {
            false
        }
    })
}

/// Ctrl/Cmd+C — snapshot the selected slot dicts to the clipboard (React copy branch).
pub fn copy_selection() -> bool {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let sel: std::collections::HashSet<String> =
            ctx.selection.borrow().iter().cloned().collect();
        if sel.is_empty() {
            return false;
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
            return false;
        };
        let clip: Vec<serde_json::Value> = map
            .as_object()
            .map(|o| {
                o.values()
                    .filter(|v| {
                        v.get("id")
                            .and_then(|i| i.as_str())
                            .is_some_and(|i| sel.contains(i))
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if clip.is_empty() {
            return false;
        }
        CLIPBOARD.with(|cb| *cb.borrow_mut() = clip);
        true
    })
}

/// Ctrl/Cmd+V — paste the clipboard at `(cx, cy)` (the map cursor), preserving the relative layout
/// (React `pasteSlots`; centroid → cursor). Mints ids, files under the resolved layer, keeps the
/// source squad id (inert while squads is empty), selects the paste. `true` if anything pasted.
///
/// **T-777** — and each copy keeps the elevation it was authored at, rather than landing at ground
/// level. See the `z_rows` note in the body for why that read is off the clipboard snapshot.
pub fn paste_at_cursor(cx: Option<f64>, cy: Option<f64>) -> bool {
    let placed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let clip = CLIPBOARD.with(|cb| cb.borrow().clone());
        if clip.is_empty() {
            return Vec::new();
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let layer_id = ensure_layer(ctx, core);
        let terrain = serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
            .ok()
            .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
            .unwrap_or_default();
        let b = map_engine_core::mission::compile::terrain_bounds(&terrain);

        let n = clip.len();
        let mut ids = Vec::with_capacity(n);
        let (mut sx, mut sy, mut srot, mut zs) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let (mut squad_ids, mut layer_ids) = (Vec::new(), Vec::new());
        let (mut roles, mut tags, mut asset_ids, mut stances, mut loadouts) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut extras = Vec::with_capacity(n);
        let g = |v: &serde_json::Value, k: &str| {
            v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
        };
        let gp = |v: &serde_json::Value, k: &str| {
            v.get("position")
                .and_then(|p| p.get(k))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
        };
        // **T-777 — the copied slots' authored `z`, keyed by SOURCE id, resolved once for the whole
        // paste.** Built here rather than inside the loop below so the resolution is hoisted, and
        // built from the CLIPBOARD rather than from the live document, for two reasons:
        //
        // 1. `copy_selection` files the RAW `slots_json()` rows on the clipboard — the very rows
        //    [`slot_z`] is written to read — so the authored `z` is already in hand and the paste
        //    costs ZERO extra `raw_slot_rows` parses (that helper is O(document) JSON). The exact
        //    f64 survives, and a slot on a hidden layer still resolves: neither would be true off
        //    the materialized SoA, whose `zs` column is f32 and which drops T-665 hidden slots.
        // 2. `x`, `y` and `rotation` above already come from this snapshot. Reading `z` from the
        //    LIVE document instead would compose the copy's old x/y with the source's *current*
        //    elevation the moment anyone nudged the original after copying, and would resolve to
        //    nothing at all when the source has since been deleted or the clipboard is pasted into
        //    a different mission — and a failed read here is a silently flattened entity.
        //
        // `keep_z_rows` is deliberately NOT the reader for this path: its guard answers "could this
        // `update_slot_position` write terrain-follow an authored z to 0.0?", a question about that
        // mutator's Option signature which `paste_slots` does not have. Its PARTNER `slot_z` is the
        // shared reader, and that is what is reused — one z-resolution vocabulary, not a third.
        let z_rows: serde_json::Map<String, serde_json::Value> = clip
            .iter()
            .filter_map(|s| Some((s.get("id")?.as_str()?.to_string(), s.clone())))
            .collect();
        // T-220 — fields the parallel paste arrays do not carry (unknown keys + unknown
        // position sub-keys). Known keys are filtered again inside `paste_slots`.
        const PASTE_KNOWN: &[&str] = &[
            "id",
            "squadId",
            "index",
            "role",
            "tag",
            "assetId",
            "stance",
            "loadoutId",
            "loadout",
        ];
        for slot in &clip {
            ids.push(mint_id(ctx, core));
            sx.push(gp(slot, "x"));
            sy.push(gp(slot, "y"));
            srot.push(gp(slot, "rotation"));
            // **T-777 — the copy keeps the original's elevation.** This used to push a literal
            // ground value "for byte-parity with the flat-map case". The operator set that parity
            // aside on 2026-08-08 (it was a migration safety net, never a contract), and it was
            // unrecoverable regardless: nothing in this frontend re-samples terrain after a paste
            // — `terrainZ` did not survive the React deletion — so the zero was FINAL, and a
            // rooftop entity fell to the ground the moment it was duplicated. Resolved through the
            // shared [`slot_z`] against `z_rows` above; a row carrying no finite `z` still reads as
            // ground, exactly as before, so a flat-map paste is unchanged.
            //
            // **ORDER.** This push and the `ids.push` at the top of this iteration are the same
            // pass of the same walk over `clip`, so `zs[i]` is by construction the elevation of the
            // row that minted `ids[i]` — a total map, not a convention two sites happen to share.
            // Both vectors reach `paste_slots` below untouched; nothing between here and that call
            // sorts, filters, or re-orders either one.
            zs.push(slot_z(&z_rows, &g(slot, "id")).unwrap_or(0.0));
            // Keep the source squad if it still exists, else "" (empty squads map → inert).
            squad_ids.push(g(slot, "squadId"));
            layer_ids.push(layer_id.clone());
            roles.push(g(slot, "role"));
            tags.push(g(slot, "tag"));
            asset_ids.push(g(slot, "assetId"));
            let st = g(slot, "stance");
            stances.push(if st.is_empty() {
                "stand".to_string()
            } else {
                st
            });
            loadouts.push(
                slot.get("loadout")
                    .filter(|l| !l.is_null())
                    .map(std::string::ToString::to_string)
                    .unwrap_or_default(),
            );
            let mut extra = serde_json::Map::new();
            if let Some(obj) = slot.as_object() {
                for (k, v) in obj {
                    if PASTE_KNOWN.contains(&k.as_str()) {
                        continue;
                    }
                    if k == "position" {
                        if let Some(pos) = v.as_object() {
                            let mut pos_extra = serde_json::Map::new();
                            for (pk, pv) in pos {
                                if !matches!(pk.as_str(), "x" | "y" | "z" | "rotation") {
                                    pos_extra.insert(pk.clone(), pv.clone());
                                }
                            }
                            if !pos_extra.is_empty() {
                                extra.insert(
                                    "position".into(),
                                    serde_json::Value::Object(pos_extra),
                                );
                            }
                        }
                        continue;
                    }
                    extra.insert(k.clone(), v.clone());
                }
            }
            extras.push(if extra.is_empty() {
                String::new()
            } else {
                serde_json::Value::Object(extra).to_string()
            });
        }
        core.paste_slots(
            ids.clone(),
            squad_ids,
            layer_ids,
            sx,
            sy,
            srot,
            zs,
            roles,
            tags,
            asset_ids,
            stances,
            loadouts,
            extras,
            cx,
            cy,
            b[2],
            b[3],
        );
        *ctx.selection.borrow_mut() = ids.clone();
        ids
    });
    if !placed.is_empty() {
        mission_history::after_local_edit();
        true
    } else {
        false
    }
}

/// T-649 SEL-ALL-001 — Ctrl/Cmd+A: replace the selection with everything **on screen**.
///
/// Eden scopes Select All to the viewport, not to the whole mission, so this is a viewport-rect
/// query over [`crate::editor::tools::select_tool::view_ids_with_vehicles`] — the marquee's own primitive with its
/// corners pinned to the canvas — and not a `soa.ids` dump. `viewport_w`/`viewport_h` are the
/// container's CSS size at keypress; the camera is snapshotted from the live engine view the same
/// way a pointer-down freezes one, so Ctrl+A and a full-canvas marquee drag agree by construction.
///
/// `vehicle_points()` is resolved BEFORE the `OPS_CTX` borrow opens (it opens its own), keeping the
/// module's one-borrow-per-`pub fn` discipline. Returns whether it acted, so the keydown arm can
/// `prevent_default` — the browser's own Select All would otherwise blue-wash the editor chrome.
pub fn select_all_in_view(viewport_w: f64, viewport_h: f64) -> bool {
    if !(viewport_w > 0.0 && viewport_h > 0.0) {
        return false;
    }
    let points = vehicle_points();
    let acted = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let cam = {
            let eng = ctx.engine.borrow();
            let Some(e) = eng.as_ref() else {
                return false;
            };
            crate::editor::tools::select_tool::frozen_camera(
                viewport_w,
                viewport_h,
                e.target_x(),
                e.target_y(),
                e.zoom(),
            )
        };
        let ids = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            crate::editor::tools::select_tool::view_ids_with_vehicles(
                &cam,
                &core.materialize(),
                &points,
            )
        };
        // The engine tint lane is slots-only (the vehicle lane draws its own selection) — the same
        // split the `LG::Marquee` commit makes in `mission_editor.rs`.
        let slot_ids: Vec<String> = ids
            .iter()
            .filter(|i| !points.iter().any(|(v, _, _)| v == *i))
            .cloned()
            .collect();
        *ctx.selection.borrow_mut() = ids;
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_selection(slot_ids);
        }
        true
    });
    if acted {
        // Selection change, not a doc edit — the SEL readout only (T-159.21), never a history step.
        mission_history::refresh_selection();
    }
    acted
}

/// Outliner slot row → select it (replacing the selection), mirroring React: "selecting a slot
/// selects it globally (no auto camera move)" (`EditorLayersSection.tsx:5`). Runs the same
/// selection-only tail a map click does — no doc edit, so no rebind / persist / undo step.
pub fn select_slot(id: String) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        // T-788 F-27 (outliner half) — a click on a row whose id is ALREADY part of a
        // multi-selection keeps the selection instead of collapsing it to `[id]`. The outliner and
        // ORBAT rows route their single click through here and their dblclick through
        // `open_attributes`, so the first click of that dblclick used to shrink SEL9→SEL1 before
        // activate fired — the modal could only ever open single-edit. A click on a row OUTSIDE
        // the selection still REPLACES (Eden semantics — the exact contract `context_menu::open`'s
        // retarget documents: "identical to a left-click on an unselected object"). The tint lane
        // is untouched when keeping: it already shows the multi.
        let keep_multi = {
            let sel = ctx.selection.borrow();
            sel.len() > 1 && sel.iter().any(|s| *s == id)
        };
        if !keep_multi {
            *ctx.selection.borrow_mut() = vec![id];
            let ids = ctx.selection.borrow().clone();
            // NAMED, not a `borrow_mut()` temporary in the `if let`: a temporary would live to the
            // end of the closure and so drop AFTER `guard` — the borrow it reads through. A binding
            // declared after `guard` drops before it (reverse declaration order).
            let mut eng = ctx.engine.borrow_mut();
            if let Some(e) = eng.as_mut() {
                e.set_selection(ids); // tint lane
            }
        }
    });
    mission_history::refresh_selection(); // SEL + dock highlight only — no tree rebuild
}

/// Outliner folder row → make it the drop target (React's `setActiveLayer`).
pub fn set_active_layer(id: Option<String>) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.active_layer.set(id);
        }
    });
}

/// T-665 — flip a layer's `hidden` flag (the outliner eye toggle), then the shared post-change tail
/// (one commit = one undo step; `after_local_edit` re-materializes, so a now-hidden layer's slots
/// vanish from the map and a re-shown layer's return, and rebuilds the dock so the glyph updates).
pub fn set_layer_hidden(id: &str, hidden: bool) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_editor_layer_hidden(id, hidden);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// T-665 — flip a layer's `locked` flag (the outliner lock toggle) + the shared tail. Its slots
/// (and its subtree's) then refuse a move at the store level; the tree rebuild re-marks the rows.
pub fn set_layer_locked(id: &str, locked: bool) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_editor_layer_locked(id, locked);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
}

/* ═══════════════════════ T-701 — per-entity editor visibility override ═══════════════════════ */
//
// 3den E9 "Enable Visibility" — an editor-LOCAL `editorHidden` flag on the slot row that NEVER
// compiles (it rides `editor.slots`, the editor-only block reloaded verbatim, and is structurally
// stripped from the MOD wire by `flatten`'s `SlotIn`/`ModSlot` — see
// `store::set_slot_editor_hidden`). Distinct from the T-665 LAYER eye (per-LAYER, on a folder row):
// this is PER-ENTITY, so a maker can declutter a single dense-area entity without hiding its layer.
// Enforcement is at `store::materialize`, where effective-hidden = `layer-hidden OR entity-hidden`.
//
// These are thin wrappers onto the SHIPPED, TESTED store mutators (`set_slots_editor_hidden` — the
// per-entity ONE-TXN batch, so hiding the whole selection is ONE undo step; `clear_all_editor_hidden`
// — the reveal-all, one txn), riding the SAME post-change tail as the layer eye
// (`after_local_edit` → `refresh_docks`): the eye and the H-key flip visibility with the SAME
// semantics (one commit = one undo step; re-materialize drops/returns the affected slots), so there
// is NO new inconsistency between the two affordances.
//
// PENDING PRODUCT DECISION (T-715, amended wave 104), INHERITED not widened: a hidden slot vanishes
// from the dock trees (the `slot_rows` feed reads `materialize`, which now also drops entity-hidden
// slots) and selection consumers can act on a hidden entity sight-unseen. Entity-hidden JOINS
// layer-hidden in exactly that same open lane — this slice does not fix it and must not widen it.
//
// UI RESIDUE (a wiring line for T-733's family — NOT this slice's `owns`): the context-menu Hide/Show
// row (`context_menu.rs` + the T-664 enum), the H-key on the selection (`mission_editor` keydown),
// and the dock hidden-glyph render (`eden_tree`) all live in files OUTSIDE these two owned modules.
// What ships visibly here: this ops-level API + the `slot_hidden_rows` accessor a dock CAN render
// once wired. The keyboard/menu/glyph are the residue.

/// Filter a selection to the ids that are SLOTS (present in `slotsById`), keeping HIDDEN ones —
/// unlike a `materialize()`-based resolve, this reads `slots_json` so an entity that is currently
/// editor-hidden is still resolvable (you must be able to SHOW what you hid). Non-slot ids (vehicles,
/// objects, or a stale id) are dropped; the visibility flag lives on the slot row, matching the
/// store's slot-only `materialize` filter (vehicles/entities have no SoA filter this slice).
pub(super) fn selected_slot_ids(core: &MissionDocCore, sel: &[String]) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    let Some(map) = root.as_object() else {
        return Vec::new();
    };
    sel.iter()
        .filter(|id| map.contains_key(id.as_str()))
        .cloned()
        .collect()
}

/// Read whether the current selection is ALL-hidden (used to decide the toggle direction and to drive
/// a menu row's checked state). Returns `None` when the selection resolves to no slots (nothing to
/// toggle). `Some(true)` ⇒ every selected slot is hidden (so a toggle SHOWS); `Some(false)` ⇒ at
/// least one is visible (a toggle HIDES, matching Eden's "any visible → hide all" bias).
fn selection_all_hidden(core: &MissionDocCore, ids: &[String]) -> Option<bool> {
    if ids.is_empty() {
        return None;
    }
    let root = serde_json::from_str::<serde_json::Value>(&core.slots_json()).ok()?;
    let map = root.as_object()?;
    Some(ids.iter().all(|id| {
        map.get(id)
            .and_then(|s| s.get("editorHidden"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }))
}

/// T-701 (3den E9) — set `editorHidden` on the whole live SELECTION in ONE undo step, then the shared
/// post-change tail (re-materialize drops/returns the affected slots, dock rebuild re-marks the rows).
/// `hidden = true` hides, `false` shows. Rides `store::set_slots_editor_hidden` — the per-entity
/// one-txn batch — so the H-key over a multi-selection is one Eden action, NOT one step per slot
/// (contrast the T-732 position lane, which lacks such a batch). Returns whether anything was flipped
/// (an empty selection, or a selection with no slots, is a no-op). Shared by the H-key affordance and
/// a context-menu Hide/Show row once those are wired (T-733's family).
fn set_selection_hidden(hidden: bool) -> bool {
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
        let ids = selected_slot_ids(core, &sel);
        if ids.is_empty() {
            return false;
        }
        core.set_slots_editor_hidden(&ids, hidden);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-701 — HIDE the live selection (declutter). One undo step; returns whether anything was hidden.
///
/// `#[allow(dead_code)]`: the canonical ops API. Its live caller — an H-key on the selection — lives
/// in `mission_editor`'s keydown, and a context-menu Hide row in `context_menu.rs`; both are OUTSIDE
/// this slice's `owns` (the H-key + menu are the stated T-733-family residue). Shipped tested here.
#[allow(dead_code)]
pub fn hide_selection() -> bool {
    set_selection_hidden(true)
}

/// T-701 — SHOW (un-hide) the live selection. One undo step; returns whether anything was shown.
///
/// `#[allow(dead_code)]` — same residue note as [`hide_selection`] (the context-menu Show row / the
/// H-key toggle live outside `owns`).
#[allow(dead_code)]
pub fn show_selection() -> bool {
    set_selection_hidden(false)
}

/// T-701 — TOGGLE `editorHidden` on the live selection (the H-key affordance's natural verb): if every
/// selected slot is already hidden, SHOW them; otherwise HIDE them all (Eden's "any visible → hide
/// all" bias, so a mixed selection collapses to hidden). One undo step. Returns whether anything
/// flipped (no-op on an empty / slot-less selection).
///
/// `#[allow(dead_code)]`: this is the verb the H-key affordance calls (T-648 keydown idiom in
/// `mission_editor`, out of `owns`). Shipped + covered by the store batch/undo tests.
#[allow(dead_code)]
pub fn toggle_hidden() -> bool {
    // Decide the direction under a read borrow, then delegate to the one-txn setter. Reading the
    // direction and committing under separate borrows is fine: this is single-threaded editor state
    // and nothing mutates the selection between the two.
    let dir = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let sel = ctx.selection.borrow().clone();
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let ids = selected_slot_ids(core, &sel);
        // all-hidden → show (flip to visible); else → hide.
        selection_all_hidden(core, &ids).map(|all_hidden| !all_hidden)
    });
    match dir {
        Some(hidden) => set_selection_hidden(hidden),
        None => false,
    }
}

/// T-701 — SHOW ALL: clear `editorHidden` on EVERY slot in the doc in ONE undo step (the reveal-all
/// command, so a maker who hid several entities across the mission un-hides them all at once). Rides
/// `store::clear_all_editor_hidden` (one txn) + the shared tail. Returns the number of entities
/// un-hidden (0 ⇒ nothing was hidden, and no visible change).
///
/// `#[allow(dead_code)]`: a menu/command entry point (a "Show All" item) is the residue; the reveal-
/// all txn + undo are proven at the store (`show_all_clears_every_flag_in_one_txn`).
#[allow(dead_code)]
pub fn show_all_hidden() -> usize {
    let cleared = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        core.clear_all_editor_hidden()
    });
    if cleared > 0 {
        mission_history::after_local_edit();
    }
    cleared
}

/// T-701 — dock-facing accessor: every slot's `(id, editorHidden)` read straight off `slots_json`
/// (`slotsById`), so it lists HIDDEN slots too — the twin of [`layer_rows`]'s `hidden` field but
/// per-ENTITY. `slot_rows` (fed from `materialize`) deliberately DROPS hidden slots (they leave the
/// tree, the T-715 inherited lane), so a dock that wants to render a hidden entity dimmed-but-present
/// (an "eye-off" glyph, the Eden affordance) reads THIS instead. Sorted by id for deterministic order
/// (mirrors `layer_rows`). Pure over the doc — the render wiring (glyph, click-to-show) is the
/// T-733-family residue, but the DATA a dock needs ships here.
///
/// `#[allow(dead_code)]`: the consuming dock render (an eye-off glyph on the slot row) is outside
/// `owns` (`eden_tree`); this accessor is the shippable-visible datum it will read once wired.
#[allow(dead_code)]
#[must_use]
pub fn slot_hidden_rows(core: &MissionDocCore) -> Vec<(String, bool)> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    let Some(map) = root.as_object() else {
        return Vec::new();
    };
    let mut rows: Vec<(String, bool)> = map
        .iter()
        .map(|(id, v)| {
            let hidden = v
                .get("editorHidden")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            (id.clone(), hidden)
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

/* ═══════════════════════════ T-666 — Outliner layer authoring ═══════════════════════════ */
//
// LAYER-CREATE-001 / LAYER-DEL-001 / SEL-LAYER-CHILDREN-001 / SEL-LAYER-DESC-001 (the ops half;
// SEL-GROUP-ICON-001 is a pure render rule in `eden_tree`). These are thin wrappers onto the
// SHIPPED, TESTED core layer mutators (`add_editor_layer` / `rename_editor_layer` /
// `remove_editor_layer` (subtree + reseed) / `reparent_editor_layer` (cycle-guarded) /
// `move_slot_to_layer`) — verified at filing to have ZERO UI callers. Each wrapper opens exactly
// one `OPS_CTX` borrow, scopes the doc write so it drops before the tail, and rides
// `mission_history::after_local_edit()` — which is what calls `refresh_docks()` (via
// `refresh_signals`), so "call core + refresh_docks" is one tail, exactly like `set_layer_hidden`.
// The core mutators each commit a SINGLE transaction, so one authoring action = one undo step.

thread_local! {
    /// Monotonic minter for created-layer ids; uniqueness is still PROVEN against the live doc in
    /// [`mint_layer_id`] (undo frees ids; an IDB restore can bring back a doc that already used one).
    static NEXT_LAYER_ID: Cell<u32> = const { Cell::new(0) };
    /// LAYER-CREATE-001 — the id of the layer just created, so the tree can arm its inline rename
    /// immediately (spec: "inline-rename armed immediately"). The dock reads+clears this reactively.
    static RENAME_ARMED: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Armed drag between a tree-row `pointerdown` and a folder/root-dropzone `pointerup`
    /// (pointer-drag — the same idiom as [`PENDING_REFILE`], not HTML5 DnD, since the Leptos
    /// frontend has no HTML5-DnD lane). A folder row arms `Folder` (drop reparents); a slot row in
    /// the layer tree arms `Slot` (drop refiles). One latch so a folder's `pointerup` dispatches
    /// either without a second source of truth to get out of step.
    static PENDING_LAYER_DRAG: RefCell<Option<LayerDrag>> = const { RefCell::new(None) };
}

/// T-666 — what a tree pointer-drag is carrying (see [`PENDING_LAYER_DRAG`]).
#[derive(Clone)]
enum LayerDrag {
    /// A folder being reparented.
    Folder(String),
    /// A slot being refiled into a folder.
    Slot(String),
    /// T-651 — an editor-only COMMENT being refiled into a folder. A separate variant rather than
    /// reusing [`LayerDrag::Slot`] because the completion calls a different mutator: a comment id is
    /// a `commentsById` key, and `move_slot_to_layer` would happen to work today (it only shuffles
    /// `entityIds`) but would silently become wrong the moment slot refiling grows a squad or
    /// selection side effect. The variant makes the two intents unmistakable at the drop site.
    Comment(String),
}

/// Mint an unused `layer-{n}` id, proven unique against the doc's live layer set.
fn mint_layer_id(core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        layer_rows(core).into_iter().map(|l| l.id).collect();
    loop {
        let id = format!("layer-{}", NEXT_LAYER_ID.with(Cell::get));
        NEXT_LAYER_ID.with(|c| c.set(c.get().saturating_add(1)));
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// Auto-name a new layer "New Layer N" where N is the smallest positive integer not already used by
/// an existing "New Layer …" name (so creating three in a row reads 1/2/3, and a delete-then-create
/// reuses the gap). Names need not be unique in the doc; this is only a friendly default.
fn mint_layer_name(core: &MissionDocCore) -> String {
    let used: std::collections::HashSet<u32> = layer_rows(core)
        .iter()
        .filter_map(|l| {
            l.name
                .strip_prefix("New Layer ")?
                .trim()
                .parse::<u32>()
                .ok()
        })
        .collect();
    let mut n = 1u32;
    while used.contains(&n) {
        n += 1;
    }
    format!("New Layer {n}")
}

/// LAYER-CREATE-001 — create a folder as a **child of the selected/active folder** (or a root when
/// none is active), auto-named "New Layer N", and arm its inline rename. Returns the new id.
///
/// Rides the shipped `add_editor_layer`; one transaction ⇒ one undo step. The parent is the active
/// layer if it still exists (a stale pointer — folder deleted/undone-away — falls back to root,
/// mirroring `ensure_layer`'s staleness handling).
pub fn create_layer() -> Option<String> {
    let created = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let id = {
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let rows = layer_rows(core);
            // Parent = the active folder if it still exists, else root (None).
            let parent = ctx
                .active_layer
                .get_untracked()
                .filter(|a| rows.iter().any(|l| &l.id == a));
            let id = mint_layer_id(core);
            let name = mint_layer_name(core);
            core.add_editor_layer(&id, &name, parent);
            id
        };
        // Make the new folder the active drop target + arm its inline rename.
        ctx.active_layer.set(Some(id.clone()));
        RENAME_ARMED.with(|r| *r.borrow_mut() = Some(id.clone()));
        Some(id)
    });
    if created.is_some() {
        mission_history::after_local_edit();
    }
    created
}

/// LAYER-CREATE-001 — take the id of the just-created layer whose inline rename should open, if any.
/// Consumed once (cleared on read) so the dock arms the input exactly once per creation.
#[must_use]
pub fn take_rename_armed() -> Option<String> {
    RENAME_ARMED.with(|r| r.borrow_mut().take())
}

/// Rename an Outliner folder (inline-rename commit). Rides the shipped `rename_editor_layer`; one
/// transaction ⇒ one undo step. A blank name after trim is rejected (a folder must keep a label).
pub fn rename_layer(id: &str, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.rename_editor_layer(id, name);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// LAYER-DEL-001 — delete a folder with the SHIPPED subtree semantics: `remove_editor_layer`
/// deletes the folder AND its whole subtree (child folders + every slot filed in any of them), keeps
/// ≥1 layer (reseeding a default if the subtree was every layer). One transaction ⇒ one undo step.
///
/// This is destructive (the whole subtree), which is why the dock guards it behind a confirm whose
/// text says so. The reseed id is minted here so a "delete the only layer" reseed can't collide.
pub fn delete_layer(id: &str) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            let reseed = mint_layer_id(core);
            core.remove_editor_layer(id, &reseed);
        }
        // If the active drop target was inside the removed subtree it is now dangling; the next
        // place's `ensure_layer` re-resolves, but clear it eagerly so the header reads honestly.
        if ctx.active_layer.get_untracked().as_deref() == Some(id) {
            ctx.active_layer.set(None);
        }
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Reparent a folder (drag-in-tree / root-dropzone). Rides the cycle-guarded `reparent_editor_layer`
/// (a drop into the folder's own subtree is a no-op at the core), so this wrapper does not re-check
/// cycles. `new_parent = None` moves it to the root. One transaction ⇒ one undo step.
pub fn reparent_layer(id: &str, new_parent: Option<String>) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.reparent_editor_layer(id, new_parent);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Refile a slot into a different folder (drag a slot row onto a folder). Rides the shipped
/// `move_slot_to_layer` (detach from every folder holding it, append to the target); squad is
/// unchanged (workflow-only). One transaction ⇒ one undo step.
pub fn refile_slot_to_layer(slot_id: &str, layer_id: &str) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            core.move_slot_to_layer(slot_id, layer_id);
        }
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/* ═══════════════════ T-651 — editor comments / annotations (PLACE-COMMENT-001) ═══════════════════ */
//
// A comment is an EDITOR-ONLY VIRTUAL ENTITY. It shows in the Outliner, files into a layer, drags
// and copies like any other row — and it NEVER COMPILES. That last part is not enforced here and
// deliberately so: the exclusion is structural in `map-engine-core`
// (`mission::flatten::EditorPayload` declares no `comments` key, so serde drops the array before the
// mod document exists), proven by `comments_never_reach_the_mod_document`. Nothing in this file
// filters anything, which is why nothing in this file can forget to.
//
// CORPUS HONESTY: this feature is evidenced by ONE community across TWO eras — FNF v3's 28 in-map
// comments (including a seven-paragraph tutorial) and the TWO that survived v4's total rewrite. WOG
// and OFCRA have no comment equivalent. It is not a four-way convergence and must not be sold as
// one; the template seed is sized to the surviving evidence (two), not the peak (28).

/// T-651 — one comment as the docks need it. Mirrors [`crate::editor::panels::outliner::CommentRow`] plus the world
/// position the tree does not need but a drag/copy caller does.
#[derive(Clone, Debug, PartialEq)]
pub struct CommentDetail {
    pub id: String,
    pub title: String,
    pub tooltip: String,
    pub x: f64,
    pub z: f64,
}

/// T-651 — read `commentsById` into tree rows, sorted by id so the Unfiled bucket's order cannot
/// depend on `serde_json`'s map type (the `layer_rows` rule).
pub(super) fn comment_rows(core: &MissionDocCore) -> Vec<CommentRow> {
    comment_details(core)
        .into_iter()
        .map(|d| CommentRow {
            id: d.id,
            title: d.title,
            tooltip: d.tooltip,
        })
        .collect()
}

/// T-651 — every comment with its position, off the narrow [`MissionDocCore::comments_json`] getter.
#[must_use]
pub fn comment_details(core: &MissionDocCore) -> Vec<CommentDetail> {
    let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.comments_json()) else {
        return Vec::new();
    };
    let Some(obj) = map.as_object() else {
        return Vec::new();
    };
    let mut rows: Vec<CommentDetail> = obj
        .iter()
        .map(|(id, v)| {
            let pos = |k: &str| {
                v.get("position")
                    .and_then(|p| p.get(k))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0)
            };
            let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
            CommentDetail {
                id: id.clone(),
                title: s("title"),
                tooltip: s("tooltip"),
                x: pos("x"),
                z: pos("z"),
            }
        })
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows
}

/// T-651 — every comment in the live doc (dock/read helper).
#[must_use]
pub fn comment_list() -> Vec<CommentDetail> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        d.as_ref().map(comment_details).unwrap_or_default()
    })
}

/// T-651 — placed-comment count.
#[must_use]
pub fn comment_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.doc.borrow().as_ref().map(MissionDocCore::comment_count))
            .unwrap_or(0)
    })
}

/// Mint an unused `cmt-{n}` id, proven unique against the live comments map.
///
/// A separate counter namespace from [`mint_id`]'s `n{…}` on purpose: comment ids share the
/// `editorLayers[].entityIds` array with slot ids, and `build_outliner_with_comments` resolves a
/// filed id by looking it up in the slot map first and the comment map second. Disjoint prefixes
/// make that lookup unambiguous by construction rather than by luck.
fn mint_comment_id(core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.comments_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    let mut n = existing.len() + 1;
    loop {
        let id = format!("cmt-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n += 1;
    }
}

/// T-651 (`PLACE-COMMENT-001`) — **place a comment at world `(x, z)`**, the RMB-on-empty →
/// "Place Comment" gesture. Files it under the resolved active layer ([`ensure_layer`], the same
/// resolution a unit place uses) so it lands where the operator is working rather than in Unfiled.
/// Returns the new comment id.
///
/// The default title/tooltip are placeholders an operator overwrites; they are non-empty so the new
/// row is visible and clickable the instant it appears (the `SLOT_FALLBACK_LABEL` reasoning).
///
/// This does NOT touch the selection. A comment id is not a slot id, and pushing it into the
/// selection lane would put a non-slot into `set_selection` (engine tint), `delete_selection` and
/// the SEL readout — three surfaces that all index the slot SoA and would find nothing.
pub fn place_comment(x: f64, z: f64) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let id = mint_comment_id(core);
        let layer_id = ensure_layer(ctx, core);
        core.add_comment(&id, "Comment", "", x, z);
        core.move_comment_to_layer(&id, &layer_id);
        Some(id)
    })?;
    mission_history::after_local_edit();
    Some(id)
}

/// T-651 (ATTR-FIELD-CMT-TITLE) — retitle a comment (inline edit).
pub fn rename_comment(id: String, title: String) -> bool {
    edit_comment(|core| core.set_comment_title(&id, &title))
}

/// T-651 (ATTR-FIELD-CMT-TOOLTIP) — rewrite a comment's tooltip body (inline edit).
pub fn set_comment_tooltip(id: String, tooltip: String) -> bool {
    edit_comment(|core| core.set_comment_tooltip(&id, &tooltip))
}

/// T-651 (ATTR-FIELD-CMT-POSITION) — **DRAG commit**: move a comment to world `(x, z)`. One core
/// transaction ⇒ one undo step, so a drag is one Ctrl+Z exactly like a slot move.
pub fn move_comment(id: String, x: f64, z: f64) -> bool {
    edit_comment(|core| core.set_comment_position(&id, x, z))
}

/// T-651 — **COPY**: duplicate a comment `offset` metres to the south-east, keeping title and
/// tooltip. Returns the new id, or `None` when the source is gone. One undo step.
///
/// The offset exists so the copy is not perfectly stacked on its source: two comments at identical
/// coordinates are one indistinguishable row in the tree and one unclickable glyph on any future
/// map render.
pub fn duplicate_comment(id: &str, offset: f64) -> Option<String> {
    let new_id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let new_id = mint_comment_id(core);
        if !core.duplicate_comment(id, &new_id, offset, -offset) {
            return None;
        }
        // Land the copy in the same folder as its source, else the resolved active layer. A copy
        // that silently jumped to another folder would be a refile the operator never asked for.
        let layer_id = layer_rows(core)
            .into_iter()
            .find(|l| l.entity_ids.iter().any(|e| e == id))
            .map_or_else(|| ensure_layer(ctx, core), |l| l.id);
        core.move_comment_to_layer(&new_id, &layer_id);
        Some(new_id)
    })?;
    mission_history::after_local_edit();
    Some(new_id)
}

/// T-651 — delete a comment (also unfiles it from its folder — see `remove_comment`).
pub fn delete_comment(id: String) -> bool {
    edit_comment(|core| core.remove_comment(&id))
}

/// T-651 — **LAYERS**: file a comment into `layer_id` (the outliner drag drop). Mirrors
/// [`refile_slot_to_layer`]; one transaction ⇒ one undo step.
pub fn refile_comment_to_layer(comment_id: &str, layer_id: &str) -> bool {
    edit_comment(|core| core.move_comment_to_layer(comment_id, layer_id))
}

/// Shared edit tail for the comment mutators: run `f` against the core, then the dirty tail (one
/// undo step). Returns `false` when there is no doc. The [`edit_composition`] idiom.
fn edit_comment(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/* ═══════ T-672 — the editor-only CONNECTION GRAPH (CONN-START/SYNC/DEL-001, ACTION-FORM-001) ════ */
//
// A connection is an EDITOR-ONLY relation between two placed things. Like a comment it NEVER
// COMPILES, and — as with comments — that is not enforced HERE and deliberately so: the exclusion is
// structural in `map-engine-core` (`mission::flatten::EditorPayload` declares no `connections` key,
// and `mission.schema.json` declares no relation collection for one to land in), proven by
// `connections_never_reach_the_mod_document`. Nothing in this file filters anything, which is why
// nothing in this file can forget to.
//
// ── THE SHAPE OF THE CONNECT GESTURE, AND WHY IT IS NOT A DRAG ───────────────────────────────────
// The ticket's code hint proposed a third pointer mode: drag from entity to entity, arming in
// `mission_editor`'s `onpointerdown` and completing in `onpointerup`. That is NOT what ships here,
// and the reason is recorded rather than left to be rediscovered.
//
// The armed-pointerup path in `mission_editor.rs` is KNOWN DEFECTIVE and is filed as T-723: it has
// no `ev.button()` filter (so a middle- or right-button release fires the armed branch), it returns
// without taking `left`, stranding an `LG::Pending` that a later bare pointermove promotes into a
// phantom drag, and there is no Esc disarm at all. The invariant comment sitting beside it claiming
// "`left`/`pan_px` are both None here" is FALSE and was refuted four waves ago. Building a THIRD
// arming mode on that machine would inherit all three defects and add a fourth arming source to the
// thing that already cannot decide who owns a release.
//
// So the connect CREATE flow starts as two context-menu acts (T-672): right-click the source →
// `Connect ▸` → a kind, which arms; right-click the target → `Connect ▸ Complete` (or `Cancel`).
// T-768 wires the Eden LMB-target half on top of T-723's fixed armed-pointerup machine: after arming,
// a sub-threshold LMB pick on an entity calls [`complete_connect`] — the SAME mutator the RMB
// Complete row uses. No new LeftGesture variant, no new Pending place-arm: only a second CALLER.
// Esc / pointercancel / RMB Cancel / panel Cancel disarm. CONN-DEL line-select still needs a
// connections render lane (`mission_history` rebind + `draw_order`) — panel Delete remains the
// disclosed substitute.
//
// ── SEE + CHECK COME FIRST ───────────────────────────────────────────────────────────────────────
// [`connection_list`] and [`connection_findings`] are the halves the ticket's warning is about, and
// they are what the Connections panel in `mission_editor` renders. A connection has no map glyph in
// this slice (the `LaneRole::SquadLinks` trace is in the slice notes), so that panel is the ONLY
// surface on which an operator can observe or audit the graph. It is not a nice-to-have attached to
// the edge verbs; the edge verbs are attached to it.

/// T-672 — one connection as the panel renders it: the doc row plus a resolved label per endpoint.
///
/// Labels are best-effort (`"SL (s0)"` for a slot whose role is known, the bare id otherwise) and are
/// display-only — every VERB here takes the `id`, so a label that cannot be resolved degrades the
/// row's readability and nothing else. An unresolvable endpoint is exactly the `CONN-DANGLING` case
/// the findings list flags by id, which is why the label does not try to hide it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionListRow {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub from_label: String,
    pub to_label: String,
}

/// T-672 — one validation finding for the panel (`code` / `connection_id` / `detail`), mirroring
/// `map-engine-core`'s `ConnectionFinding` across the JSON getter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionFindingRow {
    pub code: String,
    pub connection_id: String,
    pub detail: String,
}

/// T-780 [wave 142 F-2] — does the live document actually hold an edge with this id?
///
/// Asked over `connection_rows_json`, the SAME stable listing the panel renders and the map lane is
/// built from, so "present" means one thing to the verb, to the reconcile and to the operator's
/// eyes. The core's `remove_connection` returns unit and so cannot answer "was it there?"; this
/// takes the answer BEFORE the write instead of inferring it from a count afterwards.
pub(super) fn connection_id_in_doc(core: &MissionDocCore, id: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.connection_rows_json()) else {
        return false;
    };
    rows.as_array().is_some_and(|a| {
        a.iter()
            .any(|r| r.get("id").and_then(serde_json::Value::as_str) == Some(id))
    })
}

/// T-780 [wave 142 F-1] — the same question [`delete_connection`] gates on, asked from the map's
/// Delete arm so a stale selection FALLS THROUGH to the entity delete instead of being handed to a
/// verb that can only answer `false`. One question, one implementation, two callers.
#[must_use]
pub fn connection_exists(id: &str) -> bool {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        d.as_ref()
            .is_some_and(|core| connection_id_in_doc(core, id))
    })
}

thread_local! {
    /// T-672 / T-768 — the armed half of the connect: `Some((kind, from_id))`.
    ///
    /// A plain `thread_local` and NOT a variant of place-arm `Pending`: place-arm is consumed by the
    /// palette stamp path. Connect stays separate so RMB Complete/Cancel and the LMB pick CALLER
    /// (T-768) both read the same cell without riding the place-arm enum. `mission_editor`'s
    /// LG::Pending click path and Esc/pointercancel consult this cell; they do not promote it into
    /// LeftGesture state.
    static PENDING_CONNECT: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
}

/// T-672 (`CONN-START-001`, act 1) — arm a connect of `kind` FROM `from_id`. Returns `false`
/// (arming nothing) for an unknown kind or an empty source, so a menu row that somehow dispatched
/// with a bad payload cannot leave the editor half-armed.
///
/// Re-arming REPLACES any previous arm rather than stacking: the operator changed their mind about
/// the source or the kind, and a queue of pending connects is not a thing anyone asked for.
pub fn arm_connect(kind: &str, from_id: &str) -> bool {
    // The AUTHORITY on this vocabulary is `MissionDocCore::add_connection`, which refuses an unknown
    // kind into the document. This check is about the ARM (a UI state, not a document one): see
    // `context_menu::ConnKind::parse`'s note for why the editor keeps its own copy and why the two
    // cannot diverge dangerously.
    if from_id.is_empty() || crate::editor::panels::context_menu::ConnKind::parse(kind).is_none() {
        return false;
    }
    PENDING_CONNECT.with(|p| {
        *p.borrow_mut() = Some((kind.to_string(), from_id.to_string()));
    });
    true
}

/// T-672 — the armed connect, if any: `(kind, from_id)`. Read by `context_menu::open` so the menu it
/// paints shows `Complete` / `Cancel` instead of the three kind rows — the arm is captured AT OPEN,
/// the same rule `MenuTarget::world` follows, so a state change while the menu is up cannot make a
/// visible row mean something else by the time it is clicked.
#[must_use]
pub fn pending_connect() -> Option<(String, String)> {
    PENDING_CONNECT.with(|p| p.borrow().clone())
}

/// T-672 — drop an armed connect without writing anything.
pub fn cancel_connect() {
    PENDING_CONNECT.with(|p| *p.borrow_mut() = None);
}

/// T-672 (`CONN-START-001` / `CONN-SYNC-001`, act 2) — complete the armed connect onto `to_id`.
/// `false` when nothing was armed or the core refused the edge (self-link, duplicate, empty
/// endpoint — see `MissionDocCore::add_connection`).
///
/// **The arm is consumed on ATTEMPT, not on success.** A refused edge that left the connect armed
/// would leave the operator in a mode they thought they had exited, and their very next right-click
/// would silently mean "connect" instead of "open a menu" — a stranded arm is the T-723 defect
/// shape, and this feature is not going to reproduce it one module over.
pub fn complete_connect(to_id: &str) -> bool {
    let Some((kind, from_id)) = PENDING_CONNECT.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    if to_id.is_empty() {
        return false;
    }
    let drawn = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let id = mint_connection_id(core);
        core.add_connection(&id, &kind, &from_id, to_id)
    });
    if drawn {
        mission_history::after_local_edit();
    }
    drawn
}

/// T-672 (`CONN-DEL-001`) — delete one edge by id. This is the verb the panel's per-row button
/// calls, and it is the whole of `CONN-DEL-001` in this slice: Eden deletes a connection by selecting
/// its LINE and pressing Del, and there is no line to select here (no render lane — see the slice
/// notes), so the addressable row IS the selection. One core transaction ⇒ one Ctrl+Z.
///
/// **The returned `bool` means "this connection was there and is now gone"** [wave 142 F-2]. It used
/// to mean something weaker and wrong: the guard was `connection_count() == 0`, a COUNT rather than
/// an id-presence check, so an id the document did not hold returned `true` whenever any OTHER edge
/// existed — `after_local_edit` then dirtied a mission that never changed. That is the same class
/// T-779 removed from `set_loadout` in the same wave (a verb inventing an acknowledgement for a
/// write that did not land), and T-780's map selection made it reachable: an undo, or a panel-side
/// delete, leaves the map holding an id the graph no longer has.
///
/// `MissionDocCore::remove_connection` returns unit and cannot report what it removed, so the answer
/// is taken from the document BEFORE the write ([`connection_id_in_doc`]) instead of guessed after
/// it. If the core mutator ever grows a `bool` like `add_connection` has, this gate becomes its
/// return value and the pre-read goes away.
pub fn delete_connection(id: &str) -> bool {
    let removed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        if !connection_id_in_doc(core, id) {
            return false;
        }
        core.remove_connection(id);
        true
    });
    if removed {
        mission_history::after_local_edit();
    }
    removed
}

/// T-672 (SEE) — every connection in the live doc, in `map-engine-core`'s stable listing order, with
/// endpoint labels resolved off the slot rows.
#[must_use]
pub fn connection_list() -> Vec<ConnectionListRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let labels: std::collections::HashMap<String, String> = slot_rows(core)
            .into_iter()
            .map(|r| {
                let label = if r.role.is_empty() {
                    r.id.clone()
                } else {
                    format!("{} ({})", r.role, r.id)
                };
                (r.id, label)
            })
            .collect();
        let label_of = |id: &str| labels.get(id).cloned().unwrap_or_else(|| id.to_string());
        let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.connection_rows_json())
        else {
            return Vec::new();
        };
        rows.as_array()
            .map(|a| {
                a.iter()
                    .map(|r| {
                        let s =
                            |k: &str| r.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let (from, to) = (s("from"), s("to"));
                        ConnectionListRow {
                            from_label: label_of(&from),
                            to_label: label_of(&to),
                            id: s("id"),
                            kind: s("kind"),
                            from,
                            to,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// T-672 (CHECK) — every validation finding over the live graph. The panel renders these beside the
/// rows; `connection_id` joins them to [`connection_list`].
#[must_use]
pub fn connection_findings() -> Vec<ConnectionFindingRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.connection_findings_json())
        else {
            return Vec::new();
        };
        rows.as_array()
            .map(|a| {
                a.iter()
                    .map(|f| {
                        let s =
                            |k: &str| f.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        ConnectionFindingRow {
                            code: s("code"),
                            connection_id: s("connectionId"),
                            detail: s("detail"),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// T-672 (`ACTION-FORM-001` / `CTX-FORMATION-001`) — snap the squad led by `leader_slot_id` onto its
/// formation positions. Returns the number of units moved; 0 (and no undo step) when the id leads no
/// squad, so firing the row on a rifleman is inert rather than a silent leadership change.
///
/// The geometry and the single-transaction guarantee live in `MissionDocCore::force_to_formation`;
/// this is the thin editor-side call plus the history tick, so the action is one Ctrl+Z.
pub fn force_to_formation(leader_slot_id: &str, formation: &str) -> usize {
    let moved = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        core.force_to_formation(leader_slot_id, formation)
    });
    if moved > 0 {
        mission_history::after_local_edit();
    }
    moved
}

/// T-672 — mint an unused `conn-{n}` id, proven unique against the live connections map. A separate
/// counter namespace from [`mint_id`] / [`mint_comment_id`] for the same reason theirs are separate:
/// disjoint prefixes make "what kind of thing is this id" answerable by construction.
fn mint_connection_id(core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.connections_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    let mut n = existing.len() + 1;
    loop {
        let id = format!("conn-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n += 1;
    }
}

// ── Pointer-drag reparent/refile in the tree (the T-037-era TreeView-DnD role, on the current
//    pointer idiom — mirrors ORBAT's `begin_refile`/`complete_refile_onto_squad`). A folder row
//    arms on `pointerdown`; dropping onto another folder reparents, onto the header root-dropzone
//    reparents to root, and a slot row reuses the same latch to refile.

/// Arm a folder for a pointer-drag reparent (folder-row `pointerdown`).
pub fn begin_layer_drag(layer_id: String) {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Folder(layer_id)));
}

/// Arm a slot for a pointer-drag refile into a folder (slot-row `pointerdown`, layer tree only).
pub fn begin_layer_slot_drag(slot_id: String) {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Slot(slot_id)));
}

/// T-651 — arm a COMMENT for a pointer-drag refile into a folder (comment-row `pointerdown`). This
/// is "comments support drag" in the tree: the same latch, the same folder-row `pointerup`
/// completion, a different mutator ([`refile_comment_to_layer`]).
pub fn begin_layer_comment_drag(comment_id: String) {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = Some(LayerDrag::Comment(comment_id)));
}

/// Drop an armed drag ANYWHERE that isn't a valid target (clear without mutating).
pub fn cancel_layer_drag() {
    PENDING_LAYER_DRAG.with(|p| *p.borrow_mut() = None);
}

/// Complete an armed drag onto `dest_folder_id`: a FOLDER drag reparents under it (no-op self/subtree
/// drop — the core rejects it); a SLOT drag refiles into it. `false` when nothing was armed.
pub fn complete_layer_drop_onto_folder(dest_folder_id: String) -> bool {
    let Some(drag) = PENDING_LAYER_DRAG.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    match drag {
        LayerDrag::Folder(id) => {
            if id == dest_folder_id {
                return false;
            }
            reparent_layer(&id, Some(dest_folder_id))
        }
        LayerDrag::Slot(slot_id) => refile_slot_to_layer(&slot_id, &dest_folder_id),
        // T-651 — a comment files into a folder exactly like a slot (same `entityIds` array).
        LayerDrag::Comment(comment_id) => refile_comment_to_layer(&comment_id, &dest_folder_id),
    }
}

/// Complete an armed FOLDER drag by reparenting it to the ROOT (the header dropzone). A slot drag is
/// dropped (a slot must live in some folder; "refile to no folder" is not a thing the doc models —
/// it would just leave the slot unfiled, and the root dropzone is a folder-reparent affordance).
/// `false` when nothing was armed or a slot was armed.
pub fn complete_layer_drop_onto_root() -> bool {
    let drag = PENDING_LAYER_DRAG.with(|p| p.borrow_mut().take());
    match drag {
        Some(LayerDrag::Folder(id)) => reparent_layer(&id, None),
        _ => false,
    }
}

// ── Folder-click selection (SEL-LAYER-CHILDREN-001 / SEL-LAYER-DESC-001). Reads the UNFILTERED
//    doc (`layer_rows` → `entity_ids`), NOT `materialize()`: see `eden_tree`'s module note — a
//    folder-click must select what the DOC says the layer contains, so a hidden layer still
//    selects its slots (the T-715 lane). No doc edit ⇒ selection-only tail, no undo step.

/// SEL-LAYER-CHILDREN-001 — select a folder's DIRECT slot children (replacing the selection).
pub fn select_layer_children(layer_id: &str) {
    let ids = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        Some(
            crate::editor::panels::outliner_tree::layer_direct_slot_children(
                &layer_rows(core),
                layer_id,
            ),
        )
    });
    if let Some(ids) = ids {
        set_slot_selection(ids);
    }
}

/// SEL-LAYER-DESC-001 — select every slot in a folder's whole subtree (replacing the selection).
pub fn select_layer_descendants(layer_id: &str) {
    let ids = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        Some(
            crate::editor::panels::outliner_tree::layer_descendant_slots(
                &layer_rows(core),
                layer_id,
            ),
        )
    });
    if let Some(ids) = ids {
        set_slot_selection(ids);
    }
}

/// Replace the slot selection with `ids` and run the selection-only tail a map/outliner click takes
/// (engine tint + SEL + dock highlight; no doc edit ⇒ no rebind/persist/undo). Shared by the two
/// folder-click selectors above.
fn set_slot_selection(ids: Vec<String>) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        *ctx.selection.borrow_mut() = ids;
        let ids = ctx.selection.borrow().clone();
        let mut eng = ctx.engine.borrow_mut();
        if let Some(e) = eng.as_mut() {
            e.set_selection(ids);
        }
    });
    mission_history::refresh_selection();
}

/// Palette leaf `pointerdown` → arm a place. Consumed by [`place_at`] on a canvas release, or
/// dropped by [`cancel_pending`] on a release over chrome.
///
/// T-180.5 — no-op while the Objects chip is active (stub catalog; place must not panic).
pub fn begin_place(payload: PlacePayload) {
    arm(Pending::Character(payload));
}

/// T-215 — Vehicles-palette leaf `pointerdown` → arm a **vehicle** place. Same lifecycle as
/// [`begin_place`] (consumed by [`place_at`], dropped by [`cancel_pending`]), so the map's
/// `has_pending` ghost and the release-over-chrome cancel need no vehicle-specific branch.
pub fn begin_place_vehicle(payload: PlacePayload) {
    arm(Pending::Vehicle(payload));
}

/// T-254 — Objects-palette leaf → arm an **entity** place (`entitiesById` / schema `entities[]`).
pub fn begin_place_object(payload: PlacePayload) {
    arm(Pending::Object(payload));
}

/// T-650 (COMP-PLACE-001) — Composition-palette row press → arm a **composition** place. Same
/// one-shot lifecycle as [`begin_place_object`] (consumed by [`place_at`] on a canvas release,
/// dropped by [`cancel_pending`] on a release over chrome); the canvas release re-anchors every
/// captured entity at the drop point and writes them as one undo step ([`place_composition_at`]).
pub fn begin_place_composition(composition_id: String) {
    arm(Pending::Composition(composition_id));
}

/// T-791 — the composition id currently armed for placement, or `None`. Backs the compositions
/// panel's LIVE armed-state hint ("click the map to place… · Esc to cancel"), the mirror of
/// [`armed_marker_icon`]. Reading it under the `doc_tick` heartbeat is what makes the hint appear on
/// [`begin_place_composition`] and vanish on [`cancel_pending`] (Esc / RMB / release over chrome),
/// since both bump that tick.
#[must_use]
pub fn armed_composition_id() -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Composition(id)) => Some(id.clone()),
            _ => None,
        }
    })
}

/// Arm a place. Objects mode only accepts [`Pending::Object`]; side modes reject Object so a
/// leftover Objects arm cannot commit after the chip switches away.
///
/// T-791 — the arm bumps the dock tick on the way out (via [`bump_doc_tick`], called OUTSIDE the
/// `OPS_CTX` borrow so the nested borrow inside it cannot re-enter and panic). That is what lets a
/// palette/composition surface show a LIVE armed-state hint that appears the instant a row is armed:
/// the compositions panel reads [`armed_composition_id`] off this tick, exactly as the Markers panel
/// reads [`armed_marker_icon`]. Arming writes no document row, so this is the light `bump_doc_tick`
/// (dock re-read only), not `refresh_docks`/`after_local_edit` (tree rebuild + persist).
pub(super) fn arm(pending: Pending) {
    OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let objects = ctx.objects_mode.get_untracked();
            let ok = match &pending {
                Pending::Object(_) => objects,
                Pending::Character(_) | Pending::Vehicle(_) => !objects,
                // T-650 — a composition arms from its OWN tab, independent of the Objects chip (like
                // a zone, it is neither a BLUFOR thing nor an Objects thing). Always accepted; the
                // Composition tab is the only surface that produces this arm.
                Pending::Composition(_) => true,
                // T-069 — a marker arms from its OWN tab for the same reason a composition does: a
                // marker is neither a BLUFOR thing nor an Objects thing. Its SIDE comes from the
                // active chip at DROP time (`side_faction_id` / `ensure_side_faction` in `place_at_impl`), not from the
                // Objects flag, so the Objects chip has nothing to say about whether it may arm.
                Pending::Marker(_) => true,
                // T-582 — zones are not a palette arm and do not route through here
                // ([`begin_zone_draw`] sets `pending` itself, because a zone is authorable under
                // either chip: a play area is not a BLUFOR thing or an Objects thing). Rejecting
                // rather than accepting keeps this function's contract "palette arms only".
                Pending::Zone(_) => false,
            };
            if !ok {
                *ctx.pending.borrow_mut() = None;
                return;
            }
            *ctx.pending.borrow_mut() = Some(pending);
        }
    });
    // T-791 — reflect the new armed state in the dock (armed-state hint). Outside the borrow above.
    bump_doc_tick();
}

/// T-650 — how many entities are currently selected (backs the "Save composition…" affordance,
/// which is shown only when a selection exists). The selection is app-side slot/vehicle ids.
#[must_use]
pub fn selection_len() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .map_or(0, |ctx| ctx.selection.borrow().len())
    })
}

/// Is a palette drag in flight? The `pointerup` handler asks before doing any work.
#[must_use]
pub fn has_pending() -> bool {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|ctx| ctx.pending.borrow().is_some())
    })
}

/// Drop the armed place (release over chrome, or pointercancel).
///
/// **T-582 — a zone draw survives this.** The palette arms are one-shot, so a release over a dock
/// means "I changed my mind" and dropping them is right. A zone draw is multi-click, and the chrome
/// host stops `pointerdown` only — so every click on the Zones panel's own Close / Undo vertex /
/// Cancel buttons ALSO bubbles a `pointerup` to the map container and lands here. Dropping the draft
/// on those would make the Close control destroy the ring it is meant to commit. A zone draw is
/// therefore ended only by finishing it or by [`cancel_zone_draw`], which the Cancel button calls
/// explicitly.
///
/// T-791 — when the arm is actually cleared (Esc / RMB / release over chrome), bump the dock tick so
/// the armed-state hint disappears — the "hint gone" half of the composition-place acceptance. A zone
/// draw survives (the early return), so its hint must NOT be torn down: the bump is gated on a real
/// clear and runs OUTSIDE the `OPS_CTX` borrow (`bump_doc_tick` borrows it again).
pub fn cancel_pending() {
    let cleared = OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let mut p = ctx.pending.borrow_mut();
            if matches!(*p, Some(Pending::Zone(_))) {
                return false;
            }
            let had = p.is_some();
            *p = None;
            return had;
        }
        false
    });
    if cleared {
        bump_doc_tick();
    }
}

/// Every slot id the document actually holds — the slot half of both minters' uniqueness universe,
/// read off the EXACT [`MissionDocCore::slots_json`] row map.
///
/// **Never `materialize()`.** The SoA is a VIEW: it drops slots on a hidden layer (T-665) and slots
/// carrying their own `editorHidden` flag (T-701) before any column is pushed, so a hidden slot's id
/// is INVISIBLE to a materialize-sourced uniqueness proof. Combined with [`OpsCtx::next_id`] resetting
/// to 0 on every editor mount, that is a silent-destruction path: restore a document from IDB whose
/// `n0` sits on a hidden layer, place anything, and the mint hands back `n0` — and the doc writes are
/// upserts (`add_slot` / `place_composition` insert a fresh row under the id), so the tucked-away slot
/// is overwritten inside the placement's undo step, with no error and no warning. Hidden slots are
/// precisely the work a careful author put out of sight, i.e. the work least likely to be noticed
/// missing.
///
/// This is the wave-127 rule for the id namespace: read `slots_json` (exact, complete), never the
/// SoA/materialized view, which is lossy about hidden slots — the same reason
/// [`capture_selection_entities`] and the shared `slot_z` reader were pointed at `slots_json`.
/// Cost is one O(all slots) parse per placement gesture, the same price `paste_at_cursor` and the
/// Attributes readers already pay; a place is a rare gesture, not the render hot path.
fn live_slot_ids(core: &MissionDocCore) -> std::collections::HashSet<String> {
    serde_json::from_str::<serde_json::Value>(&core.slots_json())
        .ok()
        .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
        .unwrap_or_default()
}

/// Mint an unused slot id. The counter keeps this O(1) amortized, but uniqueness is **proven**
/// against the live doc rather than assumed: undo frees ids, and an IDB restore can bring back a
/// document that already used `n0`.
///
/// The proof runs over [`live_slot_ids`] — the raw `slots_json` keys, hidden rows included — because
/// the materialized SoA cannot see a hidden slot and would let this hand back an id that is already
/// taken. See that helper for the full argument.
pub(super) fn mint_id(ctx: &OpsCtx, core: &MissionDocCore) -> String {
    let existing = live_slot_ids(core);
    loop {
        let id = format!("n{}", ctx.next_id.get());
        ctx.next_id.set(ctx.next_id.get().saturating_add(1));
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// T-650 — mint `count` unused ids at once, proven unique against slots AND vehicles AND entities
/// (a composition place mixes all three, so uniqueness against the slot SoA alone — [`mint_id`] —
/// is not enough). The union is read once; each minted id is also added to it so a run of ids inside
/// one call cannot collide with itself.
///
/// **T-781 — `commentsById` joined that union** the moment a composition could carry a comment. The
/// disjoint `cmt-`/`n` prefixes ([`mint_comment_id`]) keep hand-placed notes out of the way, but a
/// composition's comment is minted HERE and so gets an `n{k}` id like everything else; without this
/// key a second placement could re-mint an id an earlier composed note already holds and upsert it
/// away. Uniqueness is proven against the live doc rather than assumed, exactly as [`mint_id`]
/// argues: undo frees ids and an IDB restore can bring back a document that already used them.
///
/// **The slot half comes off [`live_slot_ids`], not `materialize()`** — a hidden slot is absent from
/// the SoA, so the old source let a placement re-mint (and upsert away) an id a hidden row already
/// held. The three small-map halves below need no such treatment: `small_maps_json` dumps those root
/// maps verbatim, with no visibility filter anywhere in it.
fn mint_ids(ctx: &OpsCtx, core: &MissionDocCore, count: usize) -> Vec<String> {
    let mut existing = live_slot_ids(core);
    if let Ok(small) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) {
        for key in ["vehiclesById", "entitiesById", "commentsById"] {
            if let Some(obj) = small.get(key).and_then(|v| v.as_object()) {
                existing.extend(obj.keys().cloned());
            }
        }
    }
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let id = format!("n{}", ctx.next_id.get());
        ctx.next_id.set(ctx.next_id.get().saturating_add(1));
        if existing.insert(id.clone()) {
            out.push(id);
        }
    }
    out
}

/// T-650 — the terrain bounds `[x0, y0, width, height]` for the live mission (the `paste_at_cursor`
/// idiom: read `meta.terrain` off `small_maps_json`, resolve bounds via the compile helper).
pub(super) fn terrain_bounds_of(core: &MissionDocCore) -> [f64; 4] {
    let terrain = serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
        .ok()
        .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
        .unwrap_or_default();
    map_engine_core::mission::compile::terrain_bounds(&terrain)
}

/// T-650 — does `id` name a live slot? (Used to keep only slot ids in the post-place selection —
/// SEL/tint run over the slot SoA.) Reads the materialized SoA once per call; a placement is a rare
/// gesture, not the render hot path.
fn slot_attrs_exists(core: &MissionDocCore, id: &str) -> bool {
    core.materialize().ids.iter().any(|s| s == id)
}

/// Resolve the drop target: the active layer if it still exists, else any existing layer (the
/// lexicographically first, so the choice is deterministic), else mint the default one. Mirrors
/// React's `activeLayerId ?? ensureDefaultLayer(md)`.
fn ensure_layer(ctx: &OpsCtx, core: &MissionDocCore) -> String {
    let rows = layer_rows(core);
    if let Some(active) = ctx.active_layer.get_untracked() {
        if rows.iter().any(|l| l.id == active) {
            return active;
        }
        ctx.active_layer.set(None); // stale pointer (folder deleted / undone away)
    }
    if let Some(first) = rows.first() {
        return first.id.clone();
    }
    core.add_editor_layer(DEFAULT_LAYER_ID, DEFAULT_LAYER_NAME, None);
    DEFAULT_LAYER_ID.to_string()
}

/// T-169 smoke hook — bulk-add `n` slots (each a new squad under BLUFOR), then refresh the docks,
/// so the virtual-outliner gate can push a tree past [`crate::editor::panels::outliner::VIRTUAL_SLOT_THRESHOLD`]
/// without 50 palette drags. Not on any UI path (the `__missionDoc` bridge exposes it for the gate).
pub fn debug_seed_slots(n: u32) {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        let layer_id = ensure_layer(ctx, core);
        for _ in 0..n {
            let id = mint_id(ctx, core);
            let _ = place_character_under_side(
                core, "BLUFOR", &id, &layer_id, "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
            );
        }
    });
    mission_history::after_local_edit();
}

/* ───────────────────────── T-180.7 — ORBAT Manager mutators ───────────────────────── */

/// Slot fields the ORBAT Manager inspector / `format_slot_line` need (from `slots_json`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OrbatSlotDetail {
    pub id: String,
    pub role: String,
    pub tag: String,
    pub callsign: String,
    pub rank: String,
    pub index: u32,
    pub squad_id: String,
    pub summary: String,
    pub primary: String,
    pub launcher: String,
}

/// Snapshot of squads/factions/slot details for the ORBAT Manager (one doc read).
#[derive(Clone, Debug, Default)]
pub struct OrbatManagerSnapshot {
    pub factions: Vec<crate::editor::panels::outliner::FactionRow>,
    pub squads: Vec<crate::editor::panels::outliner::SquadRow>,
    pub slots: Vec<OrbatSlotDetail>,
}

/// Read live ORBAT rows + per-slot loadout/identity for the Stitch manager (G7 live data).
pub fn orbat_manager_snapshot() -> OrbatManagerSnapshot {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return OrbatManagerSnapshot::default();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return OrbatManagerSnapshot::default();
        };
        OrbatManagerSnapshot {
            factions: faction_rows(core),
            squads: squad_rows(core),
            slots: slot_details(core),
        }
    })
}

/// T-659 — the live input the header's slot census reads: `(factions, squads, slot squad ids)`.
///
/// Reuses [`orbat_manager_snapshot`] (one doc read of the same rows the ORBAT Manager sees) rather
/// than re-deriving from the document — so the header badge and the ORBAT dock can never disagree
/// about who is on which side. The third element is one `squadId` per slot (empty string when the
/// slot carries none); its length is the total slot count, which is what makes the pure
/// [`crate::editor::panels::top_strip::census_from_rows`] buckets provably sum to the total. Vehicles are read
/// by the sibling [`vehicle_rows`] and are deliberately NOT folded in here: the header is a *slot*
/// (people) census, and mixing crewed vehicles into the same per-side integers would misreport the
/// roster the community naming convention is built on.
#[must_use]
pub fn census_input() -> (
    Vec<crate::editor::panels::outliner::FactionRow>,
    Vec<crate::editor::panels::outliner::SquadRow>,
    Vec<String>,
) {
    let snap = orbat_manager_snapshot();
    let slot_squad_ids = snap.slots.iter().map(|s| s.squad_id.clone()).collect();
    (snap.factions, snap.squads, slot_squad_ids)
}

fn slot_details(core: &MissionDocCore) -> Vec<OrbatSlotDetail> {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return Vec::new();
    };
    let Some(map) = root.as_object() else {
        return Vec::new();
    };
    map.values()
        .filter_map(|v| {
            let o = v.as_object()?;
            let lo = o.get("loadout").and_then(|l| l.as_object());
            Some(OrbatSlotDetail {
                id: o.get("id")?.as_str()?.to_string(),
                role: o
                    .get("role")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                tag: o
                    .get("tag")
                    .and_then(|t| t.as_str())
                    .unwrap_or_default()
                    .to_string(),
                callsign: o
                    .get("callsign")
                    .and_then(|c| c.as_str())
                    .unwrap_or_default()
                    .to_string(),
                rank: o
                    .get("rank")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                index: o
                    .get("index")
                    .and_then(|i| i.as_u64().or_else(|| i.as_i64().map(|n| n as u64)))
                    .unwrap_or(0) as u32,
                squad_id: o
                    .get("squadId")
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                summary: lo
                    .and_then(|m| m.get("summary"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                primary: lo
                    .and_then(|m| m.get("primary"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
                launcher: lo
                    .and_then(|m| m.get("launcher"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

/// Canonical `faction-{SIDE}` id for an Eden side chip. Does **not** mint a faction row.
///
/// T-826 — markers bind to this id (and the store may park the row until a real mint) without
/// calling [`ensure_side_faction`]. Slot / squad / vehicle / object authorship still mints.
fn side_faction_id(side: &str) -> String {
    format!("faction-{side}")
}

/// Ensure `faction-{SIDE}` exists in the doc (mint if missing).
///
/// Used by slot / squad / vehicle / object paths — authorship that declares players. **Markers
/// must not call this** (T-826 / F-11): a briefing mark stores a side without minting a phantom
/// faction that would flip validate's `declares_players` gate.
fn ensure_side_faction(core: &MissionDocCore, side: &str) -> String {
    let faction_id = side_faction_id(side);
    let factions = faction_rows(core);
    if !factions.iter().any(|f| f.id == faction_id) {
        core.add_faction(&faction_id, side, side);
    }
    faction_id
}

fn mint_squad_id_for_side(core: &MissionDocCore, side: &str) -> String {
    let existing: std::collections::HashSet<String> =
        squad_rows(core).into_iter().map(|s| s.id).collect();
    let mut n: u32 = 1;
    loop {
        let id = format!("squad-{side}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

/// G5 — add an empty squad under `side` (`BLUFOR`/`OPFOR`/`INDFOR`).
pub fn orbat_add_squad(side: String) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return None;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return None;
        };
        if !matches!(side.as_str(), "BLUFOR" | "OPFOR" | "INDFOR") {
            return None;
        }
        let faction_id = ensure_side_faction(core, &side);
        let squad_id = mint_squad_id_for_side(core, &side);
        let ordinal = faction_rows(core)
            .iter()
            .find(|f| f.id == faction_id)
            .map(|f| f.squad_ids.len())
            .unwrap_or(0);
        let name = format!("Squad {}", ordinal + 1);
        core.add_squad(&squad_id, &faction_id, &name, None);
        Some(squad_id)
    });
    if id.is_some() {
        mission_history::after_local_edit();
    }
    id
}

/// T-188 — along-front spacing between the slots of one squad, matching
/// `apply_faction_library`'s `APPLY_ANCHOR_X + 15.0 * i` formation so a hand-built squad and an
/// applied kit lay out identically.
const ORBAT_SLOT_SPACING_X: f64 = 15.0;

/// G6 — add a role (slot) into an existing squad; default role Rifleman. Not `place_character_under_side`.
///
/// **T-188 — placement.** This used to hand `add_slot` `0.0, 0.0, 0.0, 0.0`, so every role added
/// from the ORBAT dock materialised at world origin — the terrain's south-west corner, nowhere near
/// its squad, and stacked on every other Add Role. It now anchors the way the sibling
/// [`orbat_add_vehicle`] already does, off the squad's own geometry: see [`next_slot_xy`] for the
/// lane, and [`squad_anchor_xy`] for the empty-squad fallback.
pub fn orbat_add_slot(squad_id: String, role: String) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return None;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return None;
        };
        let sq = squad_rows(core).into_iter().find(|s| s.id == squad_id)?;
        let index = sq.slot_ids.len() as u32;
        let layer_id = ensure_layer(ctx, core);
        let slot_id = mint_id(ctx, core);
        let role = if role.trim().is_empty() {
            "Rifleman".to_string()
        } else {
            role
        };
        let (x, y) = next_slot_xy(core, &sq);
        let asset_id = asset_id_for_role(core, &sq, &role);
        core.add_slot(
            &slot_id,
            &squad_id,
            &layer_id,
            index,
            &role,
            None,
            asset_id.clone(),
            x,
            y,
            0.0,
            0.0,
        );
        // T-068.15.2 — a slot that carries a character carries its default cargo too; same borrow
        // scope ⇒ one undo step with the add, exactly like the place / apply-kit hooks.
        if let Some(a) = &asset_id {
            seed_cargo_in_core(core, &slot_id, a, None);
        }
        if sq.leader_slot_id.is_empty() {
            core.set_leader(&squad_id, &slot_id);
        }
        Some(slot_id)
    });
    if id.is_some() {
        mission_history::after_local_edit();
    }
    id
}

/// T-188 — best-effort `assetId` for a role added from the ORBAT dock.
///
/// Add Role has no character picker (the palette drag is the only place a character is chosen), so
/// the slot is minted with no `assetId` of its own. Derive one from the doc instead: the resource
/// name an existing slot with the **same role** already carries, nearest first — the squad itself,
/// then sibling squads under the same faction. Deterministic: both tiers walk ordered doc arrays
/// (`faction.squadIds` / `squad.slotIds`), never the unordered `slotsById` map, so two identical
/// missions pick the same character.
///
/// Deliberately role-keyed, not "any squad-mate": borrowing the squad leader's prefab for a
/// Rifleman would swap the character out for the wrong one, which is worse than leaving it unset.
///
/// **Scope is the faction, and stops there (wave-1 fix).** A third tier used to sweep the whole
/// mission (`obj.keys()`), matching on `role` alone with no faction predicate anywhere. That is a
/// cross-faction character leak, and it is reachable with stock data: `faction-library.sample.json`
/// defines only `Rifleman` / `Squad Leader`, both on `Character_USSR_Rifleman.et`, and both ORBAT
/// "Add Role" buttons hardcode `"Rifleman"` — apply the Soviet template, add a BLUFOR squad, hit
/// Add Role, and tiers 1–2 come up empty while tier 3 hands the BLUFOR slot a USSR body. It reaches
/// the game intact: `kit-aliases.json` maps that resource to `kit:sov_rifleman` and
/// `mission::flatten` resolves the kit per-`assetId`.
///
/// Returning `None` is the **correct** outcome, not a gap. `flatten` falls back to
/// `KitAliases::faction_default(faction_key)` for a slot with no `assetId`, i.e. the slot compiles
/// to its own faction's default kit — so tier 3 traded a faction-correct default for a possibly
/// faction-wrong pick. Tiers 1–2 already cover every character the faction owns; anything beyond
/// them is by definition another faction's.
fn asset_id_for_role(
    core: &MissionDocCore,
    sq: &crate::editor::panels::outliner::SquadRow,
    role: &str,
) -> Option<String> {
    let root = serde_json::from_str::<serde_json::Value>(&core.slots_json()).ok()?;
    let obj = root.as_object()?;

    let squads = squad_rows(core);
    let siblings = faction_rows(core)
        .into_iter()
        .find(|f| f.id == sq.faction_id)
        .map(|f| f.squad_ids)
        .unwrap_or_default();

    let mut candidates: Vec<&String> = sq.slot_ids.iter().collect();
    for sid in &siblings {
        if let Some(s) = squads.iter().find(|s| s.id == *sid) {
            candidates.extend(s.slot_ids.iter());
        }
    }

    candidates.into_iter().find_map(|id| {
        let slot = obj.get(id)?;
        if slot.get("role").and_then(serde_json::Value::as_str) != Some(role) {
            return None;
        }
        slot.get("assetId")
            .and_then(serde_json::Value::as_str)
            .filter(|a| !a.is_empty())
            .map(ToString::to_string)
    })
}

/// G2 — Make SL via core `set_leader` (does not overwrite MED/ENG tag).
pub fn orbat_set_leader(squad_id: String, slot_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_leader(&squad_id, &slot_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Remove a slot (cascade detach); GC empty squad; promote leader when needed.
pub fn orbat_remove_slot(slot_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let detail = slot_details(core)
            .into_iter()
            .find(|s| s.id == slot_id)
            .unwrap_or_default();
        let squad_id = detail.squad_id.clone();
        if squad_id.is_empty() {
            return false;
        }
        let sq = squad_rows(core).into_iter().find(|s| s.id == squad_id);
        let was_leader = sq.as_ref().is_some_and(|s| s.leader_slot_id == slot_id);
        let remaining: Vec<String> = sq
            .map(|s| s.slot_ids.into_iter().filter(|id| id != &slot_id).collect())
            .unwrap_or_default();
        core.remove_slots(vec![slot_id]);
        if remaining.is_empty() {
            core.remove_squad(&squad_id);
        } else if was_leader {
            if let Some(next) = remaining.first() {
                core.set_leader(&squad_id, next);
            }
        }
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Remove a squad and its slots.
pub fn orbat_remove_squad(squad_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.remove_squad(&squad_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Rename a squad.
pub fn orbat_rename_squad(squad_id: String, name: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.rename_squad(&squad_id, &name);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-180.8 — REPLACE-apply a Faction Library doc onto `side` (H-L2 / H-L7b).
///
/// **T-308 — returns the refusal instead of swallowing it.** This used to be `-> bool` over
/// `apply_faction_library(...).is_ok()`, so the one thing the operator needed — *which* squads
/// block the apply and how to clear them — was discarded one line after it was computed, and the
/// dialog printed "Apply failed." on top of a confirm the operator had already accepted. The
/// `Err` string is `ApplyFactionError`'s own `Display`; the other two arms cover the cases where
/// there is no document to apply onto at all (previously also a bare `false`).
pub fn orbat_apply_faction(side: String, doc: FactionDoc) -> Result<(), String> {
    let res = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Err("No mission editor is open.".to_string());
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Err("No mission document is loaded.".to_string());
        };
        let layer_id = ensure_layer(ctx, core);
        let input = FactionLibraryInput {
            name: doc.name,
            roles: doc
                .roles
                .into_iter()
                .map(|r| FactionLibraryRole {
                    role: r.role,
                    tag: r.tag,
                    character: r.character,
                    loadout: r.loadout,
                })
                .collect(),
            vehicles: doc
                .vehicles
                .into_iter()
                .map(|v| FactionLibraryVehicle {
                    vehicle: v.vehicle,
                    label: v.label,
                })
                .collect(),
        };
        apply_faction_library(core, &side, &layer_id, &input).map_err(|e| e.to_string())?;
        // T-068.15.2 — seed default cargo after a kit apply. Cargo-key-absent
        // slots only, so user edits and library-carried cargo[] are preserved;
        // same borrow scope ⇒ one undo step with the apply.
        if let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) {
            if let Some(obj) = map.as_object() {
                for (sid, slot) in obj {
                    let Some(rn) = slot
                        .get("assetId")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                    else {
                        continue;
                    };
                    let lo = slot
                        .get("loadout")
                        .filter(|l| !l.is_null())
                        .map(|l| l.to_string());
                    seed_cargo_in_core(core, sid, rn, lo.as_deref());
                }
            }
        }
        Ok(())
    });
    if res.is_ok() {
        mission_history::after_local_edit();
    }
    res
}

/// T-180.8 — `add_vehicle` + `attach_vehicle` with map position (H-L7 / H8).
pub fn orbat_add_vehicle(squad_id: String, resource_name: String) -> Option<String> {
    if resource_name.trim().is_empty() {
        return None;
    }
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return None;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return None;
        };
        let sq = squad_rows(core).into_iter().find(|s| s.id == squad_id)?;
        let n = sq.vehicle_ids.len();
        let vehicle_id = mint_id(ctx, core);
        // Place near squad leader / first slot, else Everon center.
        let (x, y) = squad_anchor_xy(core, &sq).unwrap_or((APPLY_ANCHOR_X, APPLY_ANCHOR_Y));
        let x = x + 30.0 + 20.0 * n as f64;
        let y = y - 30.0;
        core.add_vehicle(
            &vehicle_id,
            &resource_name,
            Some(x),
            Some(y),
            Some(0.0),
            Some(0.0),
        );
        core.attach_vehicle(&squad_id, &vehicle_id);
        Some(vehicle_id)
    });
    if id.is_some() {
        mission_history::after_local_edit();
        // T-809 wave-203 — head the recently-placed list with the added vehicle, keyed on its
        // `resourceName` (the SAME `asset_id` a Vehicles-palette leaf records, so a re-add through
        // either path dedups to one entry). Recorded only on a real add, after the doc borrow closes;
        // the ops layer knows only the resourceName, so it doubles as the label. A no-op when the dock
        // is unmounted — not a dropped ack: the vehicle is already in the doc.
        crate::editor::panels::dock_right::record_placed(
            resource_name.clone(),
            resource_name.clone(),
        );
    }
    id
}

/// T-215 — the vehicle half of [`place_at`]: a `vehiclesById` row at the operator's map point,
/// owned by the active Eden side. Returns `false` on an unknown side (the same refusal
/// `place_character_under_side` gives), leaving the doc untouched.
///
/// **Why this does not attach the vehicle to a squad.** The obvious symmetry with the character path
/// would be `attach_vehicle` onto the side's current squad — and it is wrong.
/// `place_orbat::is_open_for_placement` treats a squad holding *any* vehicle as authored, so the
/// attach would close the side's current squad and the very next character placement would mint a
/// fresh one. Placing a truck would silently split the fireteam being built around it, which is the
/// one-squad-per-click defect T-321 exists to prevent. The side is recorded on the vehicle itself
/// (`factionId`) instead; `attach_vehicle` remains the way to say "this squad's vehicle", from the
/// ORBAT Manager where that is what the operator means.
///
/// `z`/`rotation` are `0.0` for the same reason the character path uses them: the flat-map commit
/// has no DEM sample yet. Heading is authored afterwards, not guessed at drop.
fn place_vehicle_in_core(
    core: &MissionDocCore,
    side: &str,
    vehicle_id: &str,
    resource_name: &str,
    x: f64,
    y: f64,
    with_crew: bool,
) -> bool {
    if !matches!(side, "BLUFOR" | "OPFOR" | "INDFOR") || resource_name.trim().is_empty() {
        return false;
    }
    let faction_id = ensure_side_faction(core, side);
    // T-732 — add + factionId + crewed stamp in ONE LOCAL txn (was three Ctrl+Z for unmanned).
    core.place_vehicle_with_crew_stamp(
        vehicle_id,
        resource_name,
        x,
        y,
        0.0,
        0.0,
        &faction_id,
        with_crew,
    );
    true
}

/// T-254 — Objects half of [`place_at`]: an `entitiesById` row at the map point.
///
/// `alias` is derived from the ResourceName + display label (`derive_object_alias`); `faction`
/// is the schema factionKey slug (`blufor`/`opfor`/`indfor`) from the active Eden side.
fn place_object_in_core(
    core: &MissionDocCore,
    side: &str,
    entity_id: &str,
    payload: &PlacePayload,
    x: f64,
    y: f64,
) -> bool {
    if !matches!(side, "BLUFOR" | "OPFOR" | "INDFOR") || payload.asset_id.trim().is_empty() {
        return false;
    }
    let alias = crate::editor::arsenal::asset_catalog::derive_object_alias(
        &payload.asset_id,
        &payload.role,
    );
    if alias.is_empty() {
        return false;
    }
    // Ensure the side's faction row exists (same as vehicles) so the mission graph stays coherent,
    // but write the schema factionKey on the entity itself — not `faction-{SIDE}`.
    let _ = ensure_side_faction(core, side);
    let faction = side.to_lowercase();
    core.add_entity(entity_id, &alias, &payload.asset_id, x, y, 0.0, 0.0);
    core.set_entity_faction(entity_id, &faction);
    true
}

/// T-215 — one authored cargo row on a vehicle: `{item, qty}`, `mission.schema.json`
/// `$defs/entityInventory`. No `container` key — for an entity the container *is* the entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VehicleCargoRow {
    pub item: String,
    pub qty: i64,
}

/// T-215 — one vehicle as the docks need it: identity, where it is, and what it carries.
#[derive(Clone, Debug, PartialEq)]
pub struct VehicleRow {
    pub id: String,
    pub resource_name: String,
    /// `None` for a vehicle that has never been given a map position — the state every
    /// ORBAT-added vehicle was in before this ticket, and still is when added from the ORBAT
    /// Manager without a drop.
    pub xy: Option<(f64, f64)>,
    /// Authored heading (degrees). `None` when unplaced; `Some(0.0)` is a real authored zero.
    pub rotation: Option<f64>,
    /// Elevation when placed; `None` when unplaced.
    pub z: Option<f64>,
    /// `faction-{SIDE}` when the vehicle was map-placed; empty when it only has a squad.
    pub faction_id: String,
    /// Empty when the vehicle is not attached to a squad (every map-placed vehicle).
    pub squad_id: String,
    pub cargo: Vec<VehicleCargoRow>,
    /// T-076 — the vehicle's crew map: `seat_id → slot_id`. Empty when nobody is boarded. Read off
    /// the same `crew` object the core writes ([`MissionDocCore::assign_crew_seat`]); the panel joins
    /// each generic seat (driver/gunner/commander/cargoN) against this to show who occupies it.
    pub crew: std::collections::HashMap<String, String>,
}

/// Read every `vehiclesById` row for the docks. Off `small_maps_json`, like [`squad_rows`] —
/// vehicles are deliberately off the slot SoA, so there is no columnar reader to use instead.
#[must_use]
pub fn vehicle_rows() -> Vec<VehicleRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
            return Vec::new();
        };
        let Some(map) = root.get("vehiclesById").and_then(|v| v.as_object()) else {
            return Vec::new();
        };
        let mut rows: Vec<VehicleRow> = map
            .iter()
            .map(|(id, v)| {
                let s = |k: &str| {
                    v.get(k)
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string()
                };
                let pos = v.get("position");
                VehicleRow {
                    id: id.clone(),
                    resource_name: s("resourceName"),
                    xy: pos.and_then(|p| Some((p.get("x")?.as_f64()?, p.get("y")?.as_f64()?))),
                    rotation: pos.and_then(|p| p.get("rotation")?.as_f64()),
                    z: pos.and_then(|p| p.get("z")?.as_f64()),
                    faction_id: s("factionId"),
                    squad_id: s("squadId"),
                    cargo: v
                        .get("cargo")
                        .and_then(|c| c.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|r| {
                                    Some(VehicleCargoRow {
                                        item: r.get("item")?.as_str()?.to_string(),
                                        qty: r.get("qty")?.as_i64()?,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    // T-076 — join key for the seat list: `{seat_id: slot_id}` off the vehicle row.
                    crew: v
                        .get("crew")
                        .and_then(|c| c.as_object())
                        .map(|o| {
                            o.iter()
                                .filter_map(|(seat, slot)| {
                                    Some((seat.clone(), slot.as_str()?.to_string()))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            })
            .collect();
        // `small_maps_json` is a serde_json object (key-sorted), but sort explicitly so the dock
        // order is a property of this reader rather than of a JSON implementation detail.
        rows.sort_by(|a, b| a.id.cmp(&b.id));
        rows
    })
}

/// **T-819 — live crewed-slot id set** (every `vehicle.crew` value on the hosted doc).
///
/// Derived from [`vehicle_rows`]; the same set `mission_editor::crewed_slot_ids` computes from
/// `small_maps_json`. Docks / diagnostics that need "who is boarded right now" read here — never
/// invent a parallel flag on the slot row.
#[must_use]
pub fn crewed_slot_ids() -> std::collections::HashSet<String> {
    vehicle_rows()
        .into_iter()
        .flat_map(|v| v.crew.into_values())
        .filter(|id| !id.is_empty())
        .collect()
}

/// T-215 — replace a vehicle's authored cargo, then the shared dirty tail (one undo step).
/// Rows the schema cannot represent are dropped by the core mutator; an empty list clears the key.
pub fn set_vehicle_cargo(vehicle_id: String, rows: Vec<VehicleCargoRow>) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let pairs: Vec<(String, i64)> = rows.into_iter().map(|r| (r.item, r.qty)).collect();
        core.set_vehicle_cargo(&vehicle_id, &pairs);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-425 — placed vehicle positions for map pick/marquee (`id`, world x, world y).
#[must_use]
pub fn vehicle_points() -> Vec<(String, f64, f64)> {
    vehicle_rows()
        .into_iter()
        .filter_map(|v| v.xy.map(|(x, y)| (v.id, x, y)))
        .collect()
}

/// T-425 — author a vehicle's heading (degrees). Preserves x/y/z. No-op if missing/unplaced.
pub fn set_vehicle_heading(vehicle_id: String, heading_deg: f64) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
            return false;
        };
        let Some(v) = root.get("vehiclesById").and_then(|m| m.get(&vehicle_id)) else {
            return false;
        };
        let Some(pos) = v.get("position") else {
            return false;
        };
        let (Some(x), Some(y)) = (
            pos.get("x").and_then(|n| n.as_f64()),
            pos.get("y").and_then(|n| n.as_f64()),
        ) else {
            return false;
        };
        let z = pos.get("z").and_then(|n| n.as_f64()).unwrap_or(0.0);
        core.set_vehicle_position(&vehicle_id, x, y, z, heading_deg);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-425 — drag-release commit for placed vehicles (shared world delta). Rotation preserved.
pub fn move_vehicles(ids: Vec<String>, dx: f64, dy: f64) -> bool {
    if ids.is_empty() || (dx == 0.0 && dy == 0.0) {
        return false;
    }
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.move_vehicles(&ids, dx, dy);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-425 — true when `id` is a `vehiclesById` key (used by the select/drag gesture to route
/// commits away from the slot SoA `move_entities` path).
#[must_use]
pub fn is_vehicle_id(id: &str) -> bool {
    vehicle_rows().iter().any(|v| v.id == id)
}

/// T-215 — delete a placed vehicle (the map palette can create them, so something must be able to
/// remove them). Core [`MissionDocCore::remove_vehicle`] also detaches it from any squad.
pub fn remove_vehicle(vehicle_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.remove_vehicle(&vehicle_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-076 — one placed slot as the crew seat picker needs it: the doc id plus a human-readable label
/// (its resolved role, or the raw id when a slot has no role yet). Every placed character is a
/// candidate crew member, so the picker's options are exactly [`slot_rows`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedSlotChoice {
    pub id: String,
    pub label: String,
}

/// T-076 — the placed slots a crew seat can be assigned to (board target list for the seat picker).
/// Sorted by label then id so the dropdown order is a property of this reader, not of SoA iteration.
#[must_use]
pub fn placed_slot_choices() -> Vec<PlacedSlotChoice> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let mut rows: Vec<PlacedSlotChoice> = slot_rows(core)
            .into_iter()
            .map(|s| {
                let label = if s.role.is_empty() {
                    s.id.clone()
                } else {
                    format!("{} ({})", s.role, s.id)
                };
                PlacedSlotChoice { id: s.id, label }
            })
            .collect();
        rows.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
        rows
    })
}

/// T-076 — board a placed slot into a vehicle seat (`vehicle.crew[seat_id] = slot_id`), then the
/// shared dirty tail (one undo step). The **one-seat-per-slot** rule is enforced by the core mutator
/// [`MissionDocCore::assign_crew_seat`], which vacates `slot_id` from any other seat of any vehicle
/// before writing — so this reader-agnostic op cannot author a soldier into two vehicles at once.
///
/// **T-819 — no document hide flag.** Boarding writes ONLY the vehicle's `crew` map. The map-render
/// lane derives "figure leaves the map" from that assignment (`mission_editor::map_render_slot_soa`);
/// this op must never call `set_slots_editor_hidden` / stamp `editorHidden` — unassign, vehicle
/// delete, and undo restore visibility automatically because the crew ref is gone.
pub fn assign_crew_seat(vehicle_id: String, seat_id: String, slot_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.assign_crew_seat(&vehicle_id, &seat_id, &slot_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-076 — unboard: clear one vehicle seat. Core [`MissionDocCore::clear_crew_seat`] removes the
/// `crew` key once the last seat empties, so an unboard restores the pre-board row shape.
///
/// **T-819 — figure returns by derivation.** Clearing the seat drops the id from
/// [`crewed_slot_ids`]; the next map rebind puts the figure back at its stored `slots_json`
/// position (exact f64 `z` untouched — we never wrote a hide flag or moved the slot).
pub fn clear_crew_seat(vehicle_id: String, seat_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.clear_crew_seat(&vehicle_id, &seat_id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-180.8 — inverse of Apply: build a FactionDoc from the live side graph (Save / Save as).
///
/// **T-373 — what comes back is a PARTIAL, not a document.** Two `faction-library.schema.json`
/// fields have no representation anywhere in the mission graph, so this can only ever emit `None`
/// for them, no matter how the operator authored the library entry:
///
/// - **`emblem`** — the ORBAT has no emblem concept at all. Nothing in `MissionDocCore` stores one,
///   so there is nothing here to read it back out of.
/// - **vehicle `label`** — a mission vehicle row is `{id, resourceName, position, squadId}` and
///   nothing else (`map-engine-core` `doc/store.rs::add_vehicle`), and `apply_faction_library`
///   throws the library's label away on the way IN —
///   `let _ = v.label; // label is UI-only; resourceName is the graph pin`
///   (`crates/map-engine-core/src/doc/apply_faction.rs:358`).
///
/// That matters because `PUT /factions/:id` is a whole-document **replace**
/// (`apps/website/api/src/handlers/factions.rs::update_faction`) and [`FactionDoc`]'s
/// `skip_serializing_if = "Option::is_none"` **omits** an absent key rather than nulling it. PUT
/// this straight back and the operator's emblem and every authored vehicle label are deleted from
/// the library, silently, on every save. That was T-373.
///
/// Everything else here is faithful and may be trusted as authored state: `role`, `tag`,
/// `character` and `loadout` all live on the slot, Apply writes them there
/// (`apply_faction.rs` `update_slot_loadout`), and the Arsenal edits them — so a role that comes
/// back with no loadout genuinely has no loadout, it is not a gap in the derivation.
///
/// **Callers that CREATE (`Save as` → POST) may use this as-is** — a brand-new faction has no
/// emblem and no labels to lose. **Callers that REPLACE (`Save` → PUT) must merge over the stored
/// document first:** [`crate::pages::operations::orbat_manager::merge_faction_doc_from_side`].
///
/// `None` when there is no live doc, or when the doc's own JSON will not parse — see
/// [`faction_doc_from_side_core`].
pub fn faction_doc_from_side(side: &str) -> Option<FactionDoc> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        faction_doc_from_side_core(core, side)
    })
}

/// `None` when `slots_json` / `small_maps_json` will not parse.
///
/// **T-373 — a parse failure is not an empty side.** These two early-outs used to return
/// `FactionDoc { side, name, ..Default::default() }`, i.e. a doc with `roles: []` and
/// `vehicles: []` — byte-identical to what a side that genuinely holds no squads produces. Handed
/// to the PUT that replaces the whole library document, that turned "the mission's JSON is
/// malformed" into "delete every role and vehicle in this template", and handed to `Save as` it
/// created an empty faction and reported success. A distinguishable `None` lets the caller say so
/// instead of writing.
fn faction_doc_from_side_core(core: &MissionDocCore, side: &str) -> Option<FactionDoc> {
    let factions = faction_rows(core);
    let squads = squad_rows(core);
    let faction = factions.iter().find(|f| f.key == side);
    let name = faction
        .map(|f| f.name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| side.to_string());
    let squad_ids: Vec<String> = faction.map(|f| f.squad_ids.clone()).unwrap_or_default();
    let Ok(slots_root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return None;
    };
    let Ok(small) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) else {
        return None;
    };
    let mut roles = Vec::new();
    let mut vehicles = Vec::new();
    for sid in &squad_ids {
        let Some(sq) = squads.iter().find(|s| s.id == *sid) else {
            continue;
        };
        for slot_id in &sq.slot_ids {
            let Some(slot) = slots_root.get(slot_id) else {
                continue;
            };
            roles.push(FactionRole {
                role: slot
                    .get("role")
                    .and_then(|r| r.as_str())
                    .unwrap_or("Rifleman")
                    .to_string(),
                tag: slot
                    .get("tag")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .filter(|s| !s.is_empty()),
                character: slot
                    .get("assetId")
                    .and_then(|a| a.as_str())
                    .unwrap_or_default()
                    .to_string(),
                loadout: slot.get("loadout").cloned(),
            });
        }
        for vid in &sq.vehicle_ids {
            let Some(v) = small.get("vehiclesById").and_then(|m| m.get(vid)) else {
                continue;
            };
            vehicles.push(FactionVehicle {
                vehicle: v
                    .get("resourceName")
                    .and_then(|r| r.as_str())
                    .unwrap_or_default()
                    .to_string(),
                // T-373 — structurally inexpressible, not an oversight: the graph row this reads
                // (`vehiclesById[vid]`) has no label field to read. See the fn doc.
                label: None,
            });
        }
    }
    Some(FactionDoc {
        side: side.into(),
        name,
        // T-373 — likewise inexpressible. `merge_faction_doc_from_side` carries the stored one
        // over; do NOT PUT this partial as a replacement body.
        emblem: None,
        roles,
        vehicles,
    })
}

/// One slot's map position out of a parsed `slots_json`. `None` when the id is stale (the slot was
/// removed) or the stored position is malformed.
fn slot_xy(root: &serde_json::Value, id: &str) -> Option<(f64, f64)> {
    let pos = root.get(id)?.get("position")?;
    Some((pos.get("x")?.as_f64()?, pos.get("y")?.as_f64()?))
}

/// The squad's anchor out of an already-parsed `slots_json`: the leader if it still exists, else the
/// first slot that does. `None` only for a squad with nothing live to anchor against, which leaves
/// the fallback to the caller.
///
/// **T-188 wave-1 fix — chained, not single-shot.** This used to resolve exactly one id (leader when
/// set, otherwise `slot_ids.first()`) and return `None` the moment that lookup missed.
/// [`delete_selection`] deletes straight through `core.remove_slots`, which rewrites `slotIds` but
/// never touches `leaderSlotId` — so deleting the squad leader off the map left a **stale** anchor
/// id, `root.get(&anchor_id)` missed, and both callers fell through to
/// `unwrap_or((APPLY_ANCHOR_X, APPLY_ANCHOR_Y))`: the new slot / vehicle materialised at the Everon
/// centre, the exact world-origin symptom T-188 was written to stop. Walking leader → every live
/// slot keeps the squad's own geometry as the anchor and reserves the constant for a squad that
/// genuinely has no body left.
fn squad_anchor_in(
    root: &serde_json::Value,
    sq: &crate::editor::panels::outliner::SquadRow,
) -> Option<(f64, f64)> {
    std::iter::once(sq.leader_slot_id.as_str())
        .chain(sq.slot_ids.iter().map(String::as_str))
        .filter(|id| !id.is_empty())
        .find_map(|id| slot_xy(root, id))
}

fn squad_anchor_xy(
    core: &MissionDocCore,
    sq: &crate::editor::panels::outliner::SquadRow,
) -> Option<(f64, f64)> {
    let root = serde_json::from_str::<serde_json::Value>(&core.slots_json()).ok()?;
    squad_anchor_in(&root, sq)
}

/// T-188 — where the **next** slot added to `sq` belongs, in the squad's `ORBAT_SLOT_SPACING_X`
/// row. Falls back to the Everon centre only for a squad with nothing to anchor against — the same
/// origin `apply_faction_library` mints a fresh squad at.
///
/// **Wave-1 fix — the lane comes from geometry, not from a count.** It used to be
/// `anchor + 15 m * slot_ids.len()`. `remove_slots` deletes ids without reindexing the survivors, so
/// after deleting the middle of three slots `len()` is 2 while a survivor still sits at
/// `anchor + 30 m` — the next Add Role landed exactly on top of it, re-creating the stacking this
/// ticket exists to fix. `max(x) + 15 m` is free by construction no matter which ids were removed.
fn next_slot_xy(
    core: &MissionDocCore,
    sq: &crate::editor::panels::outliner::SquadRow,
) -> (f64, f64) {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return (APPLY_ANCHOR_X, APPLY_ANCHOR_Y);
    };
    let (ax, ay) = squad_anchor_in(&root, sq).unwrap_or((APPLY_ANCHOR_X, APPLY_ANCHOR_Y));
    let x = sq
        .slot_ids
        .iter()
        .filter_map(|id| slot_xy(&root, id).map(|(x, _)| x))
        .reduce(f64::max)
        .map_or(ax, |max_x| max_x + ORBAT_SLOT_SPACING_X);
    (x, ay)
}

/// Patch inspector fields: role/tag via `update_slot`, callsign/rank via `update_slot_identity`.
pub fn orbat_update_slot_fields(
    slot_id: String,
    role: Option<String>,
    tag: Option<String>,
    callsign: Option<String>,
    rank: Option<String>,
) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        if role.is_some() || tag.is_some() {
            core.update_slot(&slot_id, role, tag, None);
        }
        if callsign.is_some() || rank.is_some() {
            core.update_slot_identity(&slot_id, callsign, rank);
        }
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/* ───────────────────────── T-180.6 — ORBAT refile (core move only) ───────────────────────── */

thread_local! {
    /// Armed slot id between ORBAT slot `pointerdown` and squad-row `pointerup` (pointer-drag, not HTML5 DnD).
    static PENDING_REFILE: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Arm a slot for refile into another squad (OrbatManager pointer-drag).
pub fn begin_refile(slot_id: String) {
    PENDING_REFILE.with(|p| *p.borrow_mut() = Some(slot_id));
}

/// Clear an armed refile without mutating the doc (drop outside a squad row).
pub fn cancel_refile() {
    PENDING_REFILE.with(|p| *p.borrow_mut() = None);
}

/// Complete an armed refile onto `dest_squad_id` via [`refile_slot`].
pub fn complete_refile_onto_squad(dest_squad_id: String) -> bool {
    let Some(slot_id) = PENDING_REFILE.with(|p| p.borrow_mut().take()) else {
        return false;
    };
    refile_slot(slot_id, dest_squad_id)
}

/// Move `slot_id` into `dest_squad_id` through core [`MissionDocCore::move_slot_to_squad`] only
/// (F-L2 — no FE `slotIds` splice), then the shared dirty tail (orbat_nodes + squad links).
pub fn refile_slot(slot_id: String, dest_squad_id: String) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            core.move_slot_to_squad(&slot_id, &dest_squad_id);
        }
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Commit an armed place at a **world** position, then select it and run the shared post-change
/// tail. Returns `false` when nothing was armed.
///
/// A **Factions** leaf files a slot into the side's current squad
/// ([`place_character_under_side`]). A **Vehicles** leaf (T-215) writes a `vehiclesById` row at the
/// same world point — see [`place_vehicle_in_core`] for why that one does not join a squad.
///
/// `z = 0.0` / `rotation = 0.0` match the T-159.19 drag commit's DEM-not-ready case (React's
/// `terrainZ` on the flat map).
/// Commit an armed place at a **world** position with no modifiers (crewed vehicle default, one-shot
/// arm). The plain-click entry point; the Ctrl/Alt gestures use [`place_at_keep`] / [`place_at_alt`].
/// Retained as the canonical no-modifier API (and the rustdoc anchor the `place_*_in_core` helpers
/// link) even though the live pointerup routes through [`place_at_alt`] to carry the Alt override.
#[allow(dead_code)]
pub fn place_at(x: f64, y: f64) -> bool {
    place_at_impl(x, y, false, false)
}

/// T-647 PLACE-CREW-001 — place with the Alt "empty vehicle" override. `alt_empty` forces a Vehicle
/// arm to stamp `crewed: false` regardless of the DockRight crew toggle (the per-gesture override of
/// the default); it is inert for a character/object arm. One-shot: the arm is consumed, exactly like
/// [`place_at`].
pub fn place_at_alt(x: f64, y: f64, alt_empty: bool) -> bool {
    place_at_impl(x, y, alt_empty, false)
}

/// T-647 PLACE-004 — Ctrl multi-place: place, but KEEP the pending armed so the next canvas click
/// drops another of the same entity. `alt_empty` carries the [`place_at_alt`] override through each
/// stamp of a multi-place run, so Ctrl+Alt drops a string of empty vehicles.
///
/// The arm is snapshotted before the (shared) consume and re-armed on a SUCCESSFUL place; a failed
/// place (nothing armed, or a core reject) does not re-arm — a multi-place that can't commit must
/// not spin. Re-arming through the raw `pending` cell (not [`arm`]) keeps the ORIGINAL arm verbatim:
/// the Objects/side-mode guard already vetted it once at [`begin_place*`] time, and the operator has
/// not changed chips mid-gesture.
pub fn place_at_keep(x: f64, y: f64, alt_empty: bool) -> bool {
    // Snapshot the armed value before the consume (zone draws never reach here — they are
    // multi-click already and `place_at_impl` returns via `advance_zone_draw`).
    let snapshot = OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.pending.borrow().clone())
    });
    let placed = place_at_impl(x, y, alt_empty, true);
    if placed {
        if let Some(p) = snapshot {
            OPS_CTX.with(|c| {
                if let Some(ctx) = c.borrow().as_ref() {
                    *ctx.pending.borrow_mut() = Some(p);
                }
            });
        }
    }
    placed
}

/// The shared place body. `alt_empty` is the PLACE-CREW-001 override (T-647); `keep` is honoured by
/// the [`place_at_keep`] wrapper (which re-arms after this returns) — this fn always `take`s the arm
/// so its borrow discipline is unchanged, and the wrapper restores it.
fn place_at_impl(x: f64, y: f64, alt_empty: bool, keep: bool) -> bool {
    // `keep` is acted on by the caller ([`place_at_keep`]); named here so the state machine reads
    // straight ("place, keeping the arm") and a future in-body use has its parameter.
    let _ = keep;
    // T-582 — a zone draw is MULTI-CLICK (circle: centre then rim; polygon: one vertex per click),
    // so it must not reach the `take` below, which is written for the one-shot palette arms.
    if zone_draw_armed() {
        return advance_zone_draw(x, y);
    }
    // T-809 wave-203 — one recently-placed entry for a composition STAMP, keyed on the composition id
    // (a 3-member stamp is ONE authoring action, so ONE entry — not one per stamped entity), with the
    // composition's title as the label. Captured INSIDE the doc borrow (where the title is readable)
    // and recorded AFTER it closes: the recorder writes a `!Send` Leptos signal in `eden_dock_right`
    // and must not run under this doc borrow / write-txn scope (the read/write-txn discipline the
    // `select` block below states). Stays `None` for every non-composition arm and when the dock is
    // unmounted the invoke below no-ops (that is not a dropped ack — the stamp already committed).
    let mut recent_stamp: Option<(String, String)> = None;
    let placed = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        // T-180.5 / T-254 — Objects mode only commits Object arms (handled below); do not
        // blanket-reject here (that was the stub that blocked entities[] placement).
        let Some(pending) = ctx.pending.borrow_mut().take() else {
            return false;
        };
        // Scoped: the mutators open write txns, which must be gone before `after_local_edit`'s
        // read txn. `select` is the id to leave selected, or `None` — the selection is the SLOT
        // selection (`select_tool::pick` runs over the slot SoA, and SEL counts it), so putting a
        // vehicle/entity id in it would show `SEL 1` with nothing highlighted anywhere.
        let select = {
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return false;
            };
            let side = ctx.active_side.get_untracked();
            let id = mint_id(ctx, core);
            match pending {
                // T-582 — unreachable: the zone branch at the top of this function returns before
                // the `take` that produced `pending`. Written as an explicit refusal rather than a
                // `_ =>` so that a fourth arm added later cannot fall silently into a slot place.
                Pending::Zone(_) => return false,
                Pending::Vehicle(payload) => {
                    // T-076 (RIGHT-CREW-001) — read the manned/unmanned toggle at place time.
                    // T-647 PLACE-CREW-001 — Alt is the per-gesture override: it forces empty
                    // (`crewed: false`) even when the dock toggle says crewed, and never the
                    // reverse (Alt cannot conjure a crew a switched-off toggle withheld).
                    let with_crew = place_with_crew() && !alt_empty;
                    if !place_vehicle_in_core(core, &side, &id, &payload.asset_id, x, y, with_crew)
                    {
                        return false;
                    }
                    None
                }
                Pending::Object(payload) => {
                    if !place_object_in_core(core, &side, &id, &payload, x, y) {
                        return false;
                    }
                    None
                }
                // T-069 (RIGHT-MODE-006) — drop a marker at the release point, under the active
                // side's faction briefing (the ONLY schema-legal marker placement; see the block
                // header at the end of this file). The slot id minted above is not used: a marker
                // is not a slot and lives in a different address space, so it mints its own id off
                // the marker list.
                Pending::Marker(icon) => {
                    let _ = id;
                    // T-826 — bind the active side chip WITHOUT minting `faction-{SIDE}`. The store
                    // parks the marker until the first slot/squad path calls `ensure_side_faction`
                    // (lazy mint via `add_faction` promote). Minting here was F-11: phantom V1.
                    let faction_id = side_faction_id(&side);
                    let marker_id = mint_marker_id(&marker_rows_of(core));
                    // `y` here is the canvas's second WORLD axis, which is mission `z` — the same
                    // rename `advance_zone_draw(x, z)` makes one screen down. `$defs/marker` is
                    // `{x, z}` and carries no height at all, so there is no third component to drop.
                    let (mx, mz) = (x, y);
                    // Fresh markers carry an EMPTY label: `$defs/marker` requires the key, and the
                    // author captions it in the panel. Inventing "Marker 1" would put a placeholder
                    // on a game server's map the moment someone forgot to overwrite it.
                    core.set_faction_briefing_marker(&faction_id, &marker_id, mx, mz, &icon, "");
                    // Markers are not slots: putting a marker id in the slot selection would show
                    // `SEL 1` with nothing highlighted (the `place_at` selection rule).
                    None
                }
                Pending::Composition(comp_id) => {
                    // The single `id` minted above is unused for a composition (it mints its own
                    // per-entity ids). Resolve the row, mint one id per captured entity, and stamp
                    // them all in ONE undo step via the core mutator.
                    let _ = id;
                    let Some(entities) = composition_entities_json(core, &comp_id) else {
                        return false;
                    };
                    let count = composition_entity_count(&entities);
                    if count == 0 {
                        return false;
                    }
                    let ids = mint_ids(ctx, core, count);
                    let layer_id = ensure_layer(ctx, core);
                    let b = terrain_bounds_of(core);
                    let written =
                        core.place_composition(&entities, &ids, &side, &layer_id, x, y, b[2], b[3]);
                    if written.is_empty() {
                        return false;
                    }
                    // T-809 wave-203 — record the stamp as ONE recently-placed entry (keyed on the
                    // composition id, labelled with its title). Captured here where the title reads off
                    // `core`; the invoke runs after this borrow closes (see `recent_stamp`'s decl).
                    recent_stamp = Some((comp_id.clone(), composition_title(core, &comp_id)));
                    // Select the placed SLOTS (SEL runs over the slot SoA; vehicle/object ids would
                    // show `SEL n` with nothing highlighted — the `place_at` selection rule).
                    let slot_ids: Vec<String> = written
                        .into_iter()
                        .filter(|w| slot_attrs_exists(core, w))
                        .collect();
                    *ctx.selection.borrow_mut() = slot_ids;
                    None
                }
                Pending::Character(payload) => {
                    let layer_id = ensure_layer(ctx, core);
                    let asset_id = payload.asset_id.clone();
                    if place_character_under_side(
                        core,
                        &side,
                        &id,
                        &layer_id,
                        &payload.role,
                        None,
                        Some(payload.asset_id),
                        x,
                        y,
                        0.0,
                        0.0,
                    )
                    .is_err()
                    {
                        return false;
                    }
                    // T-068.15.2 — a fresh placement has no loadout: seed the character's
                    // default cargo (same borrow scope ⇒ same undo step as the place).
                    seed_cargo_in_core(core, &id, &asset_id, None);
                    Some(id)
                }
            }
        };
        if let Some(id) = select {
            *ctx.selection.borrow_mut() = vec![id];
        }
        true
    });
    if placed {
        // Rebinds the glyphs from the new SoA, bumps `doc_ver`, schedules the persist, and refreshes
        // the HUD + docks — the same tail the drag commit and undo/redo run.
        mission_history::after_local_edit();
        // T-809 wave-203 — now the doc borrow is closed, head the recently-placed list with the stamp
        // (composition arm only; `None` otherwise). No-ops when the dock is unmounted — not a dropped
        // ack: the stamp above already committed.
        if let Some((asset_id, label)) = recent_stamp {
            crate::editor::panels::dock_right::record_placed(asset_id, label);
        }
    }
    placed
}

/// T-647 CONN-GROUP-001 (map half) — move the dragged character `slot_id` into the squad of the
/// character `target_id` it was Ctrl-dropped onto. Reads the target's squad off the materialized SoA
/// ([`slot_attrs`]) and refiles through the existing T-180.6 seam ([`refile_slot`], which runs the
/// shared dirty tail), so a map regroup and an ORBAT-dock refile are the SAME core move
/// (`move_slot_to_squad`) — one undo step, one squad-membership write.
///
/// Returns `false` (no-op) when the target has no squad (an unfiled slot, or the target vanished),
/// or already shares the dragged slot's squad — `move_slot_to_squad` is itself a no-op on
/// same-squad, but declining here keeps the caller from firing the dirty tail for nothing.
pub fn regroup_slot_onto(slot_id: &str, target_id: &str) -> bool {
    if slot_id == target_id {
        return false;
    }
    let dest_squad = read_attrs(target_id).map(|a| a.squad).unwrap_or_default();
    let src_squad = read_attrs(slot_id).map(|a| a.squad).unwrap_or_default();
    if dest_squad.is_empty() || dest_squad == src_squad {
        return false;
    }
    refile_slot(slot_id.to_string(), dest_squad)
}

/* ═══════════════════ T-582 — the zone draw tool (doc-mutating half) ═══════════════════ */

// T-211 shipped `zones` + eleven mutators on `MissionDocCore`; NOTHING called them. This block is
// the caller. The pure half — the schema-driven vocabularies, the 0.1 m grid, and the two shape
// predicates — lives in `eden_chrome` because it compiles on the NATIVE target, where
// `cargo test -p website-frontend` can run it; `MissionDocCore` is a wasm32-only dependency of this
// crate (see Cargo.toml), so anything that touches it can only live here, where no test runs.
//
// The division is deliberate: every decision this file makes is delegated to a predicate over there
// that IS tested (`circle_from_clicks`, `polygon_is_committable`, `zone_types`), so the untestable
// half stays as close to pure plumbing as it can be.

use crate::editor::eden_chrome::{
    circle_from_clicks, polygon_flat, polygon_is_committable, zone_types, ZoneShape,
};

// T-079 — `DrawTarget` is imported straight from its home module (`eden_chrome` re-exports the other
// zone-tool pure items, but this one is added here in a slice that does not own `eden_chrome`).
use crate::editor::panels::zones_panel::DrawTarget;

/// One authored zone, read back for the dock list and the Attributes panel.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneRow {
    pub id: String,
    /// Schema `zone.type`.
    pub kind: String,
    pub label: Option<String>,
    pub faction: Option<String>,
    /// `rules` VERBATIM, as the opaque object the doc stores. Never parsed into named fields here —
    /// see `set_zone_rules`' note in `doc/store.rs` for why a typed mirror would be the second
    /// vocabulary T-241 exists to prevent.
    pub rules: serde_json::Value,
    /// `Some((x, z, r))` for a circle.
    pub circle: Option<(f64, f64, f64)>,
    /// The ring for a polygon.
    pub polygon: Vec<(f64, f64)>,
}

impl ZoneRow {
    /// A one-line geometry summary for the dock row.
    #[must_use]
    pub fn shape_summary(&self) -> String {
        if let Some((x, z, r)) = self.circle {
            format!("circle r {r:.1} m @ {x:.0}, {z:.0}")
        } else if self.polygon.is_empty() {
            "no shape".to_string()
        } else {
            format!("polygon, {} vertices", self.polygon.len())
        }
    }
}

/// Is a zone draw in flight?
#[must_use]
pub fn zone_draw_armed() -> bool {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|ctx| matches!(*ctx.pending.borrow(), Some(Pending::Zone(_))))
    })
}

/// The in-flight draw, for the dock's live hint ("click the rim", "2 vertices — one more to close").
#[must_use]
pub fn zone_draft() -> Option<ZoneDraft> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Zone(d)) => Some(d.clone()),
            _ => None,
        }
    })
}

/// Arm a draw against `collection` ([`DrawTarget`] — the second-consumer parameter). For a ZONE,
/// `kind` is a `zone.type` and must come from [`zone_types`] — this refuses anything else rather than
/// letting an invented type reach the document, where it would save 201 and then 500 `/compiled`
/// forever (T-581 measured exactly that for `"capture"`). For a TRIGGER, `kind` is the activation
/// kind and must be one of [`TRIGGER_ACTIVATIONS`] — the same "refuse an invented value at the arm"
/// discipline, applied to the trigger's own vocabulary.
pub fn begin_zone_draw(kind: &str, shape: ZoneShape, collection: DrawTarget) -> bool {
    let valid = match collection {
        DrawTarget::Zone => zone_types().iter().any(|t| t == kind),
        DrawTarget::Trigger => TRIGGER_ACTIVATIONS.contains(&kind),
    };
    if !valid {
        return false;
    }
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        *ctx.pending.borrow_mut() = Some(Pending::Zone(ZoneDraft {
            kind: kind.to_string(),
            shape,
            centre: None,
            verts: Vec::new(),
            target: None,
            collection,
        }));
        true
    })
}

/// T-582 / T-079 — arm a RESHAPE of an existing row in `collection`: the next clicks replace its
/// `shape` through `set_*_circle` / `set_*_polygon` instead of minting a new row.
///
/// The gesture is identical to a fresh draw (circle: centre then rim; polygon: vertices then Close),
/// so there is one geometry path and one set of guards — a reshape cannot produce the `r → 0.0`
/// circle or the two-vertex ring any more than a create can. `kind` is read from the live document
/// rather than taken from the caller: reshaping is a geometry edit, and silently retyping a row
/// because the dock's picker had drifted would be a different, invisible edit. The existence check
/// reads the collection's OWN map (zones vs triggers) so a reshape can never target the wrong one.
pub fn begin_zone_reshape(row_id: &str, shape: ZoneShape, collection: DrawTarget) -> bool {
    let kind = match collection {
        DrawTarget::Zone => zone_rows()
            .into_iter()
            .find(|r| r.id == row_id)
            .map(|r| r.kind),
        // T-079 — a trigger reshape keeps the trigger's ACTIVATION (its analogue of `zone.type`) as
        // the draft `kind`, so a create and a reshape carry the same field.
        DrawTarget::Trigger => trigger_rows()
            .into_iter()
            .find(|r| r.id == row_id)
            .map(|r| r.activation),
    };
    let Some(kind) = kind else {
        return false;
    };
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        *ctx.pending.borrow_mut() = Some(Pending::Zone(ZoneDraft {
            kind,
            shape,
            centre: None,
            verts: Vec::new(),
            target: Some(row_id.to_string()),
            collection,
        }));
        true
    })
}

/// Abandon the in-flight draw without writing anything. The explicit counterpart to
/// [`cancel_pending`], which a zone draw deliberately survives. Returns whether a draw was actually
/// abandoned — `false` when nothing was in flight (or the ops context is not up).
///
/// T-792 — this is the ONE cancel a zone draw honours, so it is what the keyboard-Esc arm in
/// `mission_editor` calls to clear an in-progress circle/polygon (matching the panel Cancel button).
/// Like T-791's `cancel_pending`, a real clear bumps the dock tick OUTSIDE the `OPS_CTX` borrow
/// (`bump_doc_tick` re-borrows it): the Zones/Triggers panel's "click the rim"/vertex hint re-reads
/// [`zone_draft`] under `doc_tick` and vanishes once the draft is `None`. The panel Cancel button
/// bumps the tick itself too, so this bump is a harmless second re-read on that path.
pub fn cancel_zone_draw() -> bool {
    let cleared = OPS_CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            let mut p = ctx.pending.borrow_mut();
            if matches!(*p, Some(Pending::Zone(_))) {
                *p = None;
                return true;
            }
        }
        false
    });
    if cleared {
        bump_doc_tick();
    }
    cleared
}

/// Drop the last polygon vertex (the Undo-vertex control). Returns the remaining count.
pub fn zone_draw_pop_vertex() -> usize {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let mut p = ctx.pending.borrow_mut();
        if let Some(Pending::Zone(d)) = p.as_mut() {
            d.verts.pop();
            return d.verts.len();
        }
        0
    })
}

/// One canvas release while a zone draw is armed.
///
/// **Circle** — the first release sets the centre, the second is the rim and COMMITS. The radius is
/// the distance between them, and [`circle_from_clicks`] refuses a rim that quantises the radius to
/// zero, so the click-without-travel that produced `r = 0.04` cannot create a zone at all.
///
/// **Polygon** — every release appends a vertex and the draw stays armed; nothing is written until
/// [`close_zone_polygon`], which enforces `minItems: 3`. The doc layer deliberately does not guard
/// that (its own comment assigns it here), so a ring is never handed over short.
pub(super) fn advance_zone_draw(x: f64, z: f64) -> bool {
    enum Commit {
        Circle {
            kind: String,
            geom: (f64, f64, f64),
            target: Option<String>,
            collection: DrawTarget,
        },
        None,
    }
    let commit = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Commit::None;
        };
        let mut p = ctx.pending.borrow_mut();
        let Some(Pending::Zone(d)) = p.as_mut() else {
            return Commit::None;
        };
        match d.shape {
            ZoneShape::Polygon => {
                d.verts.push((x, z));
                Commit::None
            }
            ZoneShape::Circle => match d.centre {
                None => {
                    d.centre = Some((x, z));
                    Commit::None
                }
                Some((cx, cz)) => match circle_from_clicks(cx, cz, x, z) {
                    // A rim that would quantise the radius to zero is NOT a commit and NOT a
                    // cancel: the centre stays put so the author can simply drag further out.
                    None => Commit::None,
                    Some(geom) => {
                        let (kind, target, collection) =
                            (d.kind.clone(), d.target.clone(), d.collection);
                        *p = None;
                        Commit::Circle {
                            kind,
                            geom,
                            target,
                            collection,
                        }
                    }
                },
            },
        }
    });
    match commit {
        // T-079 — the ONLY per-collection branch in the whole draw flow: which mutator pair the
        // committed geometry calls. Everything above (centre/rim accumulation, the `r → 0.0` refusal)
        // is shared verbatim between zones and triggers.
        Commit::Circle {
            kind,
            geom: (cx, cz, r),
            target,
            collection,
        } => match (collection, target) {
            // Reshape: replaces the whole `shape` object, so name / owner / activation / rules
            // survive and the `oneOf` can never end up with both branches present.
            (DrawTarget::Zone, Some(id)) => edit_zone(|core| core.set_zone_circle(&id, cx, cz, r)),
            (DrawTarget::Zone, None) => write_row(DrawTarget::Zone, |core, id| {
                core.add_circle_zone(id, &kind, cx, cz, r);
            }),
            (DrawTarget::Trigger, Some(id)) => {
                edit_zone(|core| core.set_trigger_circle(&id, cx, cz, r))
            }
            (DrawTarget::Trigger, None) => write_row(DrawTarget::Trigger, |core, id| {
                core.add_circle_trigger(id, &kind, cx, cz, r);
            }),
        },
        // A vertex / centre landed but no document write happened yet. Report progress so the dock
        // re-reads, without running the persist tail for a doc that did not change.
        Commit::None => {
            bump_doc_tick();
            zone_draw_armed()
        }
    }
}

/// Close the in-flight ring. Refuses below three vertices — `$defs/polygon` is `minItems: 3` and a
/// two-vertex ring is a document the schema rejects.
pub fn close_zone_polygon() -> bool {
    let taken = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let mut p = ctx.pending.borrow_mut();
        let Some(Pending::Zone(d)) = p.as_ref() else {
            return None;
        };
        if d.shape != ZoneShape::Polygon || !polygon_is_committable(&d.verts) {
            return None;
        }
        let out = (
            d.kind.clone(),
            d.verts.clone(),
            d.target.clone(),
            d.collection,
        );
        *p = None;
        Some(out)
    });
    let Some((kind, verts, target, collection)) = taken else {
        return false;
    };
    let flat = polygon_flat(&verts);
    match (collection, target) {
        // Reshape — see [`begin_zone_reshape`]: whole-`shape` replacement, so a circle becomes a
        // polygon without leaving both `oneOf` branches on the row.
        (DrawTarget::Zone, Some(id)) => edit_zone(|core| core.set_zone_polygon(&id, &flat)),
        (DrawTarget::Zone, None) => write_row(DrawTarget::Zone, |core, id| {
            core.add_polygon_zone(id, &kind, &flat)
        }),
        (DrawTarget::Trigger, Some(id)) => edit_zone(|core| core.set_trigger_polygon(&id, &flat)),
        (DrawTarget::Trigger, None) => write_row(DrawTarget::Trigger, |core, id| {
            core.add_polygon_trigger(id, &kind, &flat)
        }),
    }
}

/// Mint an unused id in `collection`'s OWN namespace (`z{n}` for zones, `t{n}` for triggers), proven
/// unique against that collection's live map rather than assumed — undo frees ids and an IDB restore
/// can bring back a document that already used one. Each collection is a separate namespace (the slot
/// SoA does not contain either), so `mint_id`'s slot proof does not apply here.
fn mint_row_id(core: &MissionDocCore, collection: DrawTarget) -> String {
    let (json, prefix) = match collection {
        DrawTarget::Zone => (core.zones_json(), "z"),
        DrawTarget::Trigger => (core.triggers_json(), "t"),
    };
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&json)
            .ok()
            .and_then(|v| {
                v.as_object()
                    .map(|m| m.keys().map(ToString::to_string).collect())
            })
            .unwrap_or_default();
    (1u32..)
        .map(|n| format!("{prefix}{n}"))
        .find(|id| !existing.contains(id))
        .unwrap_or_else(|| format!("{prefix}1"))
}

/// Run a row-creating mutator (`add_*_zone` / `add_*_trigger`) under a minted id in `collection`,
/// then the shared dirty tail. The write txn is scoped so it is gone before `after_local_edit` opens
/// its read txn (the `mission_history` rule).
fn write_row(collection: DrawTarget, f: impl FnOnce(&MissionDocCore, &str)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        let id = mint_row_id(core, collection);
        f(core, &id);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Every authored zone, in `zones_json` map order, for the dock list.
#[must_use]
pub fn zone_rows() -> Vec<ZoneRow> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let map: serde_json::Value = serde_json::from_str(&core.zones_json()).ok()?;
            let obj = map.as_object()?;
            let mut rows: Vec<ZoneRow> = obj
                .iter()
                .map(|(id, z)| {
                    let shape = z.get("shape");
                    let circle = shape.and_then(|s| s.get("circle")).and_then(|c| {
                        Some((
                            c.get("x")?.as_f64()?,
                            c.get("z")?.as_f64()?,
                            c.get("r")?.as_f64()?,
                        ))
                    });
                    let polygon = shape
                        .and_then(|s| s.get("polygon"))
                        .and_then(serde_json::Value::as_array)
                        .map(|ring| {
                            ring.iter()
                                .filter_map(|p| {
                                    let a = p.as_array()?;
                                    Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    ZoneRow {
                        id: id.clone(),
                        kind: z
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        label: z
                            .get("label")
                            .and_then(|l| l.as_str())
                            .map(ToString::to_string),
                        faction: z
                            .get("faction")
                            .and_then(|f| f.as_str())
                            .map(ToString::to_string),
                        rules: z.get("rules").cloned().unwrap_or(serde_json::Value::Null),
                        circle,
                        polygon,
                    }
                })
                .collect();
            // Map iteration order is not stable across a reload; sort so the dock list does not
            // reshuffle under the author between sessions.
            rows.sort_by(|a, b| a.id.cmp(&b.id));
            Some(rows)
        })
        .unwrap_or_default()
}

/// Run a mutator against an existing zone, then the shared dirty tail.
fn edit_zone(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Attributes — schema `zone.type`. Refuses a value outside the schema enum, for the same reason
/// [`begin_zone_draw`] does.
pub fn set_zone_kind(id: &str, kind: &str) -> bool {
    if !zone_types().iter().any(|t| t == kind) {
        return false;
    }
    edit_zone(|core| core.set_zone_type(id, kind))
}

/// Attributes — schema `zone.label`. An EMPTY string and `None` are different authored states the
/// schema allows on purpose: `Some("")` writes an empty label (which the mod reads as "use the
/// PrettyZoneTitle fallback" and is a committed golden), `None` removes the key. The panel's Clear
/// control sends `None`; typing and clearing the box sends `Some("")`.
pub fn set_zone_label(id: &str, label: Option<String>) -> bool {
    edit_zone(|core| core.set_zone_label(id, label.as_deref()))
}

/// Attributes — schema `zone.faction` (a `factionKey` slug). `None` makes the zone faction-neutral.
pub fn set_zone_faction(id: &str, faction: Option<String>) -> bool {
    edit_zone(|core| core.set_zone_faction(id, faction.as_deref()))
}

/// Attributes — set or clear ONE `rules` key, read-modify-write over the OPAQUE object.
///
/// The key is a `$defs/zoneRules` property name supplied by the schema-driven panel; this function
/// never names one. `value: None` removes the key, and once the last key is gone the whole object
/// goes with it — `set_zone_rules` treats `{}` as a removal on purpose, because "the author cleared
/// every rule" and "the author never opened the panel" must stay the same document.
pub fn set_zone_rule(id: &str, key: &str, value: Option<serde_json::Value>) -> bool {
    let next = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let map: serde_json::Value = serde_json::from_str(&core.zones_json()).ok()?;
        let mut rules = map
            .get(id)?
            .get("rules")
            .and_then(|r| r.as_object().cloned())
            .unwrap_or_default();
        match value {
            Some(v) => {
                rules.insert(key.to_string(), v);
            }
            None => {
                rules.remove(key);
            }
        }
        Some(serde_json::Value::Object(rules).to_string())
    });
    let Some(next) = next else {
        return false;
    };
    edit_zone(|core| core.set_zone_rules(id, Some(&next)))
}

/// Attributes — delete the zone.
pub fn delete_zone(id: &str) -> bool {
    edit_zone(|core| core.remove_zone(id))
}

/// How many zones the document declares — backs "does this mission define a play area?".
#[must_use]
pub fn zone_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.doc.borrow().as_ref().map(MissionDocCore::zone_count))
            .unwrap_or(0)
    })
}

/* ═══════════════════ T-079 — triggers, the editor half (RIGHT-MODE-003 + CONN-TRG-OWNER-001) ═══════════════════ */

// The trigger AREA rides the shipped zone draw tool as a SECOND CONSUMER: the draw flow above is
// parameterized by [`DrawTarget`], so `begin_zone_draw(..., DrawTarget::Trigger)` and the reshape
// pair author `triggersById` through the SAME `advance_zone_draw` / `close_zone_polygon` state
// machine — there is no forked trigger draw. This block adds only the trigger-specific reads/writes
// (name / owner / activation / rules / delete), the owner picker's source, and the owner-link line
// geometry. Doc mutators are T-079's `MissionDocCore` block; the pure line projection is
// native-tested (the `ruler_tool::project_legs` idiom).

/// The three activation kinds the ticket names — a TYPED PLACEHOLDER: STORED, not evaluated (the
/// activation/effects runtime is T-676). This is the trigger's small closed vocabulary, the analogue
/// of `zone_types()` for the draw arm. Kept here (not read from the schema like `zone_types`) because
/// the schema does not declare `triggers` yet — T-706 (wave 120) will; when it does and declares an
/// `activation` enum, this should be replaced by a schema read exactly as `zone_types` is.
pub const TRIGGER_ACTIVATIONS: &[&str] = &["presence", "radio", "timer"];

/// One authored trigger, read back for the palette list and the Attributes panel. Mirrors
/// [`ZoneRow`], plus the trigger-only `name` / `owner_id` / `activation`.
#[derive(Clone, Debug, PartialEq)]
pub struct TriggerRow {
    pub id: String,
    pub name: Option<String>,
    /// CONN-TRG-OWNER-001 — the linked placed entity, or `None` (unowned). May be DANGLING: the
    /// entity it names can have been deleted; readers resolve it to nothing, they do not clear it.
    pub owner_id: Option<String>,
    /// One of [`TRIGGER_ACTIVATIONS`] (stored, not evaluated).
    pub activation: String,
    /// `rules` VERBATIM — the opaque `$defs/zoneRules`-shaped object, never parsed into named fields
    /// (the `ZoneRow::rules` reason: a typed mirror would be the second vocabulary T-241 prevents).
    pub rules: serde_json::Value,
    /// `Some((x, z, r))` for a circle.
    pub circle: Option<(f64, f64, f64)>,
    /// The ring for a polygon.
    pub polygon: Vec<(f64, f64)>,
}

impl TriggerRow {
    /// A one-line geometry summary for the palette row (the [`ZoneRow::shape_summary`] twin).
    #[must_use]
    pub fn shape_summary(&self) -> String {
        if let Some((x, z, r)) = self.circle {
            format!("circle r {r:.1} m @ {x:.0}, {z:.0}")
        } else if self.polygon.is_empty() {
            "no shape".to_string()
        } else {
            format!("polygon, {} vertices", self.polygon.len())
        }
    }

    /// The trigger's geometric CENTRE in world metres — a circle's centre, or a polygon's vertex
    /// mean. `None` for a shapeless row. This is the trigger end of the owner-link line.
    #[must_use]
    pub fn centre(&self) -> Option<(f64, f64)> {
        if let Some((x, z, _)) = self.circle {
            return Some((x, z));
        }
        if self.polygon.is_empty() {
            return None;
        }
        let n = self.polygon.len() as f64;
        let (sx, sz) = self
            .polygon
            .iter()
            .fold((0.0, 0.0), |(ax, az), (x, z)| (ax + x, az + z));
        Some((sx / n, sz / n))
    }
}

/// Every authored trigger, sorted by id, for the palette list. Off `triggers_json`, the
/// [`zone_rows`] twin.
#[must_use]
pub fn trigger_rows() -> Vec<TriggerRow> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let map: serde_json::Value = serde_json::from_str(&core.triggers_json()).ok()?;
            let obj = map.as_object()?;
            let mut rows: Vec<TriggerRow> = obj
                .iter()
                .map(|(id, t)| {
                    let shape = t.get("shape");
                    let circle = shape.and_then(|s| s.get("circle")).and_then(|c| {
                        Some((
                            c.get("x")?.as_f64()?,
                            c.get("z")?.as_f64()?,
                            c.get("r")?.as_f64()?,
                        ))
                    });
                    let polygon = shape
                        .and_then(|s| s.get("polygon"))
                        .and_then(serde_json::Value::as_array)
                        .map(|ring| {
                            ring.iter()
                                .filter_map(|p| {
                                    let a = p.as_array()?;
                                    Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    TriggerRow {
                        id: id.clone(),
                        name: t
                            .get("name")
                            .and_then(|n| n.as_str())
                            .map(ToString::to_string),
                        owner_id: t
                            .get("ownerId")
                            .and_then(|o| o.as_str())
                            .map(ToString::to_string),
                        activation: t
                            .get("activation")
                            .and_then(|a| a.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        rules: t.get("rules").cloned().unwrap_or(serde_json::Value::Null),
                        circle,
                        polygon,
                    }
                })
                .collect();
            rows.sort_by(|a, b| a.id.cmp(&b.id));
            Some(rows)
        })
        .unwrap_or_default()
}

/// Run a mutator against an existing trigger, then the shared dirty tail. The [`edit_zone`] twin.
fn edit_trigger(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// Attributes — the trigger `name`. `None` REMOVES the key; the panel sends `None` on an emptied box.
pub fn set_trigger_name(id: &str, name: Option<String>) -> bool {
    edit_trigger(|core| core.set_trigger_name(id, name.as_deref()))
}

/// Attributes — the stored-not-evaluated `activation` kind. Refuses a value outside
/// [`TRIGGER_ACTIVATIONS`], the same "no invented value reaches the doc" discipline `set_zone_kind`
/// applies to `zone.type`.
pub fn set_trigger_activation(id: &str, activation: &str) -> bool {
    if !TRIGGER_ACTIVATIONS.contains(&activation) {
        return false;
    }
    edit_trigger(|core| core.set_trigger_activation(id, activation))
}

/// CONN-TRG-OWNER-001 (the picker's write) — assign / clear the owner edge. `Some(id)` is a placed
/// entity id from [`placed_owner_options`]; `None` clears the link. No referential check — a later
/// deletion of the owner is TOLERATED as a dangling edge (see [`MissionDocCore::set_trigger_owner`]).
pub fn set_trigger_owner(id: &str, owner_id: Option<String>) -> bool {
    edit_trigger(|core| core.set_trigger_owner(id, owner_id.as_deref()))
}

/// Attributes — set or clear ONE `rules` key, read-modify-write over the OPAQUE object. Reuses the
/// exact `set_zone_rule` shape: the key is a `$defs/zoneRules` property supplied by the schema-driven
/// panel; this never names one. `value: None` removes the key, and clearing the last key drops the
/// whole object (the "cleared all" == "never authored" identity).
pub fn set_trigger_rule(id: &str, key: &str, value: Option<serde_json::Value>) -> bool {
    let next = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let map: serde_json::Value = serde_json::from_str(&core.triggers_json()).ok()?;
        let mut rules = map
            .get(id)?
            .get("rules")
            .and_then(|r| r.as_object().cloned())
            .unwrap_or_default();
        match value {
            Some(v) => {
                rules.insert(key.to_string(), v);
            }
            None => {
                rules.remove(key);
            }
        }
        Some(serde_json::Value::Object(rules).to_string())
    });
    let Some(next) = next else {
        return false;
    };
    edit_trigger(|core| core.set_trigger_rules(id, Some(&next)))
}

/// Attributes — delete the trigger.
pub fn delete_trigger(id: &str) -> bool {
    edit_trigger(|core| core.remove_trigger(id))
}

/// How many triggers the document declares — backs the palette header count.
#[must_use]
pub fn trigger_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| ctx.doc.borrow().as_ref().map(MissionDocCore::trigger_count))
            .unwrap_or(0)
    })
}

/// One entry the Owner picker offers: a placed entity's id and a human label. CONN-TRG-OWNER-001 —
/// "listing placed entities (slots/vehicles by label)".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerOption {
    pub id: String,
    pub label: String,
}

/// The Owner picker's source: every PLACED slot and every PLACED vehicle, by label. Slots read off
/// the materialized SoA (role + id); vehicles off [`vehicle_rows`] filtered to those with a map
/// position (an ORBAT-only vehicle with no `xy` is not on the map, so it cannot own a placed
/// trigger). Sorted by label then id for a stable picker order.
#[must_use]
pub fn placed_owner_options() -> Vec<OwnerOption> {
    let mut out: Vec<OwnerOption> = Vec::new();
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };
        let soa = core.materialize();
        for (i, id) in soa.ids.iter().enumerate() {
            let role = {
                let idx = soa.role_idx.get(i).copied().unwrap_or(NONE_IDX);
                if idx == NONE_IDX {
                    String::new()
                } else {
                    soa.roles.get(idx as usize).cloned().unwrap_or_default()
                }
            };
            let label = if role.is_empty() {
                format!("Slot {id}")
            } else {
                format!("{role} ({id})")
            };
            out.push(OwnerOption {
                id: id.clone(),
                label,
            });
        }
    });
    // Placed vehicles (those with a map position). `vehicle_rows` opens its own borrow, so it is
    // called OUTSIDE the borrow above.
    for v in vehicle_rows() {
        if v.xy.is_none() {
            continue;
        }
        let short = v
            .resource_name
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(&v.resource_name)
            .trim_end_matches(".et");
        let label = if short.is_empty() {
            format!("Vehicle {}", v.id)
        } else {
            format!("{short} ({})", v.id)
        };
        out.push(OwnerOption { id: v.id, label });
    }
    out.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.id.cmp(&b.id)));
    out
}

/// Resolve a placed entity id to its world position (slot OR vehicle), or `None` if no such placed
/// entity exists. This is what makes a DANGLING owner render nothing: an `ownerId` pointing at a
/// deleted entity resolves to `None` here, so [`owner_line_world`] yields no line — no panic, no
/// stale draw. Slots come off the SoA; vehicles off `vehicle_rows` (placed ones only).
#[must_use]
fn placed_entity_pos(id: &str) -> Option<(f64, f64)> {
    let slot = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let soa = core.materialize();
        let row = soa.ids.iter().position(|s| s == id)?;
        Some((f64::from(soa.xs[row]), f64::from(soa.ys[row])))
    });
    if slot.is_some() {
        return slot;
    }
    vehicle_rows()
        .into_iter()
        .find(|v| v.id == id)
        .and_then(|v| v.xy)
}

/// CONN-TRG-OWNER-001 (the line's data) — the world-metre endpoints of the owner-link line for the
/// currently selected trigger: `(trigger centre, owner position)`. `None` — so the overlay draws
/// nothing — when there is no selected trigger, the selected trigger has no shape, it has no owner,
/// **or its owner is dangling** (the entity was deleted). The dangling case is the ticket's
/// tolerance requirement, and it falls out of [`placed_entity_pos`] returning `None`.
///
/// `selected_trigger` is the trigger id the palette currently has selected (session UI state the
/// panel owns), passed in so this stays a pure resolve over the doc + one id.
#[must_use]
pub fn owner_line_world(selected_trigger: Option<&str>) -> Option<((f64, f64), (f64, f64))> {
    let id = selected_trigger?;
    let row = trigger_rows().into_iter().find(|r| r.id == id)?;
    let owner_id = row.owner_id.as_deref()?;
    let centre = row.centre()?;
    // Dangling owner → None → no line. NOT an error, NOT a clear — the edge is kept in the doc.
    let owner = placed_entity_pos(owner_id)?;
    Some((centre, owner))
}

/* ═══════════════ T-069 — map markers: the four schema-carried fields ═══════════════ */
//
// ## Where a marker goes, and why it is NOT `markersById`
//
// `grep -c marker editor_ops.rs` was **0** before this block: T-345 shipped
// `set_faction_briefing_marker` / `remove_faction_briefing_marker` on `MissionDocCore` and nothing
// in the product ever called them. This block is the caller — the same "T-211 shipped eleven zone
// mutators and NOTHING called them" shape the zone tool above has.
//
// T-069's registry summary says free placement needs generic add/move/remove on the `markersById`
// ROOT map. **That premise is dead**, and it is dead in a checkable way rather than as a matter of
// taste. `mission.schema.json` declares markers in exactly ONE place — `$defs/briefing.markers[]`,
// an array of `$defs/marker` — and declares no top-level `markers` property at all. On the compile
// side `flatten_to_mod_document` deserialises `EditorPayload { editor: EditorGraph { factions,
// squads, slots } }`, which declares no root key whatsoever. So the root map is a closed
// hydrate→emit loop: a marker authored there survives a save and reaches no mod subsystem. The
// store carries that argument on `set_faction_briefing_marker`'s §authority note, and
// `a_marker_in_the_root_map_never_reaches_the_compiled_document` (T-069, `store.rs`) now pins it as
// a test instead of prose. **Author on the briefing, not the root.**
//
// The practical consequence for this surface: a marker is SIDE-SCOPED. It is placed under the
// active Eden side chip through [`side_faction_id`] (T-826 — store side WITHOUT minting; lazy
// mint at first slot/squad via [`ensure_side_faction`]) — because `bridgehead-at-levie` gives both
// sides different orders at the same coordinates, and the mod looks a briefing up by the joining
// player's faction.
//
// ## Scope: the four schema-carried fields, and not one more
//
// `$defs/marker` = `{x, z, icon, label}`. This block authors exactly those, mapping onto
// ATTR-FIELD-MRK-POSITION (`x`/`z`), -MRK-TYPE (`icon`) and -MRK-TEXT (`label`). The schema ALSO
// declares `size` / `rotationDeg` / `shape` / `area` — all stamped "T-673, lands after T-069" in
// their own descriptions — and this block deliberately writes none of them: they are marker STYLE
// and Eden's second Area-marker model, which is a different ticket. Nothing here widens the schema.
//
// ## The icon vocabulary is READ, never typed
//
// `$defs/marker.icon` is a CLOSED enum of 64 aliases (the `TBD_MarkerIcons.EnsureAliases` register
// keys). Before that enum existed a typo validated clean and then degraded at runtime to the
// fallback DOT glyph. So the picker offers the schema's list and nothing else, and
// [`crate::editor::panels::dock_right::marker_icon_is_authorable`] — which reads the embedded schema, not a
// hand-typed copy — is the gate every write here passes through. An unknown alias is REFUSED at
// this boundary rather than stored: the store's mutator takes an `&str` and asks no questions (it
// must stay that way — its own tests author aliases this enum does not contain), so the product
// surface is where the vocabulary is enforced.

/// One authored marker, as the dock lists it. The `(faction_id, id)` pair is the address both store
/// mutators take, carried on every row so a listed marker can be moved, re-captioned, re-iconed or
/// deleted without a second lookup.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkerRow {
    /// `factionsById` key — `faction-BLUFOR` / `-OPFOR` / `-INDFOR`.
    pub faction_id: String,
    /// Doc-internal id. Addressing only; it never reaches the wire (`$defs/marker` is
    /// `additionalProperties: false`, and the serde boundary drops the key for free).
    pub id: String,
    /// ATTR-FIELD-MRK-POSITION — world metres. `$defs/marker` is `{x, z}`: a marker is a MAP glyph,
    /// so it carries no height, unlike a slot's `{x, y, z}` position.
    pub x: f64,
    pub z: f64,
    /// ATTR-FIELD-MRK-TYPE — one of the 64 closed `$defs/marker.icon` aliases.
    pub icon: String,
    /// ATTR-FIELD-MRK-TEXT — the caption, stored VERBATIM (the mod caps it at render time; capping
    /// here would destroy the authored value in the one place the author could still fix it).
    pub label: String,
}

impl MarkerRow {
    /// The side chip this marker belongs to (`BLUFOR` / `OPFOR` / `INDFOR`), derived from the
    /// `faction-{SIDE}` id [`side_faction_id`] names (minted later by [`ensure_side_faction`] on
    /// first slot/squad). Falls back to the whole id for a faction that came from a library import
    /// under some other naming.
    #[must_use]
    pub fn side(&self) -> &str {
        self.faction_id
            .strip_prefix("faction-")
            .unwrap_or(&self.faction_id)
    }

    /// The palette row's right-hand readout — ATTR-FIELD-MRK-POSITION at a glance.
    #[must_use]
    pub fn position_summary(&self) -> String {
        format!("{:.0}, {:.0}", self.x, self.z)
    }
}

/// Every authored marker on every faction, in the document's own order (faction groups sorted,
/// array order preserved inside each). Reads `briefing_marker_rows_json` — the typed reader T-345
/// never shipped, which is why its two mutators had no product caller.
#[must_use]
pub fn marker_rows() -> Vec<MarkerRow> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            Some(marker_rows_of(d.as_ref()?))
        })
        .unwrap_or_default()
}

/// The parse, taking the core directly. Split out because [`place_at_impl`] already holds the doc
/// borrow when it needs the list (to mint an unused id), and re-entering through [`marker_rows`]
/// there would re-open a borrow the place path is in the middle of.
fn marker_rows_of(core: &MissionDocCore) -> Vec<MarkerRow> {
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(&core.briefing_marker_rows_json())
    else {
        return Vec::new();
    };
    let Some(arr) = rows.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|r| {
            Some(MarkerRow {
                faction_id: r.get("factionId")?.as_str()?.to_string(),
                id: r.get("id")?.as_str()?.to_string(),
                x: r.get("x")?.as_f64()?,
                z: r.get("z")?.as_f64()?,
                icon: r
                    .get("icon")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                label: r
                    .get("label")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            })
        })
        .collect()
}

/// How many markers the document carries (the palette header readout).
#[must_use]
pub fn marker_count() -> usize {
    marker_rows().len()
}

/// RIGHT-MODE-006 — an icon press ARMS a marker place. Consumed by [`place_at`] on the next canvas
/// release, dropped by [`cancel_pending`] on a release over chrome: the one-shot palette-arm
/// lifecycle, so the map's `has_pending` ghost and the Ctrl multi-place both work with no
/// marker-specific branch.
///
/// Refuses an alias outside the closed `$defs/marker.icon` enum, so a bad vocabulary cannot even be
/// armed, let alone stored.
pub fn begin_place_marker(icon: String) {
    if !crate::editor::panels::dock_right::marker_icon_is_authorable(&icon) {
        return;
    }
    arm(Pending::Marker(icon));
}

/// The armed marker icon, or `None`. Backs the panel's "click the map to drop it" hint.
#[must_use]
pub fn armed_marker_icon() -> Option<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Marker(icon)) => Some(icon.clone()),
            _ => None,
        }
    })
}

/// Mint a marker id unused anywhere in the document.
///
/// Unique across ALL factions, not just the one being written: the dock lists every side in one
/// list and addresses a row by its id alone, so two sides sharing `mk-1` would make the list
/// ambiguous even though the store (keyed by the pair) would be perfectly happy.
fn mint_marker_id(rows: &[MarkerRow]) -> String {
    let taken: std::collections::HashSet<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let mut n: u32 = 1;
    loop {
        let id = format!("mk-{n}");
        if !taken.contains(id.as_str()) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

/// ATTR-FIELD-MRK-TYPE — re-icon an existing marker, keeping its position and caption.
///
/// Goes through the same upsert the place does, which is what makes it an in-place replace rather
/// than a delete-and-append: array order is the order the mod renders in.
#[must_use]
pub fn set_marker_icon(faction_id: &str, marker_id: &str, icon: &str) -> bool {
    if !crate::editor::panels::dock_right::marker_icon_is_authorable(icon) {
        return false;
    }
    upsert_marker_field(faction_id, marker_id, |row| row.icon = icon.to_string())
}

/// ATTR-FIELD-MRK-TEXT — re-caption an existing marker. The label is stored VERBATIM; the mod caps
/// it at render time and the emitter applies that cap when it compiles, so capping here would
/// destroy the authored value in the one place the author could still see and fix it.
#[must_use]
pub fn set_marker_label(faction_id: &str, marker_id: &str, label: &str) -> bool {
    upsert_marker_field(faction_id, marker_id, |row| row.label = label.to_string())
}

/// ATTR-FIELD-MRK-POSITION — move a marker to `(x, z)` world metres. Non-finite input is refused
/// rather than stored: `$defs/marker` types both as `number`, and a NaN would serialise as JSON
/// `null` and fail the validator at save time, far from the box that produced it.
#[must_use]
pub fn set_marker_position(faction_id: &str, marker_id: &str, x: f64, z: f64) -> bool {
    if !x.is_finite() || !z.is_finite() {
        return false;
    }
    upsert_marker_field(faction_id, marker_id, |row| {
        row.x = x;
        row.z = z;
    })
}

/// Delete one marker. Its siblings and the briefing prose beside them are untouched
/// (`remove_faction_briefing_marker` reads the briefing and writes it back whole).
#[must_use]
pub fn remove_marker(faction_id: &str, marker_id: &str) -> bool {
    let exists = marker_rows()
        .iter()
        .any(|r| r.faction_id == faction_id && r.id == marker_id);
    if !exists {
        return false;
    }
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        if let Some(ctx) = guard.as_ref() {
            let d = ctx.doc.borrow();
            if let Some(core) = d.as_ref() {
                core.remove_faction_briefing_marker(faction_id, marker_id);
            }
        }
    });
    mission_history::after_local_edit();
    true
}

/// The shared edit body: read the row, apply `edit`, write it back through the store's UPSERT.
///
/// Read-modify-write rather than a per-field mutator because the store's writer takes the whole
/// `{x, z, icon, label}` tuple — it replaces the row in place. Editing one field therefore means
/// carrying the other three across, and doing that in one function is what stops a future caller
/// from re-captioning a marker back to the origin.
fn upsert_marker_field(
    faction_id: &str,
    marker_id: &str,
    edit: impl FnOnce(&mut MarkerRow),
) -> bool {
    let Some(mut row) = marker_rows()
        .into_iter()
        .find(|r| r.faction_id == faction_id && r.id == marker_id)
    else {
        return false;
    };
    edit(&mut row);
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        if let Some(ctx) = guard.as_ref() {
            let d = ctx.doc.borrow();
            if let Some(core) = d.as_ref() {
                core.set_faction_briefing_marker(
                    faction_id, marker_id, row.x, row.z, &row.icon, &row.label,
                );
            }
        }
    });
    mission_history::after_local_edit();
    true
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// T-697 — THE DOCUMENT INDEX (3den E4 / 3DEN-TOOL-011)
// ═════════════════════════════════════════════════════════════════════════════════════════════════
//
// The read half of document search: every placed thing in the mission, projected into the ONE row
// type the pure search in `eden_dock_left` consumes. The projection lives here because the doc
// handles are the wasm-only `!Send` `Rc`s this module exists to hold; it is deliberately a READ —
// nothing below opens a transaction, so nothing below can enter the undo stack.
//
// **EVERY KIND, OR THE SEARCH IS A LIE.** The eight readers are the eight collections an author can
// place into: `slots`, `vehiclesById`, `entitiesById`, briefing `markers`, `zones`, `triggers`,
// `commentsById` and `editorLayersById`. A ninth collection appearing without a case here would be
// silently unfindable, which is exactly the failure the ticket names.
//
// **BORROW DISCIPLINE.** `vehicle_rows` / `zone_rows` / `trigger_rows` / `marker_rows` each open
// their OWN `OPS_CTX` + doc borrow, so they are called AFTER the single borrow below has been
// dropped — the module's standing rule (see `placed_owner_options`, which does the same dance).

/// T-697 — a `faction-BLUFOR` id (or a raw side/library key) as the side label the search and the
/// selection filter group by. Empty in, empty out: a row that belongs to no faction says so.
fn side_label(raw: &str) -> String {
    raw.strip_prefix("faction-").unwrap_or(raw).to_uppercase()
}

/// T-697 — a non-empty display label, or the fallback the row's own panel would show.
fn or_fallback(label: &str, fallback: &str) -> String {
    let t = label.trim();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t.to_string()
    }
}

/// T-697 — push `(field, value)` only when `value` is non-blank. A blank attribute is not a
/// searchable attribute, and keeping it would let an empty query-side value match the empty string
/// and report a field that holds nothing.
fn push_text(text: &mut Vec<(&'static str, String)>, field: &'static str, value: &str) {
    let v = value.trim();
    if !v.is_empty() {
        text.push((field, v.to_string()));
    }
}

/// T-697 — **every placed entity in the mission, with its text attributes.** The input to
/// [`crate::editor::panels::dock_left::search_document`].
///
/// The per-kind attribute sets are the fields an author actually types into and would search by —
/// a slot's role/callsign/tag/rank/description, a marker's caption, a zone's label, a trigger's
/// name, a comment's title and body — plus, on every row, its ID and (where it has one) the tail of
/// its Enfusion class name. The id is in the set because quoting an id out of a validation finding
/// and pasting it into the search box is a real workflow; the class tail is in it so a plain
/// `UAZ` finds a vehicle whose only authored text is its `resourceName`.
#[must_use]
pub fn document_entities() -> Vec<crate::editor::panels::dock_left::DocEntity> {
    use crate::editor::panels::dock_left::{DocEntity, DocKind};

    let mut out: Vec<DocEntity> = Vec::new();

    // ── Pass 1: everything reachable from ONE doc borrow ─────────────────────────────────────────
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return;
        };

        // A slot's side is squad → faction → key: the join the ORBAT tree already makes.
        let side_of_faction: HashMap<String, String> = faction_rows(core)
            .into_iter()
            .map(|f| {
                let key = if f.key.is_empty() { f.name } else { f.key };
                (f.id, side_label(&key))
            })
            .collect();
        let side_of_squad: HashMap<String, String> = squad_rows(core)
            .into_iter()
            .map(|s| {
                let side = side_of_faction
                    .get(&s.faction_id)
                    .cloned()
                    .unwrap_or_default();
                (s.id, side)
            })
            .collect();

        let raw = raw_slot_rows(core);
        for s in slot_details(core) {
            let asset_id = row_str(&raw, &s.id, "assetId");
            let description = row_str(&raw, &s.id, "description");
            let mut text = Vec::new();
            push_text(&mut text, "role", &s.role);
            push_text(&mut text, "callsign", &s.callsign);
            push_text(&mut text, "tag", &s.tag);
            push_text(&mut text, "rank", &s.rank);
            push_text(&mut text, "description", &description);
            push_text(&mut text, "loadout", &s.summary);
            push_text(
                &mut text,
                "class",
                crate::editor::arsenal::asset_catalog::classname_tail(&asset_id),
            );
            push_text(&mut text, "id", &s.id);
            out.push(DocEntity {
                label: or_fallback(&s.role, &format!("Slot {}", s.id)),
                faction: side_of_squad.get(&s.squad_id).cloned().unwrap_or_default(),
                class_name: asset_id,
                kind: DocKind::Slot,
                id: s.id,
                text,
            });
        }

        // Placed world objects (T-254 `entitiesById`). No typed reader exists for them — the panels
        // that render objects render the CATALOGUE, not the placed rows — so this is the read.
        if let Ok(root) = serde_json::from_str::<serde_json::Value>(&core.small_maps_json()) {
            if let Some(map) = root.get("entitiesById").and_then(|v| v.as_object()) {
                for (id, v) in map {
                    let s = |k: &str| {
                        v.get(k)
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_string()
                    };
                    let alias = s("alias");
                    let resource_name = s("resourceName");
                    let mut text = Vec::new();
                    push_text(&mut text, "alias", &alias);
                    push_text(
                        &mut text,
                        "class",
                        crate::editor::arsenal::asset_catalog::classname_tail(&resource_name),
                    );
                    push_text(&mut text, "id", id);
                    out.push(DocEntity {
                        id: id.clone(),
                        kind: DocKind::Object,
                        label: or_fallback(&alias, &format!("Object {id}")),
                        faction: side_label(&s("faction")),
                        class_name: resource_name,
                        text,
                    });
                }
            }
        }

        for cm in comment_details(core) {
            let mut text = Vec::new();
            push_text(&mut text, "title", &cm.title);
            push_text(&mut text, "note", &cm.tooltip);
            push_text(&mut text, "id", &cm.id);
            out.push(DocEntity {
                label: or_fallback(&cm.title, &format!("Comment {}", cm.id)),
                kind: DocKind::Comment,
                class_name: String::new(),
                faction: String::new(),
                id: cm.id,
                text,
            });
        }

        for l in layer_rows(core) {
            let mut text = Vec::new();
            push_text(&mut text, "name", &l.name);
            push_text(&mut text, "id", &l.id);
            out.push(DocEntity {
                label: or_fallback(&l.name, &format!("Layer {}", l.id)),
                kind: DocKind::Layer,
                class_name: String::new(),
                faction: String::new(),
                id: l.id,
                text,
            });
        }
    });

    // ── Pass 2: the readers that open their own borrow ───────────────────────────────────────────
    for v in vehicle_rows() {
        let tail =
            crate::editor::arsenal::asset_catalog::classname_tail(&v.resource_name).to_string();
        let mut text = Vec::new();
        push_text(&mut text, "class", &tail);
        push_text(&mut text, "id", &v.id);
        out.push(DocEntity {
            label: or_fallback(&tail, &format!("Vehicle {}", v.id)),
            kind: DocKind::Vehicle,
            faction: side_label(&v.faction_id),
            class_name: v.resource_name,
            id: v.id,
            text,
        });
    }
    for z in zone_rows() {
        let label = z.label.clone().unwrap_or_default();
        let mut text = Vec::new();
        push_text(&mut text, "label", &label);
        push_text(&mut text, "type", &z.kind);
        push_text(&mut text, "id", &z.id);
        out.push(DocEntity {
            label: or_fallback(&label, &format!("Zone {}", z.id)),
            kind: DocKind::Zone,
            class_name: String::new(),
            faction: side_label(z.faction.as_deref().unwrap_or_default()),
            id: z.id,
            text,
        });
    }
    for t in trigger_rows() {
        let name = t.name.clone().unwrap_or_default();
        let mut text = Vec::new();
        push_text(&mut text, "name", &name);
        push_text(&mut text, "activation", &t.activation);
        push_text(&mut text, "id", &t.id);
        out.push(DocEntity {
            label: or_fallback(&name, &format!("Trigger {}", t.id)),
            kind: DocKind::Trigger,
            class_name: String::new(),
            faction: String::new(),
            id: t.id,
            text,
        });
    }
    for m in marker_rows() {
        let mut text = Vec::new();
        push_text(&mut text, "caption", &m.label);
        push_text(&mut text, "icon", &m.icon);
        push_text(&mut text, "id", &m.id);
        out.push(DocEntity {
            label: or_fallback(&m.label, &or_fallback(&m.icon, &format!("Marker {}", m.id))),
            kind: DocKind::Marker,
            class_name: String::new(),
            faction: side_label(&m.faction_id),
            id: m.id,
            text,
        });
    }
    out
}

/// T-697 — the CURRENT SELECTION, projected exactly as [`document_entities`] projects the document.
///
/// Derived from the document index rather than re-read per kind, so the selection filter's chips and
/// the search's rows can never disagree about what an entity's type or faction is. Selection order
/// is not preserved (the document order is): the chips are counts and id sets, and nothing
/// downstream reads a selection as a sequence.
#[must_use]
pub fn selection_entities() -> Vec<crate::editor::panels::dock_left::DocEntity> {
    let sel: Vec<String> = OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .map(|ctx| ctx.selection.borrow().clone())
            .unwrap_or_default()
    });
    if sel.is_empty() {
        return Vec::new();
    }
    document_entities()
        .into_iter()
        .filter(|e| sel.iter().any(|id| id == &e.id))
        .collect()
}

/// T-697 — replace the selection with `ids` (the selection filter's apply), returning how many
/// entities ended up selected.
///
/// Goes through [`set_slot_selection`], the SAME selection-only tail a folder click and a map click
/// take (engine tint + SEL readout + dock highlight, no doc edit ⇒ **no undo step**). Narrowing a
/// selection is not authored content, so it must not enter the history — the T-642 line, and the
/// reason this is not a mutator. An empty `ids` is refused rather than obeyed: "narrow to nothing"
/// is never what an author meant by a filter chip, and the pure `selection_facets` never emits one.
pub fn set_selection_ids(ids: Vec<String>) -> usize {
    if ids.is_empty() {
        return 0;
    }
    let n = ids.len();
    set_slot_selection(ids);
    n
}
