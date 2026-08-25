use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

const HIST: &str = include_str!("../state/history.rs");

fn hist_live() -> String {
    live_code(HIST)
}

/// This file from the page component onward, scrubbed — the mount-time first bind lives there.
/// The anchor is split so this module's own copy of it is not a second occurrence in the raw
/// file (the T-784 `glyph_block` idiom).
fn page() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

/// The comment glyph block, scrubbed — read from `canvas/render_sync.rs` since T-934.10 moved
/// the pure belt there (see the T-784 pin's note on why the slice starts from a raw anchor).
fn glyph_block() -> String {
    let anchor = format!("pub(crate) struct Comment{}", "Point");
    let raw = include_str!("../canvas/render_sync.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

fn symbology_bind() -> String {
    format!("{}{}", "slots_bind_", "symbology(")
}

fn legacy_slot_bind() -> String {
    format!("{}{}", "slots_bind_", "soa(")
}

/// **Feeder 1 — the slot lane carries ROLE and HEADING.** All THREE binds: the engine-mount
/// first bind (this file — the only bind a mission that is never edited ever gets), the IDB /
/// hydrate rebind, and the post-commit rebind. A single site left on `slots_bind_soa` is not a
/// partial win: that path draws rifleman-north, so the symbology would flicker in and out with
/// whichever feed ran last.
#[test]
fn every_slot_feed_binds_role_and_heading() {
    let hist = hist_live();
    let bind = symbology_bind();
    for (name, body) in [
        (
            "rebind_engine_from_doc",
            only_body(&hist, "pub fn rebind_engine_from_doc"),
        ),
        ("after_doc_change", only_body(&hist, "fn after_doc_change")),
    ] {
        assert!(
            body.contains(&bind),
            "T-808: {name} must bind the slot lane through the symbology signature; body:\n\
             {body}"
        );
        assert!(
            body.contains("soa_roles(&soa)"),
            "T-808: {name} must pass the per-row role column; body:\n{body}"
        );
        assert!(
            body.contains("&soa.rotations"),
            "T-808: {name} must pass the per-row heading column; body:\n{body}"
        );
    }
    let mount = page();
    assert!(
        mount.contains(&bind) && mount.contains("soa.rotations"),
        "T-808: the engine-mount first bind must be the symbology bind too — a mission opened \
         and never edited would otherwise be a field of north-pointing riflemen"
    );
    let legacy = legacy_slot_bind();
    assert!(
        !hist.contains(&legacy) && !mount.contains(&legacy),
        "T-808: no feeder may keep calling the id-less slot bind; it passes an empty role and \
         heading column, which the engine honours by drawing the pre-T-808 look"
    );
    // The heading is the DOCUMENT's compass bearing, handed over untouched: `slots_gpu`'s
    // `screen_yaw_for_heading_deg` owns the sign flip, so a feeder that converted would flip it
    // twice and every unit would face its own mirror image.
    assert!(
        !hist.contains("rotations.iter()") && !hist.contains("screen_yaw"),
        "T-808: the feeder must not pre-convert the heading — the engine owns that flip"
    );
}

/// **Feeder 2 — the vehicle lane's four columns come from ONE id-sorted reader.**
///
/// This is the alignment pin, and it is the whole reason the ticket named a trap.
/// `editor_ops::vehicle_rows` sorts by id; `MissionDocCore::vehicle_xy_flat` (what this lane was
/// fed before) walks the `yrs` map in ITERATION order. Feeding `xy` from one and kind / tint /
/// heading from the other would put every vehicle in another vehicle's clothes — the wave-127
/// zip lesson, and strictly worse than the amber disc it replaces, because a silhouette pointing
/// confidently the wrong way is believed.
#[test]
fn the_vehicle_lane_columns_come_from_one_sorted_reader() {
    let hist = hist_live();
    let fields = only_body(&hist, "fn vehicle_lane_fields()");
    assert!(
        fields.contains("vehicle_rows()"),
        "T-808: every vehicle column must come off the id-sorted row reader; body:\n{fields}"
    );
    assert!(
        !hist.contains("vehicle_xy_flat"),
        "T-808: the yrs-iteration-order xy flattener must be gone from the feed — mixing it \
         with the id-sorted reader misaligns all four columns"
    );
    assert_eq!(
        fields.matches("for r in ").count(),
        1,
        "T-808: one pass over one list is what makes the four columns row-aligned BY \
         CONSTRUCTION; a second loop is a second order; body:\n{fields}"
    );
    // One push per column per row, and the only skip (an unplaced ORBAT vehicle) happens
    // BEFORE any column is written — the two facts that make "row i is the same vehicle in all
    // four arrays" a property of the code's shape rather than a claim in a comment.
    assert_eq!(fields.matches("xy.push(").count(), 2, "body:\n{fields}");
    assert_eq!(
        fields.matches("headings.push(").count(),
        1,
        "body:\n{fields}"
    );
    assert_eq!(
        fields.matches("aliases.push(").count(),
        1,
        "body:\n{fields}"
    );
    assert_eq!(
        fields.matches("tints.extend_from_slice(").count(),
        1,
        "body:\n{fields}"
    );
    let skip = fields.find("continue").expect("the unplaced-row skip");
    let first_write = fields.find("xy.push(").expect("counted above");
    assert!(
        skip < first_write,
        "T-808: the unplaced-vehicle skip must precede every column write, or one array gets a \
         row the others do not; body:\n{fields}"
    );

    let bind = format!("{}{}", "vehicles_bind_", "symbology(");
    for (name, body) in [
        (
            "rebind_engine_from_doc",
            only_body(&hist, "pub fn rebind_engine_from_doc"),
        ),
        ("after_doc_change", only_body(&hist, "fn after_doc_change")),
    ] {
        assert!(
            body.contains(&bind),
            "T-808: {name} must bind vehicles through the symbology signature; body:\n{body}"
        );
    }
    assert!(
        !hist.contains("vehicles_bind(&"),
        "T-808: no feeder may keep uploading the kind-less, heading-less disc lane"
    );
}

/// **Feeder 2b — the DRAG PREVIEW keeps the symbology.**
///
/// `after_doc_change` binding through the symbology signature is not enough: `select_tool`'s
/// preview re-binds the same lane on every pointermove, and it called the old `vehicles_bind`.
/// So the moment a drag began, every vehicle on the map lost its silhouette, its side colour and
/// its heading and reverted to an amber disc — popping back only on the commit. Mid-gesture is
/// exactly when the operator is looking at the thing.
///
/// The trap is the same one [`the_vehicle_lane_columns_come_from_one_sorted_reader`] names, and
/// the answer is to REUSE that single builder rather than grow a second one here: the preview's
/// positions are the dragged lane, its other three columns are the document's, and both lists
/// are the id-sorted `vehicle_rows` reader. `vehicle_xy_flat` (yrs iteration order) may not
/// appear on this path at all.
#[test]
fn the_drag_preview_binds_through_the_symbology_signature() {
    let tool = live_code(include_str!("../tools/select_tool.rs"));
    let bind = format!("{}{}", "vehicles_bind_", "symbology(");
    let binder = only_body(&tool, "fn bind_vehicle_preview_lane(");

    assert!(
        binder.contains(&bind),
        "T-808: the preview must upload through the symbology signature or the drag reverts \
         every vehicle to a disc; body:\n{binder}"
    );
    // The POSITIONS bound are the previewed ones — that is the whole point of a preview. The
    // document's xy comes back from the shared builder and must be spent on the row-count gate,
    // never on the lane.
    let first_arg = binder
        .split(&bind)
        .nth(1)
        .expect("the symbology bind, asserted above")
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    assert_eq!(
        first_arg, "xy",
        "T-808: the lane must be bound at the DRAGGED positions, not the document's; \
         body:\n{binder}"
    );
    assert!(
        binder.contains("doc_xy.len() == xy.len()"),
        "T-808: the two snapshots must be row-count gated before they are zipped — a row added \
         or removed between the reads shifts every column after it; body:\n{binder}"
    );

    // ONE column builder. Not a second reader, not a second tint/heading expansion here.
    assert!(
        binder.contains("vehicle_lane_fields()"),
        "T-808: the columns must come from the ONE id-sorted builder the committed render \
         uses, so the preview cannot drift from the drop; body:\n{binder}"
    );
    for forbidden in ["vehicle_rows()", "side_rgba(", "faction-"] {
        assert!(
            !binder.contains(forbidden),
            "T-808: `{forbidden}` here would be a SECOND column builder in a second row order \
             — every vehicle in another's clothes; body:\n{binder}"
        );
    }
    assert!(
        !tool.contains("vehicle_xy_flat"),
        "T-808: the yrs-iteration-order flattener must not reach the preview path — mixing it \
         with the id-sorted reader misaligns all four columns"
    );

    // Both gesture ends go through the one binder, so preview and restore cannot bind
    // different lanes (the restore is what runs on a pointercancel or a zero-delta release).
    for name in ["pub fn push_drag_preview(", "pub fn clear_drag_preview("] {
        let body = only_body(&tool, name);
        assert!(
            body.contains("bind_vehicle_preview_lane("),
            "T-808: {name} must bind through the shared binder; body:\n{body}"
        );
        assert!(
            !body.contains("vehicles_bind"),
            "T-808: {name} must not upload the lane itself — a second bind site is a second \
             chance to drop the symbology; body:\n{body}"
        );
    }
}

/// **Feeder 3 — the comment lane names its rows.** `comments_bind` with no ids marks every
/// bubble unselected, so T-796's selection treatment shipped invisible: the engine held
/// coordinates it could not match against the selection. The ids must be the PICK's own list,
/// or row *i*'s ring lands on row *j*.
#[test]
fn every_comment_feed_names_its_rows() {
    let hist = hist_live();
    let bind = format!("{}{}", "comments_bind", "_ids(");
    for (name, body) in [
        (
            "rebind_engine_from_doc",
            only_body(&hist, "pub fn rebind_engine_from_doc"),
        ),
        ("after_doc_change", only_body(&hist, "fn after_doc_change")),
    ] {
        assert!(
            body.contains(&bind),
            "T-808: {name} must upload the comment lane WITH its ids; body:\n{body}"
        );
        assert!(
            body.contains("comment_lane_ids(doc)"),
            "T-808: {name} must pack the ids through the shared reader; body:\n{body}"
        );
    }
    assert!(
        !hist.contains("e.comments_bind(&"),
        "T-808: the id-less comment bind cannot remain a feed — it silently un-selects"
    );
    // Both columns are projections of `comment_points`, which is also what `pick_comment`
    // hit-tests: drawn, picked and named are one list (the T-784/T-748 rule).
    let me = glyph_block();
    let ids = only_body(&me, &format!("pub(crate) fn comment_lane{}", "_ids("));
    let xy = only_body(&me, &format!("pub(crate) fn comment_lane{}", "_xy("));
    let points = format!("comment{}", "_points(");
    assert!(
        ids.contains(&points) && xy.contains(&points),
        "T-808: the id column and the xy column must both be comment_points packed, or they \
         are two readers that can disagree; ids:\n{ids}\nxy:\n{xy}"
    );
    // …and `mission_history` must not grow a second parser of its own (the T-784 shape).
    let feed = only_body(&hist, &format!("fn comment_lane{}", "_ids(doc:"));
    assert!(
        feed.contains(&format!("mission_editor::comment_lane{}", "_ids(")),
        "T-808: the id feed must delegate to the module that owns the pick's list; got:\n{feed}"
    );
}
