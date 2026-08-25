use super::{
    connection_lane_verts, connection_segments, pick_connection, ConnSegment, CONN_LINE_RGBA,
    CONN_LINE_SELECTED_RGBA,
};
use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
use std::collections::HashMap;

fn positions() -> HashMap<String, (f64, f64)> {
    [
        ("a".to_string(), (0.0, 0.0)),
        ("b".to_string(), (100.0, 0.0)),
        ("c".to_string(), (0.0, 100.0)),
    ]
    .into_iter()
    .collect()
}

fn rows(pairs: &[(&str, &str, &str)]) -> String {
    let arr: Vec<serde_json::Value> = pairs
        .iter()
        .map(|(id, from, to)| serde_json::json!({"id": id, "kind": "sync", "from": from, "to": to}))
        .collect();
    serde_json::Value::Array(arr).to_string()
}

/// A placed edge becomes one segment between its endpoints, in listing order.
#[test]
fn placed_edges_become_segments_in_listing_order() {
    let segs = connection_segments(&rows(&[("k2", "a", "b"), ("k1", "b", "c")]), &positions());
    assert_eq!(
        segs,
        vec![
            ConnSegment {
                id: "k2".to_string(),
                ax: 0.0,
                ay: 0.0,
                bx: 100.0,
                by: 0.0
            },
            ConnSegment {
                id: "k1".to_string(),
                ax: 100.0,
                ay: 0.0,
                bx: 0.0,
                by: 100.0
            },
        ]
    );
}

/// A DANGLING edge draws nothing — never a line to the origin. `CONN-DANGLING` is a panel
/// finding; a wrong line would be a second report that also lies about where the entity is.
/// A self-link (`CONN-SELF`) is dropped too: a zero-length segment is not a clickable artifact.
#[test]
fn dangling_and_self_edges_draw_nothing() {
    let segs = connection_segments(
        &rows(&[
            ("dangle-from", "ghost", "b"),
            ("dangle-to", "a", "ghost"),
            ("self", "a", "a"),
            ("good", "a", "b"),
        ]),
        &positions(),
    );
    assert_eq!(segs.len(), 1, "only the resolvable, non-self edge draws");
    assert_eq!(segs[0].id, "good");
    // Malformed input is inert, not a panic.
    assert!(connection_segments("not json", &positions()).is_empty());
    assert!(connection_segments("{}", &positions()).is_empty());
}

/// 6 floats per vertex, 2 vertices per segment, and the selected edge — and ONLY it — is tinted.
#[test]
fn lane_verts_tint_exactly_the_selected_edge() {
    let segs = connection_segments(&rows(&[("k1", "a", "b"), ("k2", "b", "c")]), &positions());
    let v = connection_lane_verts(&segs, Some("k2"));
    assert_eq!(v.len(), 2 * 12, "6 floats/vert, 2 verts/segment");
    assert_eq!((v[0], v[1]), (0.0, 0.0));
    assert_eq!((v[6], v[7]), (100.0, 0.0));
    assert_eq!(v[2..6], CONN_LINE_RGBA, "k1 is not selected");
    assert_eq!(v[8..12], CONN_LINE_RGBA, "both k1 verts share its colour");
    assert_eq!(v[14..18], CONN_LINE_SELECTED_RGBA, "k2 is selected");
    assert_eq!(v[20..24], CONN_LINE_SELECTED_RGBA);
    // No selection ⇒ nothing tinted; an id that names no edge ⇒ nothing tinted (a stale
    // selection whose edge was undone away is inert, never a mis-tint of some other edge).
    for sel in [None, Some("gone")] {
        let plain = connection_lane_verts(&segs, sel);
        assert!(plain.chunks_exact(6).all(|c| c[2..6] == CONN_LINE_RGBA[..]));
    }
    assert!(connection_lane_verts(&[], None).is_empty());
}

/// The hit test is point-to-SEGMENT: near the span hits, past the end does not (the
/// infinite-line form would select a short edge from far off its extension), and the NEAREST
/// edge wins when two are in range.
#[test]
fn pick_is_nearest_segment_within_tolerance() {
    let segs = connection_segments(&rows(&[("k1", "a", "b"), ("k2", "a", "c")]), &positions());
    // 3 m off the middle of the a→b edge, tolerance 5 m.
    assert_eq!(
        pick_connection(&segs, 50.0, 3.0, 5.0).as_deref(),
        Some("k1")
    );
    // Same perpendicular offset, outside tolerance.
    assert_eq!(pick_connection(&segs, 50.0, 9.0, 5.0), None);
    // On the a→b LINE but 40 m past its `b` end — the extension is not the edge.
    assert_eq!(pick_connection(&segs, 140.0, 0.0, 5.0), None);
    // Near the shared corner both edges are in range; the closer one wins.
    assert_eq!(
        pick_connection(&segs, 2.0, 9.0, 20.0).as_deref(),
        Some("k2")
    );
    assert_eq!(
        pick_connection(&segs, 9.0, 2.0, 20.0).as_deref(),
        Some("k1")
    );
    assert_eq!(pick_connection(&[], 0.0, 0.0, 100.0), None);
}

fn page() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

/// **The lane is fed from the DOCUMENT.** The mount body must bind `connections_bind` from
/// `live_connection_segments` (which reads `MissionDocCore`) packed by `connection_lane_verts`,
/// re-reading on `doc_tick` — the one channel `after_doc_change` (edit / undo / redo),
/// `refresh_hud` (seed / hydrate) and `rebind_engine_from_doc` (IDB restore + engine mount) all
/// bump. Deleting the `doc_tick` read is exactly the T-069 / T-672 stale-lane defect, and it
/// turns this pin RED.
#[test]
fn lane_is_bound_from_the_document_on_every_doc_tick() {
    let mount = only_body(&page(), "canvas_ref.on_load(").to_string();
    let bind = ["connections", "_bind("].concat();
    let build = ["live_connection", "_segments"].concat();
    let pack = ["connection_lane", "_verts("].concat();
    assert!(
        mount.contains(&bind),
        "T-780: the mount must call connections_bind — without it the lane draws nothing"
    );
    assert!(
        mount.contains(&build),
        "T-780: the lane must be built from live_connection_segments (the document read)"
    );
    assert!(
        mount.contains(&pack),
        "T-780: the lane must be packed by connection_lane_verts"
    );
    let tick = ["doc_tick", ".get()"].concat();
    assert!(
        mount.contains(&tick),
        "T-780: the feed must re-read doc_tick, or the lane goes stale on undo/redo/restore"
    );
}

/// **Delete on the map routes through the PANEL's verb.** The keydown must call
/// `editor_ops::delete_connection` — the same function `ConnectionsPanelOverlay`'s per-row
/// button calls — off the map selection, and the slot `delete_selection` must survive as the
/// other branch of the SAME arm, so one keypress can never delete both an edge and a slot.
///
/// [wave 142 F-1] The ORDER is now three deep, not two: the arm must READ the armed id, RESOLVE
/// it against the live document, and only then delete. The middle step is the finding — without
/// it an id the document no longer holds (undo, a panel-side delete, the T-672 endpoint cascade)
/// was handed straight to the verb, which reported success over a write that never happened.
#[test]
fn map_delete_calls_the_panels_delete_connection() {
    let keys = only_body(&page(), "let onkeydown =").to_string();
    let verb = ["editor_ops", "::", "delete_connection("].concat();
    let sel = ["selected_", "connection.try_get_untracked()"].concat();
    assert!(
        keys.contains(&verb),
        "T-780: Delete must route through editor_ops::delete_connection (the panel's own verb)"
    );
    let at_verb = keys.find(&verb).expect("asserted above");
    let at_sel = keys
        .find(&sel)
        .expect("T-780: the Delete branch must read the map connection selection");
    assert!(
        at_sel < at_verb,
        "T-780: the map selection must be read before the delete, not after it"
    );
    let resolve = ["editor_ops", "::", "connection_exists("].concat();
    let at_resolve = keys.find(&resolve).expect(
        "wave 142 F-1: the Delete branch must resolve the armed id against the live document \
         before firing — a stale id must fall through to the entity delete, not be handed to a \
         verb that can only answer false",
    );
    assert!(
        at_sel < at_resolve && at_resolve < at_verb,
        "wave 142 F-1: read the selection, RESOLVE it, then delete — in that order"
    );
    let fallthrough = ["editor_ops", "::", "delete_selection()"].concat();
    assert!(
        keys[at_verb..].contains(&fallthrough),
        "T-780: the slot Delete must survive as the other branch — this is an addition to the \
         Delete arm, not a replacement of it"
    );
}

/// **The verb's `bool` means the connection was there and is now gone** [wave 142 F-2].
///
/// It used to mean "the document holds at least one connection", because the guard was a COUNT:
/// `delete_connection` returned `true` for an id the graph did not hold whenever any other edge
/// existed, and `after_local_edit` then dirtied a mission that never changed. That is the T-779
/// class — a verb inventing an acknowledgement for a write that did not land — and this pin is
/// the shape of it: no count in the guard, an id-presence check, taken BEFORE the write, because
/// the core's `remove_connection` returns unit and cannot report what it removed afterwards.
#[test]
fn delete_connection_answers_the_document_not_a_count() {
    let ops = live_code(include_str!("../state/operations/entity.rs"));
    let verb = only_body(&ops, "pub fn delete_connection(");
    let count = ["connection", "_count("].concat();
    assert!(
        !verb.contains(&count),
        "wave 142 F-2: a COUNT cannot say whether THIS id was there; got:\n{verb}"
    );
    let gate = ["connection_id", "_in_doc("].concat();
    let at_gate = verb.find(&gate).expect(
        "wave 142 F-2: delete_connection must gate on the id being PRESENT in the document",
    );
    let remove = ["core.", "remove_connection("].concat();
    let at_remove = verb
        .find(&remove)
        .expect("the verb must still be the one place the core mutator is reached");
    assert!(
        at_gate < at_remove,
        "wave 142 F-2: the presence check must be taken before the write, not inferred after it"
    );
    // ONE question, ONE implementation: the map arm's fall-through gate asks the same thing, so
    // the branch the keydown takes and the write that follows it cannot disagree.
    assert!(
        only_body(&ops, "pub fn connection_exists(").contains(&gate),
        "wave 142 F-1: connection_exists must ask the same question the verb gates on"
    );
}

/// **The two selections cannot coexist — by construction, not by convention** [wave 142 F-1].
///
/// T-780 claimed exclusivity from the map pick's ordering, which says nothing about the Outliner
/// row, the marquee, the click-to-select router or a place: through any of those an edge and a
/// slot were both selected, and Delete removed the edge while the operator watched a highlighted
/// slot. The construction that makes it true is this: every entity-selection write in the editor
/// reaches the UI through ONE mirror, and the reconcile lives inside that mirror. So the pin is
/// not "someone remembered to clear it" — it is that `selected_ids` has exactly one writer.
#[test]
fn an_edge_selection_and_an_entity_selection_cannot_coexist() {
    // T-934.7 — the ops module was split; concatenate every submodule so the file-wide
    // absence / uniqueness assertions keep their whole-module meaning.
    let ops = live_code(
        &[
            include_str!("../state/operations/attrs.rs"),
            include_str!("../state/operations/cargo.rs"),
            include_str!("../state/operations/compositions.rs"),
            include_str!("../state/operations/context.rs"),
            include_str!("../state/operations/entity.rs"),
            include_str!("../state/operations/transform.rs"),
        ]
        .concat(),
    );
    let mirror = ["mirror_", "selection(ctx)"].concat();
    let reconcile = ["reconcile_connection", "_selection(ctx)"].concat();
    assert!(
        only_body(&ops, "fn mirror_selection(").contains(&reconcile),
        "wave 142 F-1: the mirror must reconcile the map's edge selection"
    );
    for marker in ["pub fn refresh_docks(", "pub fn refresh_selection_mirrors("] {
        assert!(
            only_body(&ops, marker).contains(&mirror),
            "wave 142 F-1: {marker} must push the selection through the shared mirror"
        );
    }
    let push = ["selected_ids", ".set("].concat();
    assert_eq!(
        ops.matches(&push).count(),
        1,
        "wave 142 F-1: exactly ONE writer of selected_ids — a second would be a route that puts \
         an entity selection on screen without the reconcile, which is the finding itself"
    );
    // The reconcile can only reach the map's selection because the page hands the signal over.
    let handoff = ["editor_ops", "::", "set_connection_selection_signal("].concat();
    assert!(
        only_body(&page(), "canvas_ref.on_load(").contains(&handoff),
        "wave 142 F-1: on_load must register the connection selection with editor_ops"
    );
    // And the reconcile drops the id for BOTH reasons it can stop naming what Delete removes:
    // a live entity selection, and an id the document no longer holds.
    let body = only_body(&ops, "fn reconcile_connection_selection(");
    let in_doc = ["connection_id", "_in_doc("].concat();
    assert!(
        body.contains("selection.borrow().is_empty()") && body.contains(&in_doc),
        "wave 142 F-1: the reconcile must test the entity selection AND the document; got:\n\
         {body}"
    );
}

/// **No second deletion path, and no kind list.** UNSCOPED over the whole live page (never
/// scoped — a scoped negative is green by construction): this file must reach the core's
/// `remove_connection` only through `editor_ops`, and the map path must not re-derive which
/// connections are actionable from a hardcoded vocabulary (the wave-129 rule — a kind list is a
/// second answer to a question that already has one).
#[test]
fn no_second_delete_path_and_no_hardcoded_kind_list() {
    let code = page();
    let core_delete = ["core.", "remove_connection("].concat();
    assert!(
        !code.contains(&core_delete),
        "T-780: a map-side core.remove_connection would be a second CONN-DEL-001 vocabulary"
    );
    let kinds = ["ConnKind", "::parse"].concat();
    assert!(
        !code.contains(&kinds),
        "T-780: the map connection path must not gate on a hardcoded kind list"
    );
}

/// **THE HISTORY CHAIN, checked rather than claimed.** The acceptance for this lane is: draw an
/// edge, then UNDO, REDO and RESTORE FROM IDB, and the line survives each one. The feed above
/// re-binds on `doc_tick`; this pin proves every one of those paths actually reaches `doc_tick`,
/// so "it survives history" is a checked chain and not a hope:
///
/// ```text
///   undo / redo / a committed edit  → after_doc_change      ─┐
///   the mount seed / server hydrate → refresh_hud           ─┼→ refresh_signals
///   the IDB restore swap + the      → rebind_engine_from_doc ┘        │
///   engine-mount handshake                                           ▼
///                                          editor_ops::refresh_docks → doc_tick.set(n + 1)
/// ```
///
/// This is the T-069 / T-672 defect stated as a test: break ANY link and the lane keeps drawing
/// whatever the document held before the undo. `mission_history.rs` and `editor_ops.rs` are read
/// here, never written — the chain already existed; what is new is that something checks it.
#[test]
fn every_history_path_reaches_the_doc_tick_the_lane_binds_on() {
    let hist = live_code(include_str!("../state/history.rs"));
    let ops = live_code(include_str!("../state/operations/context.rs"));
    let signals = ["refresh_", "signals("].concat();
    let docks = ["editor_ops", "::", "refresh_docks()"].concat();
    let tail = ["after_doc", "_change(ctx)"].concat();

    for (name, marker) in [("undo", "pub fn undo"), ("redo", "pub fn redo")] {
        assert!(
            only_body(&hist, marker).contains(&tail),
            "T-780: {name} must run after_doc_change, or the lane never re-reads the document"
        );
    }
    for (name, marker) in [
        ("after_doc_change (edit/undo/redo)", "fn after_doc_change"),
        ("refresh_hud (mount seed / hydrate)", "pub fn refresh_hud"),
        (
            "rebind_engine_from_doc (IDB restore)",
            "pub fn rebind_engine_from_doc",
        ),
    ] {
        assert!(
            only_body(&hist, marker).contains(&signals),
            "T-780: {name} must reach refresh_signals — it is the only route to doc_tick"
        );
    }
    assert!(
        only_body(&hist, "fn refresh_signals").contains(&docks),
        "T-780: refresh_signals must call editor_ops::refresh_docks"
    );
    let bump = only_body(&ops, "pub fn refresh_docks");
    assert!(
        bump.contains("doc_tick") && bump.contains(".set("),
        "T-780: refresh_docks must bump doc_tick — the one channel the lane binds on"
    );
}

/// Hollow canary: the two pins above are load-bearing, not decorative — stripping either needle
/// from an in-memory copy of the real source breaks the assertion that found it.
#[test]
fn connection_pins_are_load_bearing() {
    let mount = only_body(&page(), "canvas_ref.on_load(").to_string();
    let bind = ["connections", "_bind("].concat();
    assert!(
        mount.contains(&bind),
        "canary: the real mount binds the lane"
    );
    assert!(
        !mount.replacen(&bind, "/* hollow */", 1).contains(&bind),
        "fired rule: deleting connections_bind must break the T-780 feed pin"
    );
    let keys = only_body(&page(), "let onkeydown =").to_string();
    let verb = ["editor_ops", "::", "delete_connection("].concat();
    assert!(
        keys.contains(&verb),
        "canary: the real keydown deletes edges"
    );
    assert!(
        !keys.replacen(&verb, "/* hollow */", 1).contains(&verb),
        "fired rule: deleting the delete_connection call must break the T-780 delete pin"
    );
    // [wave 142] The three new needles, same treatment: strip each from an in-memory copy of the
    // real source and the assertion that found it has nothing left to find.
    let resolve = ["editor_ops", "::", "connection_exists("].concat();
    assert!(keys.contains(&resolve), "canary: the real arm resolves");
    assert!(
        !keys
            .replacen(&resolve, "/* hollow */", 1)
            .contains(&resolve),
        "fired rule: dropping the document resolve must break the F-1 arm pin"
    );
    // T-934.7 — the ops module was split; concatenate every submodule so the file-wide
    // absence / uniqueness assertions keep their whole-module meaning.
    let ops = live_code(
        &[
            include_str!("../state/operations/attrs.rs"),
            include_str!("../state/operations/cargo.rs"),
            include_str!("../state/operations/compositions.rs"),
            include_str!("../state/operations/context.rs"),
            include_str!("../state/operations/entity.rs"),
            include_str!("../state/operations/transform.rs"),
        ]
        .concat(),
    );
    let gate = ["connection_id", "_in_doc("].concat();
    let verb_body = only_body(&ops, "pub fn delete_connection(");
    assert!(verb_body.contains(&gate), "canary: the real verb gates");
    assert!(
        !verb_body.replacen(&gate, "/* hollow */", 1).contains(&gate),
        "fired rule: dropping the id-presence gate must break the F-2 verb pin"
    );
    let reconcile = ["reconcile_connection", "_selection(ctx)"].concat();
    let mirror_body = only_body(&ops, "fn mirror_selection(");
    assert!(
        mirror_body.contains(&reconcile),
        "canary: the real mirror reconciles"
    );
    assert!(
        !mirror_body
            .replacen(&reconcile, "/* hollow */", 1)
            .contains(&reconcile),
        "fired rule: dropping the reconcile must break the F-1 exclusivity pin"
    );
}
