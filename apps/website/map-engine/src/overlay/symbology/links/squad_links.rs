//! Role: squad links.
//! Position: `overlay/symbology/links` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::collections::{HashMap, HashSet};

use crate::overlay::symbology::roles::classify::side_rgba;

/// One squad's link inputs for [`build_squad_link_segments`].
#[derive(Clone, Debug)]
pub struct SquadLinkInput {
    /// Leader slot id.
    pub leader_slot_id: String,

    /// Member slot ids.
    pub member_slot_ids: Vec<String>,

    /// Faction `key` (`BLUFOR` / `OPFOR` / `INDFOR`) → [`side_rgba`].
    pub side: String,
}

/// LineList verts: `[x0,y0,r,g,b,a, x1,y1,r,g,b,a, …]` (2 verts/segment, 6 f32/vert).
#[must_use]
pub fn build_squad_link_segments(
    squads: &[SquadLinkInput],
    xy_by_slot: &HashMap<String, (f32, f32)>,
) -> Vec<f32> {
    let mut out = Vec::new();
    for sq in squads {
        emit_squad_segments(sq, xy_by_slot, None, 0.0, 0.0, &mut out);
    }
    out
}

/// Same vert layout as [`build_squad_link_segments`]. Dragged slot ids receive `(dx, dy)` at lookup time (both ends of a segment when a multi-select includes both). Only **affected** squads (leader or any member in `drag_ids`) re-resolve with the offset; unaffected squads emit from authored xy — no whole-map xy clone, no offset work on idle tethers.
#[must_use]
pub fn pack_squad_link_drag_preview(
    squads: &[SquadLinkInput],
    xy_by_slot: &HashMap<String, (f32, f32)>,
    drag_ids: &[String],
    dx: f32,
    dy: f32,
) -> Vec<f32> {
    if drag_ids.is_empty() || (dx == 0.0 && dy == 0.0) {
        return build_squad_link_segments(squads, xy_by_slot);
    }
    let dragged: HashSet<&str> = drag_ids.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    for sq in squads {
        if squad_touches_drag(sq, &dragged) {
            emit_squad_segments(sq, xy_by_slot, Some(&dragged), dx, dy, &mut out);
        } else {
            emit_squad_segments(sq, xy_by_slot, None, 0.0, 0.0, &mut out);
        }
    }
    out
}

#[inline]
fn squad_touches_drag(sq: &SquadLinkInput, dragged: &HashSet<&str>) -> bool {
    if dragged.contains(sq.leader_slot_id.as_str()) {
        return true;
    }
    sq.member_slot_ids
        .iter()
        .any(|id| dragged.contains(id.as_str()))
}

fn emit_squad_segments(
    sq: &SquadLinkInput,
    xy_by_slot: &HashMap<String, (f32, f32)>,
    dragged: Option<&HashSet<&str>>,
    dx: f32,
    dy: f32,
    out: &mut Vec<f32>,
) {
    if sq.leader_slot_id.is_empty() {
        return;
    }
    if !sq.member_slot_ids.iter().any(|id| id == &sq.leader_slot_id) {
        return;
    }
    let Some((lx, ly)) = preview_xy(xy_by_slot, &sq.leader_slot_id, dragged, dx, dy) else {
        return;
    };
    let c = rgba_f32(side_rgba(&sq.side));
    for mid in &sq.member_slot_ids {
        if mid == &sq.leader_slot_id {
            continue;
        }
        let Some((mx, my)) = preview_xy(xy_by_slot, mid, dragged, dx, dy) else {
            continue;
        };
        out.push(lx);
        out.push(ly);
        out.extend_from_slice(&c);
        out.push(mx);
        out.push(my);
        out.extend_from_slice(&c);
    }
}

#[inline]
fn preview_xy(
    xy_by_slot: &HashMap<String, (f32, f32)>,
    id: &str,
    dragged: Option<&HashSet<&str>>,
    dx: f32,
    dy: f32,
) -> Option<(f32, f32)> {
    let &(x, y) = xy_by_slot.get(id)?;
    match dragged {
        Some(d) if d.contains(id) => Some((x + dx, y + dy)),
        _ => Some((x, y)),
    }
}

#[inline]
fn rgba_f32(c: [u8; 4]) -> [f32; 4] {
    [
        f32::from(c[0]) / 255.0,
        f32::from(c[1]) / 255.0,
        f32::from(c[2]) / 255.0,
        f32::from(c[3]) / 255.0,
    ]
}

#[cfg(test)]
#[path = "tests/squad_links_tests.rs"]
mod tests;
