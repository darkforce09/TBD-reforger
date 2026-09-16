//! Role: squad links tests.
//! Position: `symbology/links/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::symbology::links::squad_links::*;
use crate::symbology::roles::classify::SIDE_OPFOR_RGBA;

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

    assert_eq!(&preview[0..2], &[0.0, 0.0]);
    assert_eq!(&preview[6..8], &[6.0, 0.0]);

    assert_eq!(&preview[12..], &authored[12..]);
}

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
