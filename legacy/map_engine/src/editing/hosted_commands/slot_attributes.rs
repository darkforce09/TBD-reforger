//! Role: the Attributes panel's reads and commits over the hosted document.
//! Position: `editing/hosted_commands` in the map engine.
//! Signals & state: none of its own; the document and the selected ids come from the host.
//! Invariants: a commit that changed nothing runs no post-change tail, and a multi-slot commit
//! runs exactly ONE tail for the whole set — the fan-out lives inside the document's own batch, so
//! an apply-to-all is one undo step rather than one per slot. A `None` argument is a field the
//! operator did not opt in; the document mutators leave those columns untouched.

use crate::data::store::operations::attrs;
use crate::editing::history::after_local_edit;
use crate::editing::host::{selection_ids, with_doc};

/// Which fields disagree across a multi-slot selection.
pub use crate::data::store::operations::attrs::AttrDiff;

/// One slot's editable attributes, as the panel's fields read them.
pub use crate::data::store::operations::attrs::SlotAttrs;

/// Read one slot's editable attributes for the modal's field values. `None` when the slot no
/// longer exists (undone away while open → the modal closes).
pub fn read_attrs(id: &str) -> Option<SlotAttrs> {
    with_doc(|core| attrs::read_attrs(core, id)).flatten()
}

/// The modal needs the COUNT, not a bool, because a multi-selection can straddle the lock: all
/// locked ⇒ the Transform fields are disabled outright; some locked ⇒ the fields stay live (the
/// unlocked members really will move) and the modal says how many will not. Reporting either case
/// as the other is a lie in a new costume.
#[must_use]
pub fn attrs_locked_count(ids: &[String]) -> usize {
    with_doc(|core| attrs::attrs_locked_count(core, ids)).unwrap_or(0)
}

/// Attributes Transform commit — x/y clamp to terrain bounds, rotation normalizes, manual z
/// sticks — plus the shared post-change tail, so one commit is one undo step.
pub fn attrs_update_position(
    id: &str,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    rotation: Option<f64>,
) {
    let did =
        with_doc(|core| attrs::attrs_update_position(core, id, x, y, z, rotation)).unwrap_or(false);
    if did {
        after_local_edit();
    }
}

/// Field-by-field, exactly like the single-slot [`attrs_update_position`]: a `None` argument is a
/// field the operator did not opt in (its checkbox is unticked), and the slot mutator leaves those
/// columns untouched — so ticking "Rotation" and typing a heading can never also stamp one slot's
/// X onto the rest of the selection.
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
    let did = with_doc(|core| attrs::attrs_update_position_multi(core, ids, x, y, z, rotation))
        .unwrap_or(false);
    if did {
        after_local_edit();
    }
}

/// Attributes Identity/stance commit — role, tag, stance, type and description — plus the shared
/// tail.
pub fn attrs_update_slot(
    id: &str,
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
) {
    if role.is_none()
        && tag.is_none()
        && stance.is_none()
        && asset_id.is_none()
        && description.is_none()
    {
        return;
    }
    let did = with_doc(|core| {
        attrs::attrs_update_slot(core, id, role, tag, stance, asset_id, description)
    })
    .unwrap_or(false);
    if did {
        after_local_edit();
    }
}

/// The identity half of an apply-to-all. `slot_half` says whether any of the three slot columns
/// was opted in, so a type-only or description-only commit never opens a slot transaction.
pub fn attrs_update_slot_multi(
    ids: &[String],
    role: Option<String>,
    tag: Option<String>,
    stance: Option<String>,
    asset_id: Option<String>,
    description: Option<String>,
) {
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
    let did = with_doc(|core| {
        attrs::attrs_update_slot_multi(
            core,
            ids,
            role,
            tag,
            stance,
            asset_id,
            description,
            slot_half,
        )
    })
    .unwrap_or(false);
    if did {
        after_local_edit();
    }
}

/// The multi-edit target set for the slot the modal opened on: empty unless at least two ids are
/// selected and the open slot is one of them, and filtered to the slot SoA so the header never
/// counts a vehicle as an editable slot.
#[must_use]
pub fn attrs_multi_ids(open_id: &str) -> Vec<String> {
    let sel = selection_ids();
    if sel.len() < 2 || !sel.iter().any(|s| s == open_id) {
        return Vec::new();
    }
    with_doc(|core| {
        let soa = core.materialize();
        let ids: Vec<String> = sel
            .into_iter()
            .filter(|s| soa.ids.iter().any(|r| r == s))
            .collect();
        if ids.len() < 2 { Vec::new() } else { ids }
    })
    .unwrap_or_default()
}

/// Which of the edited set's fields disagree, over one materialized snapshot.
#[must_use]
pub fn read_attrs_diff(ids: &[String]) -> AttrDiff {
    if ids.len() < 2 {
        return AttrDiff::default();
    }
    with_doc(|core| attrs::read_attrs_diff(core, ids)).unwrap_or_default()
}
