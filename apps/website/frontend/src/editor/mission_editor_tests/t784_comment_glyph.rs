use super::{
    comment_lane_xy, comment_points, pick_comment, route_target, RouteTarget, COMMENT_PICK_PX,
};
use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

/// Three notes, deliberately NOT in id order in the JSON text, so a reader that trusted the
/// map's iteration order would produce a different sequence from one that sorts.
fn comments() -> String {
    serde_json::json!({
        "cmt-3": { "title": "South", "position": { "x": 300.0, "z": -30.0 } },
        "cmt-1": { "title": "North", "position": { "x": 100.0, "z": 10.0 } },
        "cmt-2": { "title": "East",  "position": { "x": 105.0, "z": 10.0 } },
    })
    .to_string()
}

fn page() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

/// The T-784 glyph block, scrubbed — sliced from the RAW source at the struct anchor, exactly as
/// [`page`] is. T-934.10 moved the pure belt to `canvas/render_sync.rs`, so the definitions are
/// read from there; the CALL-form pins below still read `mission_editor.rs`, where the wiring is.
fn glyph_block() -> String {
    let anchor = format!("pub(crate) struct Comment{}", "Point");
    let raw = include_str!("../canvas/render_sync.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

/// **What is drawn IS what can be picked — both directions, over the whole set.**
///
/// The lane is `comment_points` packed, so for every glyph the lane draws there is a pick at
/// those very coordinates that returns that glyph's id, and every id a pick can return is a
/// glyph the lane drew. A second parser (which is what `mission_history` held before this
/// ticket) is exactly how those two sets start to differ.
#[test]
fn the_lane_is_the_pick_list_packed() {
    let pts = comment_points(&comments());
    let xy = comment_lane_xy(&comments());
    assert_eq!(pts.len(), 3, "all three notes must reach both surfaces");
    assert_eq!(
        xy.len(),
        pts.len() * 2,
        "T-784: the lane is two floats per picked point — no filtering between them"
    );
    let ids: Vec<&str> = pts.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        ids,
        ["cmt-1", "cmt-2", "cmt-3"],
        "T-784: sorted by id, so the lane's instance order cannot depend on serde_json's map \
         type across undo/redo/restore"
    );
    for (i, p) in pts.iter().enumerate() {
        // DRAWN ⇒ PICKABLE: hit-test at the exact coordinates the lane uploaded.
        #[allow(clippy::cast_possible_truncation)]
        let (lx, ly) = (f64::from(xy[i * 2]), f64::from(xy[i * 2 + 1]));
        assert!(
            (lx - p.x).abs() < 1e-3 && (ly - p.y).abs() < 1e-3,
            "T-784: lane vertex {i} is not the picked point {p:?}"
        );
        assert_eq!(
            pick_comment(&pts, lx, ly, 1.0).as_deref(),
            Some(p.id.as_str()),
            "T-784: a glyph drawn at ({lx}, {ly}) must be findable by a click there"
        );
    }
}

/// A comment's second axis is `z` — TWO HORIZONTALS, no height. Reading `y` would file every
/// note at northing 0 in the lane AND in the pick, which is a glyph drawn on the equator.
#[test]
fn the_second_axis_is_z_not_y() {
    let json = serde_json::json!({
        "cmt-1": { "position": { "x": 12.0, "y": 999.0, "z": 34.0 } }
    })
    .to_string();
    let pts = comment_points(&json);
    assert_eq!(
        (pts[0].x, pts[0].y),
        (12.0, 34.0),
        "T-784: `{{x, z}}`, never `y`"
    );
}

/// Nearest wins; beyond the tolerance nothing is picked (a click on empty map must stay a
/// deselect, not a phantom hit on the closest note in the mission).
#[test]
fn pick_takes_the_nearest_and_refuses_beyond_the_tolerance() {
    let pts = comment_points(&comments());
    // Between cmt-1 (100) and cmt-2 (105), one metre nearer cmt-2.
    assert_eq!(
        pick_comment(&pts, 103.0, 10.0, 20.0).as_deref(),
        Some("cmt-2"),
        "T-784: overlapping glyphs must resolve by distance, not by listing order"
    );
    assert_eq!(pick_comment(&pts, 103.0, 10.0, 1.0), None);
    assert_eq!(pick_comment(&pts, -9000.0, -9000.0, 50.0), None);
    assert_eq!(pick_comment(&[], 0.0, 0.0, 1e9), None);
    assert!(comment_points("not json").is_empty());
    assert!(comment_points("[]").is_empty());
}

/// The tolerance IS the slot pick radius, read back from the core's own declaration. A comment
/// is drawn with the slot ring glyph, so a hit box of a different size would be a picture that
/// lies about where it can be clicked — and `MissionDocCore::PICK_RADIUS_PX` is unreachable
/// natively (the `doc` feature is wasm32-only here), which is why this is a source read.
#[test]
fn comment_pick_px_is_the_slot_pick_radius() {
    let store = include_str!("../../../../../../crates/map-engine-core/src/doc/store.rs");
    let needle = ["PICK_RADIUS", "_PX: f64 = "].concat();
    assert_eq!(
        store.matches(needle.as_str()).count(),
        1,
        "T-784: the slot pick radius must have exactly one declaration to read back"
    );
    let tail = &store[store.find(needle.as_str()).expect("counted") + needle.len()..];
    let value: f64 = tail[..tail.find(';').expect("a const ends in `;`")]
        .trim()
        .parse()
        .expect("the slot pick radius must be a plain f64 literal");
    assert!(
        (COMMENT_PICK_PX - value).abs() < f64::EPSILON,
        "T-784: COMMENT_PICK_PX is {COMMENT_PICK_PX} but the slot pick radius is {value} — a \
         comment wears the slot ring glyph, so its hit box must be the same size as its picture"
    );
}

/// **The resolver learned comments** — which is what lets `subject_id_routes` (the Outliner
/// row's affordance AND the dock-left search hit's) answer honestly, with no kind list on
/// either surface. The existing arms must be untouched: a widening that changed what an
/// already-resolving id resolves to would be a regression dressed as a feature.
#[test]
fn route_target_resolves_a_comment_without_disturbing_the_other_arms() {
    let root = serde_json::json!({
        "vehiclesById": { "v1": { "position": { "x": 7.0, "y": 9.0 } } },
        "entitiesById": { "e1": { "position": { "x": 1.0, "y": 2.0 } } },
        "zonesById": { "z1": { "shape": "circle", "center": { "x": 4.0, "y": 5.0 }, "radius": 3.0 } },
        "commentsById": { "cmt-1": { "position": { "x": 100.0, "z": 10.0 } } },
    });
    let not_slot = |_: &str| false;
    assert_eq!(
        route_target(&root, "cmt-1", &not_slot),
        Some(RouteTarget::Comment { x: 100.0, y: 10.0 }),
        "T-784: a commentsById row must resolve, at `{{x, z}}`"
    );
    assert_eq!(route_target(&root, "cmt-404", &not_slot), None);
    assert_eq!(
        route_target(&root, "v1", &not_slot),
        Some(RouteTarget::Vehicle { x: 7.0, y: 9.0 })
    );
    assert_eq!(
        route_target(&root, "e1", &not_slot),
        Some(RouteTarget::Entity { x: 1.0, y: 2.0 })
    );
    assert_eq!(
        route_target(&root, "s1", &|_| true),
        Some(RouteTarget::Slot)
    );
    // A comment row with no position resolves to NOTHING rather than to the origin — an
    // affordance over a click that would fly the camera to (0,0) is the T-754 defect.
    let broken = serde_json::json!({ "commentsById": { "cmt-2": { "title": "x" } } });
    assert_eq!(route_target(&broken, "cmt-2", &not_slot), None);
}

/// **The lane and the pick are one document read, and `mission_history` no longer holds a
/// second parser.** That file is `#![cfg(target_arch = "wasm32")]` end to end, so this has to
/// be an `include_str!` pin — the same reason the T-748 feed pin in `map-engine-render` is one.
#[test]
fn mission_history_packs_the_lane_through_this_module() {
    let hist = live_code(include_str!("../state/history.rs"));
    let feed = only_body(&hist, &format!("fn comment_lane{}", "_xy(doc:"));
    let shared = ["mission_editor", "::", "comment_lane_xy("].concat();
    assert!(
        feed.contains(&shared),
        "T-784: the lane feed must delegate to mission_editor::comment_lane_xy — the function \
         the pick's own list is packed from; got:\n{feed}"
    );
    assert!(
        !feed.contains("serde_json"),
        "T-784: a second parse in the feed is the two-readers shape this ticket removed; got:\n\
         {feed}"
    );
    // And the packing really is a projection of the picked list, not a parallel parse.
    let me = glyph_block();
    let pack = only_body(&me, &format!("pub(crate) fn comment_lane{}", "_xy("));
    assert!(
        pack.contains(&format!("comment{}", "_points(")),
        "T-784: comment_lane_xy must be comment_points packed; got:\n{pack}"
    );
}

/// **The map click folds the comment into `hit`.** Not a branch beside it: `apply_click` then
/// gives a comment the same replace/toggle semantics a slot has, Ctrl+click COMPOSES it with an
/// entity selection (the one `Vec` `save_composition` reads), and the edge selection is dropped
/// by the reconcile inside `mirror_selection` rather than by a clear written here.
///
/// ORDER IS LOAD-BEARING and pinned: the fold must come AFTER `complete_connect`, or an armed
/// connection would take a comment as an endpoint — an edge to a thing that never compiles.
#[test]
fn the_map_click_folds_the_comment_into_the_entity_hit() {
    // Scope to the CLICK path — the pointerup handler. T-796 added a SECOND comment pick at the
    // drag-START (the pointermove handler, earlier in the file) so a drag grabs a note; that pick
    // is verified by `t796_comment_drag::the_drag_start_folds_the_comment_into_the_move_hit`.
    // This test is about the click ordering (comment after complete_connect, before the edge /
    // apply_click), all of which live in pointerup, so anchoring here keeps that intent exact.
    let whole = page();
    let up = whole
        .find("let onpointerup = ")
        .expect("T-784: the pointerup handler must survive");
    let code = &whole[up..];
    let pick = ["pick", "_comment("].concat();
    let read = ["comment", "_points("].concat();
    let connect = ["editor_ops", "::", "complete_connect("].concat();
    let edge = ["pick", "_connection("].concat();
    let apply = ["apply", "_click("].concat();
    let at_connect = code.find(&connect).expect("the connect arm must survive");
    let at_pick = code
        .find(&pick)
        .expect("T-784: the map must hit-test the comment glyph");
    let at_edge = code.find(&edge).expect("the connection pick must survive");
    let at_apply = code.find(&apply).expect("the click must reach apply_click");
    assert!(
        code.contains(&read),
        "T-784: the pick must be fed from comment_points — the lane's own document read"
    );
    assert!(
        at_connect < at_pick,
        "T-784: the comment pick must run AFTER complete_connect, or a connect arm would take \
         a note as an endpoint"
    );
    assert!(
        at_pick < at_edge && at_pick < at_apply,
        "T-784: the glyph must be picked before the edge (a note is a POINT target with a \
         tight radius; an edge is a long segment that would otherwise swallow every click \
         landing near a note that crosses it) and before apply_click consumes the hit"
    );
    // No second selection route: the map must not write the selection for a comment itself.
    let fold = ["let hit = hit.", "or_else("].concat();
    assert!(
        code.contains(&fold),
        "T-784: the comment hit must be FOLDED INTO `hit`; a branch of its own would be a \
         second selection path with its own replace/toggle/compose semantics to keep in step"
    );
}

/// **A comment selection COMPOSES with an entity selection, and the reconcile stays inside the
/// single writer.** The composition capture classifies each id in ONE `Vec` as slot / vehicle /
/// object / comment, so a comment in a lane of its own could never be captured — "exclusive"
/// would defeat T-781 outright. Nothing here adds a per-route clear: the edge selection is
/// dropped because `reconcile_connection_selection` tests "is the entity selection non-empty?",
/// a question a comment id answers by simply being in it.
#[test]
fn a_comment_composes_and_the_reconcile_is_still_the_one_writers_job() {
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
    let capture = only_body(&ops, &format!("fn capture_selection{}", "_entities("));
    assert!(
        capture.contains(&format!("comments{}", ".get(id)")),
        "T-784: the composition capture must still read a comment out of the SAME selection \
         slice it reads slots/vehicles/objects from — that is the evidence for COMPOSES"
    );
    let reconcile = ["reconcile_connection", "_selection(ctx)"].concat();
    assert!(
        only_body(&ops, "fn mirror_selection(").contains(&reconcile),
        "T-784: the reconcile must remain inside the one selection writer"
    );
    let body = only_body(&ops, "fn reconcile_connection_selection(");
    assert!(
        body.contains("selection.borrow().is_empty()"),
        "T-784: the reconcile must key on the selection being NON-EMPTY, not on a kind — that \
         is what makes a comment selection drop the edge with no code of its own"
    );
    for kind in ["Comment", "comment"] {
        assert!(
            !body.contains(kind),
            "T-784: the reconcile must not learn about comments; a kind test there is the \
             per-route shape wave 129 F-7 and wave 142 F-1 were both caused by"
        );
    }
    let push = ["selected_ids", ".set("].concat();
    assert_eq!(
        ops.matches(&push).count(),
        1,
        "T-784: still exactly ONE writer of selected_ids — a comment selection must reach the \
         screen through the same mirror everything else does"
    );
}

/// **Delete removes the comment, and nothing else.** The verb partitions the selection and asks
/// `comment_details` — the same `comments_json` read the rows, the lane and the pick share —
/// never a `cmt-` prefix test (that prefix is a minting convention, not a document invariant).
/// `remove_slots` is guarded so a comment-only Delete cannot open an empty transaction over a
/// document it did not change: that is the T-779 class, and it is precisely what handing a
/// comment id to `remove_slots` used to do.
#[test]
fn delete_partitions_the_selection_by_what_the_document_says() {
    let ops = live_code(include_str!("../state/operations/entity.rs"));
    let del = only_body(&ops, "pub fn delete_selection(");
    let ask = ["comment", "_details(core)"].concat();
    let part = "partition(";
    let remove = ["core.", "remove_comment("].concat();
    let slots = ["core.", "remove_slots("].concat();
    let at_ask = del
        .find(&ask)
        .expect("T-784: membership must be asked of the document's own comment map");
    let at_part = del.find(part).expect("T-784: the selection must be split");
    let at_remove = del
        .find(&remove)
        .expect("T-784: the comment half must reach the comment mutator");
    assert!(
        at_ask < at_part && at_part < at_remove,
        "T-784: ask the document, split, then delete — in that order"
    );
    assert!(
        del.contains(&slots) && del.contains("is_empty()"),
        "T-784: the slot half must survive AND be guarded — an unguarded remove_slots on an \
         empty half is an undo step over a document this delete did not touch"
    );
    assert!(
        !del.contains("cmt-") && !del.contains("starts_with"),
        "T-784: a prefix test is a second vocabulary for 'is this a comment?' and it is wrong \
         for any hydrated mission whose ids were not minted here"
    );
}
