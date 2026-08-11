//! T-180.4 — pure squad leader→member map-line geometry (no wgpu).
//!
//! Segment contract: for each squad of size N (including leader) emit **N−1** LineList
//! segments from `leaderSlotId` to every other member. Peer↔peer edges are forbidden.
//! Stroke RGBA comes from [`crate::slots_gpu::side_rgba`] / `SIDE_*` (normalized to f32/255).
//!
//! T-801 — [`pack_squad_link_drag_preview`] offsets endpoints that participate in a live drag so
//! the hairline lane can track the GPU sprite preview without a document write. Commit still
//! rebuilds via [`build_squad_link_segments`] from authored xy.

use std::collections::{HashMap, HashSet};

use crate::slots_gpu::side_rgba;

/// One squad's link inputs for [`build_squad_link_segments`].
#[derive(Clone, Debug)]
pub struct SquadLinkInput {
    pub leader_slot_id: String,
    pub member_slot_ids: Vec<String>,
    /// Faction `key` (`BLUFOR` / `OPFOR` / `INDFOR`) → [`side_rgba`].
    pub side: String,
}

/// LineList verts: `[x0,y0,r,g,b,a, x1,y1,r,g,b,a, …]` (2 verts/segment, 6 f32/vert).
///
/// - Invalid / missing leader id or leader xy → 0 segments for that squad (no panic).
/// - Missing member xy → skip that segment only.
/// - Colors = `side_rgba(side)` as f32/255 on both endpoints.
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

/// T-801 — live drag preview pack for the squad-tether hairline lane.
///
/// Same vert layout as [`build_squad_link_segments`]. Dragged slot ids receive `(dx, dy)` at
/// lookup time (both ends of a segment when a multi-select includes both). Only **affected**
/// squads (leader or any member in `drag_ids`) re-resolve with the offset; unaffected squads
/// emit from authored xy — no whole-map xy clone, no offset work on idle tethers.
///
/// Empty `drag_ids` (or a zero delta) is the identity restore [`build_squad_link_segments`] would
/// produce — the cancel / clear-preview path.
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
mod tests {
    use super::*;
    use crate::slots_gpu::SIDE_OPFOR_RGBA;

    fn xy(pairs: &[(&str, f32, f32)]) -> HashMap<String, (f32, f32)> {
        pairs
            .iter()
            .map(|(id, x, y)| ((*id).to_string(), (*x, *y)))
            .collect()
    }

    fn segment_count(verts: &[f32]) -> usize {
        verts.len() / 12
    }

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    /// D1 — five members + valid leader ⇒ 4 segments; verts.len() == 48.
    #[test]
    fn squad_link_segment_count() {
        let squads = [SquadLinkInput {
            leader_slot_id: "L".into(),
            member_slot_ids: vec!["L".into(), "a".into(), "b".into(), "c".into(), "d".into()],
            side: "BLUFOR".into(),
        }];
        let map = xy(&[
            ("L", 0.0, 0.0),
            ("a", 1.0, 0.0),
            ("b", 2.0, 0.0),
            ("c", 3.0, 0.0),
            ("d", 4.0, 0.0),
        ]);
        let verts = build_squad_link_segments(&squads, &map);
        assert_eq!(segment_count(&verts), 4);
        assert_eq!(verts.len(), 48);
    }

    /// D2 — no segment with both endpoints non-leader.
    #[test]
    fn squad_link_no_peer_segments() {
        let squads = [SquadLinkInput {
            leader_slot_id: "L".into(),
            member_slot_ids: vec!["L".into(), "a".into(), "b".into(), "c".into()],
            side: "BLUFOR".into(),
        }];
        let map = xy(&[
            ("L", 0.0, 0.0),
            ("a", 10.0, 0.0),
            ("b", 0.0, 10.0),
            ("c", 10.0, 10.0),
        ]);
        let verts = build_squad_link_segments(&squads, &map);
        let leader = (0.0_f32, 0.0_f32);
        for seg in verts.chunks_exact(12) {
            let p0 = (seg[0], seg[1]);
            let p1 = (seg[6], seg[7]);
            let touches_leader = (p0 == leader) || (p1 == leader);
            assert!(touches_leader, "peer segment forbidden: ({p0:?})–({p1:?})");
        }
    }

    /// D3 — solo squad ⇒ 0 segments.
    #[test]
    fn squad_link_solo_zero_segments() {
        let squads = [SquadLinkInput {
            leader_slot_id: "solo".into(),
            member_slot_ids: vec!["solo".into()],
            side: "BLUFOR".into(),
        }];
        let map = xy(&[("solo", 1.0, 2.0)]);
        let verts = build_squad_link_segments(&squads, &map);
        assert!(verts.is_empty());
        assert_eq!(segment_count(&verts), 0);
    }

    /// D4 — OPFOR stroke == SIDE_OPFOR_RGBA as f32/255.
    #[test]
    fn squad_link_side_color() {
        let squads = [SquadLinkInput {
            leader_slot_id: "L".into(),
            member_slot_ids: vec!["L".into(), "m".into()],
            side: "OPFOR".into(),
        }];
        let map = xy(&[("L", 0.0, 0.0), ("m", 5.0, 5.0)]);
        let verts = build_squad_link_segments(&squads, &map);
        assert_eq!(segment_count(&verts), 1);
        let expect = rgba_f32(SIDE_OPFOR_RGBA);
        assert_eq!(&verts[2..6], &expect);
        assert_eq!(&verts[8..12], &expect);
    }

    /// D5 — two squads sizes 3+2 ⇒ 2+1 = 3 segments.
    #[test]
    fn squad_link_multi_squad() {
        let squads = [
            SquadLinkInput {
                leader_slot_id: "L1".into(),
                member_slot_ids: vec!["L1".into(), "a".into(), "b".into()],
                side: "BLUFOR".into(),
            },
            SquadLinkInput {
                leader_slot_id: "L2".into(),
                member_slot_ids: vec!["L2".into(), "c".into()],
                side: "INDFOR".into(),
            },
        ];
        let map = xy(&[
            ("L1", 0.0, 0.0),
            ("a", 1.0, 0.0),
            ("b", 2.0, 0.0),
            ("L2", 10.0, 10.0),
            ("c", 11.0, 10.0),
        ]);
        let verts = build_squad_link_segments(&squads, &map);
        assert_eq!(segment_count(&verts), 3);
    }

    /// D6 — missing member xy ⇒ skip that segment only.
    #[test]
    fn squad_link_skips_missing_xy() {
        let squads = [SquadLinkInput {
            leader_slot_id: "L".into(),
            member_slot_ids: vec!["L".into(), "a".into(), "missing".into(), "b".into()],
            side: "BLUFOR".into(),
        }];
        let map = xy(&[("L", 0.0, 0.0), ("a", 1.0, 0.0), ("b", 2.0, 0.0)]);
        let verts = build_squad_link_segments(&squads, &map);
        assert_eq!(segment_count(&verts), 2);
        assert_eq!(verts.len(), 24);
    }

    /// T-801 — single-id drag: the dragged endpoint tracks `(dx, dy)`; the other end stays put.
    #[test]
    fn squad_link_drag_preview_offsets_single_dragged_endpoint() {
        let squads = [SquadLinkInput {
            leader_slot_id: "L".into(),
            member_slot_ids: vec!["L".into(), "a".into()],
            side: "BLUFOR".into(),
        }];
        let map = xy(&[("L", 10.0, 20.0), ("a", 30.0, 40.0)]);
        let verts = pack_squad_link_drag_preview(&squads, &map, &ids(&["a"]), 7.5, -3.25);
        assert_eq!(segment_count(&verts), 1);
        assert_eq!(verts[0], 10.0);
        assert_eq!(verts[1], 20.0);
        assert_eq!(verts[6], 37.5);
        assert_eq!(verts[7], 36.75);
    }

    /// T-801 — multi-select drag moves **both** ends of a shared tether by the same delta.
    #[test]
    fn squad_link_drag_preview_offsets_both_ends_when_multi_selected() {
        let squads = [SquadLinkInput {
            leader_slot_id: "L".into(),
            member_slot_ids: vec!["L".into(), "a".into()],
            side: "BLUFOR".into(),
        }];
        let map = xy(&[("L", 10.0, 20.0), ("a", 30.0, 40.0)]);
        let verts = pack_squad_link_drag_preview(&squads, &map, &ids(&["L", "a"]), 7.5, -3.25);
        assert_eq!(segment_count(&verts), 1);
        assert_eq!(verts[0], 17.5);
        assert_eq!(verts[1], 16.75);
        assert_eq!(verts[6], 37.5);
        assert_eq!(verts[7], 36.75);
    }

    /// T-801 — only affected squads take the offset; an idle squad's verts stay authored.
    #[test]
    fn squad_link_drag_preview_repacks_only_affected_squads() {
        let squads = [
            SquadLinkInput {
                leader_slot_id: "L1".into(),
                member_slot_ids: vec!["L1".into(), "a".into()],
                side: "BLUFOR".into(),
            },
            SquadLinkInput {
                leader_slot_id: "L2".into(),
                member_slot_ids: vec!["L2".into(), "c".into()],
                side: "OPFOR".into(),
            },
        ];
        let map = xy(&[
            ("L1", 0.0, 0.0),
            ("a", 1.0, 0.0),
            ("L2", 10.0, 10.0),
            ("c", 11.0, 10.0),
        ]);
        let authored = build_squad_link_segments(&squads, &map);
        let preview = pack_squad_link_drag_preview(&squads, &map, &ids(&["a"]), 5.0, 0.0);
        assert_eq!(segment_count(&preview), 2);
        // First segment (L1→a): member endpoint moved.
        assert_eq!(&preview[0..2], &[0.0, 0.0]);
        assert_eq!(&preview[6..8], &[6.0, 0.0]);
        // Second segment (L2→c): byte-identical to the authored pack (unaffected).
        assert_eq!(&preview[12..], &authored[12..]);
    }

    /// T-801 — empty drag / zero delta is the identity restore the cancel path uses.
    #[test]
    fn squad_link_drag_preview_identity_on_clear() {
        let squads = [SquadLinkInput {
            leader_slot_id: "L".into(),
            member_slot_ids: vec!["L".into(), "a".into()],
            side: "BLUFOR".into(),
        }];
        let map = xy(&[("L", 1.0, 2.0), ("a", 3.0, 4.0)]);
        let authored = build_squad_link_segments(&squads, &map);
        assert_eq!(
            pack_squad_link_drag_preview(&squads, &map, &[], 9.0, 9.0),
            authored
        );
        assert_eq!(
            pack_squad_link_drag_preview(&squads, &map, &ids(&["a"]), 0.0, 0.0),
            authored
        );
    }
}
