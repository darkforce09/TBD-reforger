//! Role: zones.
//! Position: `editor/state/operations/entity` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

use super::*;

/// Every authored zone, in `zones_json` map order, for the dock list.
#[must_use]
pub fn zone_rows() -> Vec<ZoneRow> {
    OPS_CTX
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            website_map_engine::data::store::operations::entity::zone_rows(core)
        })
        .unwrap_or_default()
}

/// Edit zone using the supplied domain data.
pub(in crate::editor::state::operations) fn edit_zone(f: impl FnOnce(&MissionDocCore)) -> bool {
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

/// Attributes — schema `zone.type`. Refuses a value outside the schema enum, for the same reason [`begin_zone_draw`] does.
pub fn set_zone_kind(id: &str, kind: &str) -> bool {
    if !zone_types().iter().any(|t| t == kind) {
        return false;
    }
    edit_zone(|core| core.set_zone_type(id, kind))
}

/// Attributes — schema `zone.label`. An EMPTY string and `None` are different authored states the schema allows on purpose: `Some("")` writes an empty label (which the mod reads as "use the PrettyZoneTitle fallback" and is a committed golden), `None` removes the key. The panel's Clear control sends `None`; typing and clearing the box sends `Some("")`.
pub fn set_zone_label(id: &str, label: Option<String>) -> bool {
    edit_zone(|core| core.set_zone_label(id, label.as_deref()))
}

/// Attributes — schema `zone.faction` (a `factionKey` slug). `None` makes the zone faction-neutral.
pub fn set_zone_faction(id: &str, faction: Option<String>) -> bool {
    edit_zone(|core| core.set_zone_faction(id, faction.as_deref()))
}

/// Attributes — set or clear ONE `rules` key, read-modify-write over the OPAQUE object.
pub fn set_zone_rule(id: &str, key: &str, value: Option<serde_json::Value>) -> bool {
    let next = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        website_map_engine::data::store::operations::entity::set_zone_rule(core, id, key, value)
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

/// Every mission wants a play area, and today the only way to get one is to draw a 12.8 km ring by hand through [`begin_zone_draw`] — vertex by vertex, on a map where a pixel is metres. This is that ring, authored from the map itself.
pub fn add_whole_terrain_zone() -> Option<String> {
    use crate::editor::panels::zones_panel;

    let (terrain, bounds) = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        Some((terrain_key_of(core), terrain_bounds_of(core)))
    })?;
    let ring = zones_panel::terrain_rect_ring(&terrain, bounds)?;

    let kind = zones_panel::whole_terrain_zone_type()?;
    write_row_returning_id(DrawTarget::Zone, |core, id| {
        core.add_polygon_zone_labelled(
            id,
            &kind,
            &ring,
            Some(zones_panel::WHOLE_TERRAIN_ZONE_LABEL),
        );
    })
}
