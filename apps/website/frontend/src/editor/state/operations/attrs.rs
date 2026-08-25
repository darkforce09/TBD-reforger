//! T-934.7 — Attributes-modal reads/writes of the old `state/operations.rs`: `read_attrs`,
//! the `attrs_update_*` commits, sticky-z helpers and `AttrDiff`.
//! Split from `operations.rs`; the façade re-exports keep paths stable.

use crate::editor::state::history as mission_history;
use map_engine_core::doc::{EntityTransformPatch, MissionDocCore, NONE_IDX};

#[allow(unused_imports)]
use super::{cargo::*, compositions::*, context::*, entity::*, transform::*};

/// One slot's editable attributes for the Attributes modal.
///
/// T-082 — read from TWO sources, not one, and the split is the whole of this ticket. `x`/`y`/`z`/
/// `rotation`/`stance`/`role`/`tag`/`squad` come from the materialized SoA, which is the render
/// projection and carries only the columns the GPU needs. `asset_id` and `description` are NOT in
/// it — `SlotSoa` has no such column and never will — so they come from the raw slot row
/// (`slots_json`). Before this ticket `read_attrs` read the SoA alone, which is why the entity TYPE
/// was unreadable in the modal even though the core could already write it: the field was missing
/// from the READ path, not from the mutator.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotAttrs {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub rotation: f64,
    pub stance: String,
    pub role: String,
    pub tag: String,
    pub squad: String,
    /// T-082 ATTR-FIELD-OBJ-TYPE — the slot's `assetId` (the entity type it spawns as). Empty when
    /// unset, which is the common case: a slot with no `assetId` compiles to its faction's default
    /// kit alias.
    pub asset_id: String,
    /// T-082 ATTR-FIELD-OBJ-ROLE-DESC — Eden's free-text "Role Description". A field of its OWN:
    /// `role` is the short label ("Rifleman") the ORBAT and the compiled document use, and having it
    /// double as the prose description is precisely the gap this ticket closes.
    pub description: String,
}

/// T-082 — every slot row of the doc, keyed by id, straight off `slots_json()`.
///
/// The raw rows, NOT the SoA: `assetId` and `description` (and every other authored key) live only
/// here. Parsed once per call and handed to the readers below, because `slots_json` is O(all slots)
/// JSON and the modal must not pay it per field. Both callers already pay one `materialize()` of
/// the same order, and both run on a modal render — never the frame loop.
pub(super) fn raw_slot_rows(core: &MissionDocCore) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::from_str::<serde_json::Value>(&core.slots_json()) {
        Ok(serde_json::Value::Object(m)) => m,
        _ => serde_json::Map::new(),
    }
}

/// T-082 — one string key off a raw slot row; empty when absent or not a string (the `add_slot`
/// omit idiom means "absent" is the canonical unset, so it must read back as empty, not as a hole).
pub(super) fn row_str(
    rows: &serde_json::Map<String, serde_json::Value>,
    id: &str,
    key: &str,
) -> String {
    rows.get(id)
        .and_then(|r| r.get(key))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// **wave-127 F-2** — one slot's CURRENT `z`, exact, straight off the raw row. `None` when the slot
/// is absent or carries no finite numeric `z`.
///
/// Read from the raw row rather than the materialized SoA for two reasons, both of which would
/// corrupt the value this exists to preserve: the SoA's `zs` column is **f32**, so round-tripping an
/// authored `z` through it would rewrite it as a slightly different number on every X tweak; and a
/// slot filed under a HIDDEN layer is omitted from the SoA entirely (T-665), so the read would fail
/// on precisely the slots a careful operator has tucked away — and a failed read is a zeroed z.
pub(crate) fn slot_z(rows: &serde_json::Map<String, serde_json::Value>, id: &str) -> Option<f64> {
    rows.get(id)?
        .get("position")?
        .get("z")?
        .as_f64()
        .filter(|v| v.is_finite())
}

/// **wave-127 F-2** — the slot rows an Attributes position commit needs in order to KEEP an authored
/// `z`; `None` when this commit cannot zero one, and the callers then skip the read entirely.
///
/// `update_slot_position` terrain-follows on any x/y write — `z = None` with `x` or `y` set stores
/// `pz = 0.0`. In the JS oracle that is harmless because the caller re-samples the DEM and writes the
/// real elevation straight back. **In this frontend nothing re-samples after an Attributes commit**:
/// `terrainZ` did not survive the React deletion (the only mention left in this module is a comment
/// on [`place_at`]), so the document simply keeps the literal `0.0`. An operator who authored a
/// rooftop `z` and later nudged X by a metre in the Transform tab lost that `z` — silently, and
/// inside the same undo step as the X edit.
///
/// The fix is here, at the CALLER, not in the mutator: `MissionDocCore::update_slot_position` claims
/// byte-parity with `ydoc.updateSlotPosition` and keeps it. Its callers read the current `z` and pass
/// it back in, which makes the terrain-follow a no-op for the paths that have no sampler behind them.
///
/// **wave-127 F-5 — the placement helpers are one of those paths.** [`commit_positions`] (Align /
/// Distribute / the placement patterns) writes x/y with `z = None` for every slot it moves, so it
/// zeroed an authored z exactly the way the Attributes tab did — while preserving it for vehicles in
/// the same selection. It now resolves the z through this pair too.
///
/// **wave-127 F-6 — and so does the marquee/handle DRAG.** It commits through
/// `move_entities_and_vehicles` in `mission_editor` (the `LG::Move` arm), which used to pass
/// `vec![0.0; n]` and so flattened every dragged slot inside one txn. That caller lives outside this
/// module, which is why this pair is `pub(crate)`: the drag reads the rows ONCE and maps them over
/// its `slot_ids` in order. One z-resolution vocabulary for all three paths, not three.
///
/// Reading the rows is O(document) JSON, so it is done once per COMMIT — once per BATCH for
/// `commit_positions`, which moves many entities — and only for a commit that could actually zero a
/// `z`: an explicit z write, or a rotation-only edit, never reaches it.
pub(crate) fn keep_z_rows(
    core: &MissionDocCore,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    (z.is_none() && (x.is_some() || y.is_some())).then(|| raw_slot_rows(core))
}

/// Read one slot's editable attributes for the modal's field values.
/// `None` when the slot no longer exists (undone away while open → the modal closes).
///
/// T-082 — the SoA supplies the transform/identity columns; the raw row (`raw_slot_rows`) supplies
/// `assetId` and `description`, which the SoA does not carry. See [`SlotAttrs`] for why that split
/// is the ticket rather than an implementation detail.
///
/// **T-744 (wave-113 F-2)** — existence is RAW membership, not SoA membership.
/// `MissionDocCore::materialize()` drops T-665 layer-hidden and T-701 `editorHidden` slots before
/// any column is pushed. Gating `Option` on `soa.ids.iter().position(…)?` made Hide close the
/// Attributes modal through the same `None` → `close_attributes()` path as "slot was undone away".
/// Transform/identity columns still prefer the SoA when the slot is visible; when materialize has
/// filtered it, those columns fall back to the raw row so the open modal keeps real field values
/// (a zeroed fallback would corrupt the next Transform commit).
pub fn read_attrs(id: &str) -> Option<SlotAttrs> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let rows = raw_slot_rows(core);
        // T-744 — Option gate is RAW existence. Hide must not look like undo-away.
        if !rows.contains_key(id) {
            return None;
        }
        let soa = core.materialize();
        if let Some(row) = soa.ids.iter().position(|s| s == id) {
            let dict = |idx: u32, dict: &[String]| {
                if idx == NONE_IDX {
                    String::new()
                } else {
                    dict.get(idx as usize).cloned().unwrap_or_default()
                }
            };
            let stance = match soa.stance.get(row).copied().unwrap_or(0) {
                map_engine_core::doc::STANCE_CROUCH => "crouch",
                map_engine_core::doc::STANCE_PRONE => "prone",
                _ => "stand",
            };
            Some(SlotAttrs {
                id: id.to_string(),
                x: f64::from(soa.xs[row]),
                y: f64::from(soa.ys[row]),
                z: f64::from(soa.zs[row]),
                rotation: f64::from(soa.rotations[row]),
                stance: stance.to_string(),
                role: dict(soa.role_idx[row], &soa.roles),
                tag: dict(soa.tag_idx[row], &soa.tags),
                squad: dict(soa.squad_idx[row], &soa.squads),
                asset_id: row_str(&rows, id, "assetId"),
                description: row_str(&rows, id, "description"),
            })
        } else {
            // Hidden (layer or editorHidden) but still in the doc — keep the modal open.
            Some(slot_attrs_from_raw(&rows, id))
        }
    })
}

/// T-744 — Attributes snapshot from a raw `slots_json` row. Used when `materialize()` has filtered
/// the slot (hide) so the modal can stay open without inventing zeros for Transform fields.
fn slot_attrs_from_raw(rows: &serde_json::Map<String, serde_json::Value>, id: &str) -> SlotAttrs {
    let pos = rows.get(id).and_then(|r| r.get("position"));
    let num = |key: &str| -> f64 {
        pos.and_then(|p| p.get(key))
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0)
    };
    // Prefer the exact-z helper (finite filter) so a malformed z does not display as NaN-coerced 0.
    let z = slot_z(rows, id).unwrap_or_else(|| num("z"));
    let stance_raw = row_str(rows, id, "stance");
    let stance = match stance_raw.as_str() {
        "crouch" => "crouch",
        "prone" => "prone",
        _ => "stand",
    };
    SlotAttrs {
        id: id.to_string(),
        x: num("x"),
        y: num("y"),
        z,
        rotation: num("rotation"),
        stance: stance.to_string(),
        role: row_str(rows, id, "role"),
        tag: row_str(rows, id, "tag"),
        squad: row_str(rows, id, "squadId"),
        asset_id: row_str(rows, id, "assetId"),
        description: row_str(rows, id, "description"),
    }
}

/// T-082 (wave-102 F-7) — how many of `ids` are transform-locked.
///
/// The modal needs the COUNT, not a bool, because a multi-selection can straddle the lock: all
/// locked ⇒ the Transform fields are disabled outright; some locked ⇒ the fields stay live (the
/// unlocked members really will move) and the modal says how many will not. Reporting either case
/// as the other is the F-7 lie in a new costume.
///
/// Asks the CORE (`slot_layer_is_locked`), never a re-derived layer walk here: the whole value of
/// the affordance is that it cannot disagree with the mutator that refuses the write.
#[must_use]
pub fn attrs_locked_count(ids: &[String]) -> usize {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return 0;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return 0;
        };
        ids.iter()
            .filter(|id| core.slot_layer_is_locked(id))
            .count()
    })
}

/// Attributes Transform commit — `update_slot_position` (x/y clamp to terrain bounds, rotation
/// normalizes, manual z sticks) + the shared post-change tail (A4: one commit = one undo step).
///
/// T-082 (wave-102 F-7) — a slot the core will REFUSE (transform-locked layer) no longer fires the
/// tail. `did` used to be "the ops context and the document both exist", which is not the same
/// question as "did anything change": a refused write still bumped `doc_ver`, marked the mission
/// DIRTY and armed a persist for an edit that never happened. The UI half of F-7 is the disabled
/// affordance the modal draws from [`attrs_locked_count`]; this is the state half.
///
/// **T-775 — "did the number change?" is answered by the FIELD, not here, and that is deliberate.**
/// This function has no equality skip: `did` means "the core did not refuse the slot", so every call
/// that reaches it rewrites `position` and fires `after_local_edit()`. That made a focus/blur on an
/// untouched coordinate a real edit — a dirty mission, an armed persist and an undo step for
/// nothing. The guard belongs at `attributes::number_field`'s `commit`, which is the only place that
/// knows the settled value the operator started from.
///
/// It is not merely convenient to guard there, it is the only correct place: this layer cannot see
/// what the operator started from — it is handed a number, not an edit. "Same x" and "same document"
/// were also different questions here for a second reason that no longer holds: an x-only call used
/// to terrain-follow the slot's z to `0.0`, so it changed the document even when `x` was unchanged.
///
/// **wave-127 F-2 — an x/y edit no longer discards an authored z.** The comment that used to sit here
/// said the zeroed z was "DEM re-sampled JS-side"; that is FALSE for this path — nothing follows an
/// Attributes commit to re-sample, so the `0.0` was final and an authored rooftop z died under a 1 m
/// X nudge. [`keep_z_rows`] explains the fix and why it lives at this caller instead of in the core
/// mutator.
pub fn attrs_update_position(
    id: &str,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        // T-649 — was an inline copy of `terrain_bounds_of` (T-650 added the identical helper
        // below); both this and the multi commit now resolve the clamp through the one function so
        // they cannot drift apart.
        if core.slot_layer_is_locked(id) {
            return false; // the core would skip this write; do not report it as an edit
        }
        // wave-127 F-2 — an x/y edit carries the slot's CURRENT z back in, so the core's
        // terrain-follow cannot flatten an authored one. See `keep_z_rows`.
        let z = z.or_else(|| keep_z_rows(core, x, y, z).and_then(|rows| slot_z(&rows, id)));
        let b = terrain_bounds_of(core);
        core.update_slot_position(id, x, y, z, rotation, b[2], b[3]);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// T-649 ATTR-MULTI-001 — the Transform commit applied to EVERY id in `ids`.
///
/// Field-by-field, exactly like the single-slot [`attrs_update_position`]: a `None` argument is a
/// field the operator did not opt in (its checkbox is unticked), and the slot mutator leaves those
/// columns untouched — so ticking "Rotation" and typing a heading can never also stamp one slot's
/// X onto the rest of the selection.
///
/// **Undo (T-732):** one LOCAL txn via [`MissionDocCore::update_entity_transforms`] = **one** undo
/// step for the whole stamp (N Ctrl+Z for N slots was the pre-T-732 defect). Still fires **one**
/// history/persist tail (`after_local_edit` once, below).
pub fn attrs_update_position_multi(
    ids: &[String],
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) {
    if ids.is_empty() || (x.is_none() && y.is_none() && z.is_none() && rotation.is_none()) {
        return;
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
        let b = terrain_bounds_of(core);
        // wave-127 F-2 — read ONCE for the whole stamp, then hand each member its own z back. An X
        // stamp across a selection used to flatten every member's authored z in one undo step.
        let rows = keep_z_rows(core, x, y, z);
        // T-082 (F-7) — `update_entity_transforms` returns how many patches wrote; locked / unknown
        // ids are skipped inside the one txn, so an all-locked selection yields 0 and skips the tail.
        let mut patches: Vec<EntityTransformPatch> = Vec::with_capacity(ids.len());
        for id in ids {
            if core.slot_layer_is_locked(id) {
                continue;
            }
            let z = z.or_else(|| rows.as_ref().and_then(|r| slot_z(r, id)));
            patches.push(EntityTransformPatch {
                id: id.clone(),
                is_slot: true,
                x,
                y,
                z,
                rotation,
            });
        }
        core.update_entity_transforms(&patches, b[2], b[3]) > 0
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// Attributes Identity/stance commit — `update_slot(role/tag/stance)` + the shared tail.
///
/// T-082 — `asset_id` (ATTR-FIELD-OBJ-TYPE) and `description` (ATTR-FIELD-OBJ-ROLE-DESC) ride the
/// SAME commit seam under the same `None`-means-not-opted-in discipline, but land through a second
/// core mutator (`update_slot_object`) because they are not `update_slot` columns. Each is a no-op
/// when nothing in its half is `Some`, so a role keystroke opens exactly one transaction and a type
/// keystroke opens exactly one — the modal's one-commit-one-undo-step contract is unchanged.
/// (`update_slot_role_character` is deliberately NOT the writer here; see its counterpart's note on
/// `MissionDocCore::update_slot_object` for why routing a type edit through it would wipe `tag`.)
///
/// NOT gated on the transform lock, and that is the core's rule rather than an omission: T-665 locks
/// TRANSFORM only, so identity/type/description edits are legal on a locked slot.
///
/// **T-745 (wave-113 F-3)** — `did` used to mean "ops context + document both exist", so an
/// all-`None` call (or a call against a nonexistent id) still ran `after_local_edit` and armed a
/// false save. Mirror [`attrs_update_slot_multi`]: only fire the tail when something actually
/// changed. Existence is RAW (`raw_slot_rows`), not SoA — a hidden slot is still real (T-744).
pub fn attrs_update_slot(
    id: &str,
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
) {
    // T-745 — nothing opted in ⇒ no write, no history/persist tail. Same shape as the multi peer
    // (T-082 widened that guard by the object fields; the single-target path lacked it).
    if role.is_none()
        && tag.is_none()
        && stance.is_none()
        && asset_id.is_none()
        && description.is_none()
    {
        return;
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
        // T-745 — a nonexistent id is a core no-op; do not bump doc_ver / dirty / persist.
        // RAW map, not SoA: Hide filters the SoA (T-744) but the row is still in the document.
        if !raw_slot_rows(core).contains_key(id) {
            return false;
        }
        if role.is_some() || tag.is_some() || stance.is_some() {
            core.update_slot(id, role, tag, stance);
        }
        core.update_slot_object(id, asset_id, description);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
}

/* ─────────── T-649 ATTR-MULTI-001 / ATTR-MULTI-CHK-001 — multi-selection Attributes ─────────── */

/// T-649 — the Identity/stance commit applied to EVERY id in `ids`. Peer of
/// [`attrs_update_position_multi`]; same `None`-means-not-opted-in field discipline (`update_slot`
/// leaves a `None` column alone) and the same one-tail / N-undo-steps honesty note.
pub fn attrs_update_slot_multi(
    ids: &[String],
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
) {
    // Nothing opted in ⇒ no writes at all. T-082 widened this guard by the two new fields: a commit
    // that opts into NEITHER half must stay a no-op, not become N transactions of `None`.
    if ids.is_empty()
        || (role.is_none()
            && tag.is_none()
            && stance.is_none()
            && asset_id.is_none()
            && description.is_none())
    {
        return;
    }
    let slot_half = role.is_some() || tag.is_some() || stance.is_some();
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        // F-26 (T-788) — ONE txn for the whole apply-to-all, so a multi-slot identity/type commit is
        // ONE undo step (was N — the loop opened a `begin()` per id under `capture_timeout_millis =
        // 0`, so a 9-slot Role apply cost 9 Ctrl+Z). `update_slots_attr_batch` runs the SAME per-slot
        // `update_slot` / `update_slot_object` logic the loop called (the shared `_in_txn` helpers),
        // so every slot's bytes are identical — only the transaction boundary collapses. The T-082
        // object half rides the same call under the same per-field `Option` discipline. This mirrors
        // T-732's `attrs_update_position_multi` → `update_entity_transforms` batch exactly.
        core.update_slots_attr_batch(
            ids,
            slot_half,
            role.clone(),
            tag.clone(),
            stance.clone(),
            asset_id.clone(),
            description.clone(),
        );
        true
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// T-649 — the slot ids the Attributes modal is editing when it opened over a MULTI-selection.
///
/// An **empty** return means single-edit, and the modal renders exactly as it always has (no
/// checkboxes anywhere). It is non-empty only when both:
///   * the live selection still contains `open_id` — `open_attrs_modal` already collapses a
///     right-click that retargeted outside the selection, so this re-check is what keeps the modal
///     honest if a dock edits the selection while it is open; and
///   * at least two of the selected ids are real slot rows.
///
/// Vehicles are filtered out on purpose: every field in this modal is a slot-SoA column
/// (x/y/z/rotation/stance/role/tag) and `vehiclesById` rows have none of them, so counting a
/// vehicle would show "N selected" while a Role write silently missed it. T-741: the modal header
/// must name this slot subset (and say vehicles are excluded when [`attrs_selection_len`] is
/// wider) — never report the full selection as the write set.
#[must_use]
pub fn attrs_multi_ids(open_id: &str) -> Vec<String> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let sel = ctx.selection.borrow().clone();
        if sel.len() < 2 || !sel.iter().any(|s| s == open_id) {
            return Vec::new();
        }
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
        let soa = core.materialize();
        let ids: Vec<String> = sel
            .into_iter()
            .filter(|s| soa.ids.iter().any(|r| r == s))
            .collect();
        if ids.len() < 2 {
            Vec::new()
        } else {
            ids
        }
    })
}

/// T-741 — live selection length (slots + vehicles + …). Attributes multi-edit compares this to
/// [`attrs_multi_ids`]'s slot subset so the header can say when vehicles were excluded from the
/// write set (wave-112 NIT-4 / Ctrl+A via `view_ids_with_vehicles`).
#[must_use]
pub fn attrs_selection_len() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .map(|ctx| ctx.selection.borrow().len())
            .unwrap_or(0)
    })
}

/// T-649 ATTR-MULTI-CHK-001 — which Attributes fields DISAGREE across a multi-selection.
///
/// Eden's multi-edit rule has two halves. This is the first: a field whose value is identical on
/// every selected entity can show that value; a field whose values differ has no value to show, so
/// the modal blanks it and disables it until its per-field checkbox opts it in. `attributes.rs`
/// owns the second half (the checkbox + the disable).
///
/// A single `materialize()` feeds every comparison, so the flags are a consistent snapshot of one
/// doc state rather than seven independent reads. Floats compare by **bits**, not by `==`: the
/// question is "is this literally the same stored value", and bit compare answers it exactly
/// without an epsilon that would call two genuinely different headings equal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttrDiff {
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub rotation: bool,
    pub stance: bool,
    pub role: bool,
    pub tag: bool,
    /// T-082 ATTR-FIELD-OBJ-TYPE — compared off the RAW rows, not the SoA (it has no such column).
    pub asset_id: bool,
    /// T-082 ATTR-FIELD-OBJ-ROLE-DESC — same, and for the same reason.
    pub description: bool,
}

impl AttrDiff {
    /// True when at least one field disagrees — the modal's "Multiple values" hint.
    #[must_use]
    pub fn any(self) -> bool {
        self.x
            || self.y
            || self.z
            || self.rotation
            || self.stance
            || self.role
            || self.tag
            || self.asset_id
            || self.description
    }
}

/// T-649 — [`AttrDiff`] for `ids`. Fewer than two resolvable rows ⇒ all-false (nothing can differ).
#[must_use]
pub fn read_attrs_diff(ids: &[String]) -> AttrDiff {
    if ids.len() < 2 {
        return AttrDiff::default();
    }
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return AttrDiff::default();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return AttrDiff::default();
        };
        let soa = core.materialize();
        // T-082 — carry the ID alongside the SoA row index. The two new fields are compared off the
        // raw slot rows (the SoA has no `assetId` / `description` column) and those are keyed by id,
        // so the SoA index alone is no longer enough to name a member of the selection. The MEMBER
        // SET is still exactly the set that resolves in the SoA, so which entities are compared is
        // unchanged — only how many columns are compared over them.
        let rows: Vec<(&String, usize)> = ids
            .iter()
            .filter_map(|id| Some((id, soa.ids.iter().position(|s| s == id)?)))
            .collect();
        let Some((&(first_id, first), rest)) = rows.split_first() else {
            return AttrDiff::default();
        };
        let raw = raw_slot_rows(core);
        // Resolve dict-coded columns to their STRINGS before comparing: `materialize()` gives no
        // guarantee that two rows carrying the same role text share an index, so an index compare
        // could report a difference the operator cannot see in the field.
        let text = |idx: u32, dict: &[String]| {
            if idx == NONE_IDX {
                String::new()
            } else {
                dict.get(idx as usize).cloned().unwrap_or_default()
            }
        };
        let mut d = AttrDiff::default();
        for &(id, r) in rest {
            d.x |= soa.xs[r].to_bits() != soa.xs[first].to_bits();
            d.y |= soa.ys[r].to_bits() != soa.ys[first].to_bits();
            d.z |= soa.zs[r].to_bits() != soa.zs[first].to_bits();
            d.rotation |= soa.rotations[r].to_bits() != soa.rotations[first].to_bits();
            d.stance |= soa.stance.get(r).copied().unwrap_or(0)
                != soa.stance.get(first).copied().unwrap_or(0);
            d.role |= text(soa.role_idx[r], &soa.roles) != text(soa.role_idx[first], &soa.roles);
            d.tag |= text(soa.tag_idx[r], &soa.tags) != text(soa.tag_idx[first], &soa.tags);
            // T-082 — absent reads back as `""` (`row_str`), so "one slot has no type and the other
            // has one" is a DIFFERENCE, which is what the operator sees in the field.
            d.asset_id |= row_str(&raw, id, "assetId") != row_str(&raw, first_id, "assetId");
            d.description |=
                row_str(&raw, id, "description") != row_str(&raw, first_id, "description");
        }
        d
    })
}
