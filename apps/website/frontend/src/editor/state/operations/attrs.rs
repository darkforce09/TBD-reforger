//! Role: attrs.
//! Position: `editor/state/operations` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use crate::editor::state::history as mission_history;

/// Expose website mission core :: doc :: operations :: attrs :: keep z rows at this domain boundary.
pub(crate) use website_map_engine::data::store::operations::attrs::keep_z_rows;

/// Expose website mission core :: doc :: operations :: attrs :: slot z at this domain boundary.
pub(crate) use website_map_engine::data::store::operations::attrs::slot_z;

/// Expose website mission core :: doc :: operations :: attrs ::  attr diff at this domain boundary.
pub use website_map_engine::data::store::operations::attrs::AttrDiff;

/// Expose website mission core :: doc :: operations :: attrs ::  slot attrs at this domain boundary.
pub use website_map_engine::data::store::operations::attrs::SlotAttrs;

#[allow(unused_imports)]
use super::{cargo::*, compositions::*, context::*, entity::*, transform::*};

/// Read one slot's editable attributes for the modal's field values. `None` when the slot no longer exists (undone away while open → the modal closes).
pub fn read_attrs(id: &str) -> Option<SlotAttrs> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::attrs::read_attrs(core, id)
    })
}

/// The modal needs the COUNT, not a bool, because a multi-selection can straddle the lock: all locked ⇒ the Transform fields are disabled outright; some locked ⇒ the fields stay live (the unlocked members really will move) and the modal says how many will not. Reporting either case as the other is the F-7 lie in a new costume.
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
        website_map_engine::data::store::operations::attrs::attrs_locked_count(core, ids)
    })
}

/// Attributes Transform commit — `update_slot_position` (x/y clamp to terrain bounds, rotation normalizes, manual z sticks) + the shared post-change tail (A4: one commit = one undo step).
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
        website_map_engine::data::store::operations::attrs::attrs_update_position(
            core, id, x, y, z, rotation,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// Field-by-field, exactly like the single-slot [`attrs_update_position`]: a `None` argument is a field the operator did not opt in (its checkbox is unticked), and the slot mutator leaves those columns untouched — so ticking "Rotation" and typing a heading can never also stamp one slot's X onto the rest of the selection.
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
        website_map_engine::data::store::operations::attrs::attrs_update_position_multi(
            core, ids, x, y, z, rotation,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// Attributes Identity/stance commit — `update_slot(role/tag/stance)` + the shared tail.
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
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        website_map_engine::data::store::operations::attrs::attrs_update_slot(
            core,
            id,
            role,
            tag,
            stance,
            asset_id,
            description,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// Attrs update slot multi using the supplied domain data.
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
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        website_map_engine::data::store::operations::attrs::attrs_update_slot_multi(
            core,
            ids,
            role,
            tag,
            stance,
            asset_id,
            description,
            slot_half,
        )
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// Attrs multi ids using the supplied domain data.
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

/// Attrs selection len using the supplied domain data.
#[must_use]
pub fn attrs_selection_len() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .map(|ctx| ctx.selection.borrow().len())
            .unwrap_or(0)
    })
}

/// Read attrs diff using the supplied domain data.
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
        website_map_engine::data::store::operations::attrs::read_attrs_diff(core, ids)
    })
}
