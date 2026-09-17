//! Role: the multi-click zone/trigger draw — arm a kind and shape, collect the ring or the
//! centre-and-radius, and commit the finished geometry as one authored row.
//! Position: `editor/bridge/host_state/armed_placement` in the frontend editor shell.
//! Signals & state: the draft rides the armed value on the installed editor context, and every
//! vertex nudges the reactive document tick so the dock's live hint re-reads.
//! Invariants: the draft lives on the armed value rather than in a reading of its own, because "is
//! a draw in flight" is what routes a map release to the draw instead of the select machine, and
//! re-deriving that from a second source is how the two get out of step. Nothing is written to the
//! document until the shape closes, so an abandoned draw leaves no row and no undo step. The kind
//! is taken from the schema's closed enum by whoever arms the draw — never typed.

use super::Pending;
use crate::v2::apps::editor::bridge::host_state::editor_context::{
    bump_doc_tick, ZoneDraft, EDITOR_CONTEXT,
};
use crate::v2::apps::editor::shell::eden_chrome::{
    circle_from_clicks, polygon_flat, polygon_is_committable, zone_types, ZoneShape,
};
use crate::v2::apps::editor::ui::inspector::zones_panel::DrawTarget;
use website_map_engine::data::store::operations::entity::ZoneDrawStep;
use website_map_engine::editing::hosted_commands as engine_ops;

/// Is a zone draw in flight?
#[must_use]
pub fn zone_draw_armed() -> bool {
    EDITOR_CONTEXT.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|ctx| matches!(*ctx.pending.borrow(), Some(Pending::Zone(_))))
    })
}

/// The in-flight draw, for the dock's live hint ("click the rim", "2 vertices — one more to close").
#[must_use]
pub fn zone_draft() -> Option<ZoneDraft> {
    EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let p = ctx.pending.borrow();
        match &*p {
            Some(Pending::Zone(d)) => Some(d.clone()),
            _ => None,
        }
    })
}

/// Arm a draw that MINTS a new row. Refuses a kind the collection's schema enum does not carry.
pub fn begin_zone_draw(kind: &str, shape: ZoneShape, collection: DrawTarget) -> bool {
    let valid = website_map_engine::data::store::operations::entity::zone_draft_kind_is_valid(
        kind, collection, zone_types,
    );
    if !valid {
        return false;
    }
    EDITOR_CONTEXT.with(|c| {
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

/// Arm a draw that RESHAPES an existing row, keeping the kind that row already carries.
pub fn begin_zone_reshape(row_id: &str, shape: ZoneShape, collection: DrawTarget) -> bool {
    let kind = match collection {
        DrawTarget::Zone => engine_ops::zone_rows()
            .into_iter()
            .find(|r| r.id == row_id)
            .map(|r| r.kind),

        DrawTarget::Trigger => engine_ops::trigger_rows()
            .into_iter()
            .find(|r| r.id == row_id)
            .map(|r| r.activation),
    };
    let Some(kind) = kind else {
        return false;
    };
    EDITOR_CONTEXT.with(|c| {
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

/// Abandon the in-flight draw without writing anything — the explicit counterpart a zone draw needs
/// because it deliberately survives the ordinary disarm. `false` when nothing was in flight.
pub fn cancel_zone_draw() -> bool {
    let cleared = EDITOR_CONTEXT.with(|c| {
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

/// Drop the last polygon vertex (the undo-vertex control). Returns the remaining count.
pub fn zone_draw_pop_vertex() -> usize {
    EDITOR_CONTEXT.with(|c| {
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

/// One canvas release while a zone draw is armed: take a vertex, or close a circle and commit it.
pub(crate) fn advance_zone_draw(x: f64, z: f64) -> bool {
    let step = EDITOR_CONTEXT.with(|c| {
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
            (DrawTarget::Zone, Some(id)) => {
                engine_ops::commit_document_edit(|core| core.set_zone_circle(&id, cx, cz, r))
            }
            (DrawTarget::Zone, None) => {
                engine_ops::add_authored_row(DrawTarget::Zone, |core, id| {
                    core.add_circle_zone(id, &kind, cx, cz, r);
                })
                .is_some()
            }
            (DrawTarget::Trigger, Some(id)) => {
                engine_ops::commit_document_edit(|core| core.set_trigger_circle(&id, cx, cz, r))
            }
            (DrawTarget::Trigger, None) => {
                engine_ops::add_authored_row(DrawTarget::Trigger, |core, id| {
                    core.add_circle_trigger(id, &kind, cx, cz, r);
                })
                .is_some()
            }
        },

        Some(ZoneDrawStep::Drawing) | None => {
            bump_doc_tick();
            zone_draw_armed()
        }
    }
}

/// Close the in-flight ring. Refuses below three vertices — `$defs/polygon` is `minItems: 3`, and a
/// two-vertex ring is a document the schema rejects.
pub fn close_zone_polygon() -> bool {
    let taken = EDITOR_CONTEXT.with(|c| {
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
        (DrawTarget::Zone, Some(id)) => {
            engine_ops::commit_document_edit(|core| core.set_zone_polygon(&id, &flat))
        }
        (DrawTarget::Zone, None) => engine_ops::add_authored_row(DrawTarget::Zone, |core, id| {
            core.add_polygon_zone(id, &kind, &flat);
        })
        .is_some(),
        (DrawTarget::Trigger, Some(id)) => {
            engine_ops::commit_document_edit(|core| core.set_trigger_polygon(&id, &flat))
        }
        (DrawTarget::Trigger, None) => {
            engine_ops::add_authored_row(DrawTarget::Trigger, |core, id| {
                core.add_polygon_trigger(id, &kind, &flat);
            })
            .is_some()
        }
    }
}
