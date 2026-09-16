//! Role: zone draw.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;
use website_map_engine::data::store::operations::entity::ZoneDrawStep;

/// Is a zone draw in flight?.
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

/// Begin zone draw using the supplied domain data.
pub fn begin_zone_draw(kind: &str, shape: ZoneShape, collection: DrawTarget) -> bool {
    let valid = website_map_engine::data::store::operations::entity::zone_draft_kind_is_valid(
        kind, collection, zone_types,
    );
    if !valid {
        return false;
    }
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        *ctx.pending.borrow_mut() = Some(Pending::Zone(
            website_map_engine::data::store::operations::entity::begin_zone_draft(
                kind.to_string(),
                shape,
                collection,
                None,
            ),
        ));
        true
    })
}

/// Begin zone reshape using the supplied domain data.
pub fn begin_zone_reshape(row_id: &str, shape: ZoneShape, collection: DrawTarget) -> bool {
    let kind = match collection {
        DrawTarget::Zone => zone_rows()
            .into_iter()
            .find(|r| r.id == row_id)
            .map(|r| r.kind),

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
        *ctx.pending.borrow_mut() = Some(Pending::Zone(
            website_map_engine::data::store::operations::entity::begin_zone_draft(
                kind,
                shape,
                collection,
                Some(row_id.to_string()),
            ),
        ));
        true
    })
}

/// Abandon the in-flight draw without writing anything. The explicit counterpart to [`cancel_pending`], which a zone draw deliberately survives. Returns whether a draw was actually abandoned — `false` when nothing was in flight (or the ops context is not up).
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
            return website_map_engine::data::store::operations::entity::pop_zone_draft_vertex(d);
        }
        0
    })
}

/// One canvas release while a zone draw is armed.
pub(in crate::editor::state::operations) fn advance_zone_draw(x: f64, z: f64) -> bool {
    let step = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let mut p = ctx.pending.borrow_mut();
        let Some(Pending::Zone(d)) = p.as_mut() else {
            return None;
        };
        let step = website_map_engine::data::store::operations::entity::advance_zone_draft(
            d,
            x,
            z,
            circle_from_clicks,
        );
        if matches!(step, ZoneDrawStep::CircleClosed { .. }) {
            *p = None;
        }
        Some(step)
    });
    match step {
        Some(ZoneDrawStep::CircleClosed {
            kind,
            centre: (cx, cz),
            radius: r,
            target,
            collection,
        }) => match (collection, target) {
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

        Some(ZoneDrawStep::Drawing) | None => {
            bump_doc_tick();
            zone_draw_armed()
        }
    }
}

/// Close the in-flight ring. Refuses below three vertices — `$defs/polygon` is `minItems: 3` and a two-vertex ring is a document the schema rejects.
pub fn close_zone_polygon() -> bool {
    let taken = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let mut p = ctx.pending.borrow_mut();
        let Some(Pending::Zone(d)) = p.as_ref() else {
            return None;
        };
        let commit = website_map_engine::data::store::operations::entity::close_zone_polygon_draft(
            d,
            polygon_is_committable,
        )?;
        *p = None;
        Some(commit)
    });
    let Some(commit) = taken else {
        return false;
    };
    let kind = commit.kind;
    let flat = polygon_flat(&commit.ring);
    match (commit.collection, commit.target) {
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

/// Write row using the supplied domain data.
pub(in crate::editor::state::operations) fn write_row(
    collection: DrawTarget,
    f: impl FnOnce(&MissionDocCore, &str),
) -> bool {
    write_row_returning_id(collection, f).is_some()
}

/// Write row returning id using the supplied domain data.
pub(in crate::editor::state::operations) fn write_row_returning_id(
    collection: DrawTarget,
    f: impl FnOnce(&MissionDocCore, &str),
) -> Option<String> {
    let id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::entity::write_row_returning_id(
            core, collection, f,
        )
    });
    if id.is_some() {
        mission_history::after_local_edit();
    }
    id
}
