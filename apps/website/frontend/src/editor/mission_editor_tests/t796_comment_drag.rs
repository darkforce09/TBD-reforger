use super::{comment_drag_lane_xy, comment_points, dragged_comment_points};
use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

/// Two notes; a hydrated mission whose ids were NOT minted with the `cmt-` prefix, to prove
/// membership is asked of the document, never of the id text.
fn comments() -> String {
    serde_json::json!({
        "note-a": { "title": "A", "position": { "x": 100.0, "z": 10.0 } },
        "note-b": { "title": "B", "position": { "x": 300.0, "z": -30.0 } },
    })
    .to_string()
}

fn page() -> String {
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    let raw = include_str!("../mission_editor.rs");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    live_code(&raw[raw.find(anchor.as_str()).expect("counted")..])
}

/// **The dragged set is a projection of the document's comment list, selected by id.** Base
/// positions ride along, so the commit's `base + delta` and the preview's offset are ONE read.
#[test]
fn dragged_points_are_the_document_notes_filtered_by_id() {
    let pts = comment_points(&comments());
    let got = dragged_comment_points(&pts, &["note-b".to_string()]);
    assert_eq!(got.len(), 1, "only the dragged id is returned");
    assert_eq!(got[0].id, "note-b");
    assert_eq!(
        (got[0].x, got[0].y),
        (300.0, -30.0),
        "with its authored x/z"
    );
    // An id not in the document contributes nothing (a stale selection entry cannot move a ghost).
    assert!(dragged_comment_points(&pts, &["ghost".to_string()]).is_empty());
    assert!(dragged_comment_points(&pts, &[]).is_empty());
}

/// **The preview re-packs EVERY note, offsetting only the dragged ones.** The notes not in the
/// drag must stay drawn where they are (this lane draws them all), and the dragged note's glyph
/// must sit at base + delta — the "glyph follows the cursor" the O-7 preview-parity note asks for.
#[test]
fn preview_offsets_only_the_dragged_note_and_keeps_the_rest() {
    // Drag note-a by (+50, +5); note-b is not dragged.
    let xy = comment_drag_lane_xy(&comments(), &["note-a".to_string()], 50.0, 5.0);
    assert_eq!(
        xy.len(),
        4,
        "both notes still in the lane — a drag hides nothing"
    );
    // Lane order is `comment_points` order (id-sorted): note-a then note-b.
    assert!(
        (xy[0] - 150.0).abs() < 1e-3 && (xy[1] - 15.0).abs() < 1e-3,
        "T-796: the dragged note is drawn at base + delta, not at its stored position"
    );
    assert!(
        (xy[2] - 300.0).abs() < 1e-3 && (xy[3] - (-30.0)).abs() < 1e-3,
        "T-796: a note NOT in the drag keeps its authored position mid-drag"
    );
    // Zero delta (or an empty drag set) is the identity re-pack the non-commit exits rely on.
    let rest = comment_drag_lane_xy(&comments(), &["note-a".to_string()], 0.0, 0.0);
    assert_eq!(rest, super::comment_lane_xy(&comments()));
    let none = comment_drag_lane_xy(&comments(), &[], 50.0, 5.0);
    assert_eq!(none, super::comment_lane_xy(&comments()));
}

/// The northing (`z`, the note's SECOND HORIZONTAL) is a plane axis, so a drag `dy` translates it
/// like `dx` translates x — it is NOT an elevation to be preserved-or-zeroed. `move_comment(id, x,
/// z)` takes both, and the commit passes `p.y + dy` as z. This guards against the z-family trap
/// (a drag that wrote z=None or zeroed a stored elevation) — for a comment there is no elevation,
/// the second axis is northing and it moves.
#[test]
fn the_northing_translates_with_the_drag() {
    let xy = comment_drag_lane_xy(&comments(), &["note-b".to_string()], 0.0, 100.0);
    // note-b is second in id order; its z was -30, +100 delta ⇒ 70.
    assert!(
        (xy[3] - 70.0).abs() < 1e-3,
        "T-796: dy moves the northing; it is a horizontal, not a preserved elevation"
    );
}

/// **Drag-start folds the comment pick into `hit`** — after slot/vehicle, before the marquee
/// fallthrough. The precedence the chosen ordering makes: ring (T-795) short-circuits earlier,
/// then slot/vehicle wins its pixels, then a comment, then None ⇒ marquee. The fold is the same
/// `hit.or_else(|| … pick_comment …)` shape T-784 uses on the click path, so the drag grabs a note
/// over the identical hit box the click selects it with — not a second pick with its own radius.
#[test]
fn the_drag_start_folds_the_comment_into_the_move_hit() {
    let code = page();
    let start = code
        .find("st::pick_slot_or_vehicle(")
        .expect("T-796: the drag-start slot/vehicle pick must survive");
    // The FIRST fold after the drag-start pick is the drag one (the click-path fold is later in
    // the file, inside the pointerup selection block).
    let region = &code[start..];
    let fold = ["let hit = hit.", "or_else("].concat();
    let at_fold = region
        .find(&fold)
        .expect("T-796: the drag-start must fold a comment pick into `hit`");
    let pick = ["pick", "_comment("].concat();
    let read = ["comment", "_points("].concat();
    let at_pick = region[at_fold..]
        .find(&pick)
        .expect("T-796: the fold must hit-test the comment glyph");
    assert!(
        region[at_fold..].contains(&read),
        "T-796: the drag-start comment pick must be fed from comment_points — the lane's read"
    );
    let at_marquee = region
        .find("LG::Marquee")
        .expect("T-796: the marquee fallthrough must survive");
    assert!(
        at_fold < at_marquee && at_fold + at_pick < region.find("LG::Marquee").unwrap(),
        "T-796: a comment must be picked BEFORE the None => LG::Marquee arm, or a drag on a note \
         falls through to a marquee that destroys the selection (the F-16 class)"
    );
}

/// **The move commit routes comments to `move_comment`, and the slot/vehicle half to the atomic
/// translate — split by asking the document.** A comment id must never reach `move_entities`
/// (which reads the slot SoA and would move a note nowhere — the O-6 defect); it is partitioned
/// out by `comment_details` (the `delete_selection` membership rule, never a `cmt-` prefix) and
/// sent to `move_comment`, base + delta, one txn each.
#[test]
fn the_move_commit_partitions_comments_to_their_own_mutator() {
    let code = page();
    // Scope to the LG::Move pointerup commit: from the move-commit doc comment to the marquee arm.
    let commit_start = code
        .find("move_entities_and_vehicles(")
        .expect("T-796: the slot/vehicle move-commit must survive");
    // Walk BACK to the start of the drag-commit block (the comment partition sits before it).
    let block_anchor = code[..commit_start]
        .rfind("if dx != 0.0")
        .expect("T-796: the drag-commit delta guard must survive");
    let region = &code[block_anchor..];
    let details = ["comment", "_details("].concat();
    let mv = ["editor_ops", "::", "move_comment("].concat();
    let at_details = region
        .find(&details)
        .expect("T-796: the commit must ask the document which ids are comments");
    let at_move = region
        .find(&mv)
        .expect("T-796: the comment half must reach move_comment, not move_entities");
    let at_entities = region.find("move_entities_and_vehicles(").expect("counted");
    assert!(
        at_details < at_move && at_move < at_entities,
        "T-796: ask the document, move the notes, THEN move the slot/vehicle remainder — so a \
         note is never handed to move_entities (which would move it nowhere)"
    );
    // The slot/vehicle partition must EXCLUDE the comment ids, or a note double-commits.
    let veh_part = region
        .find("partition(|id| editor_ops::is_vehicle_id(id))")
        .expect("T-796: the veh/slot partition must survive");
    assert!(
        region[..veh_part].contains("!comment_ids"),
        "T-796: the slot/vehicle partition must filter the comment ids out first"
    );
    assert!(
        !region[..at_move].contains("cmt-") && !region[..at_move].contains("starts_with"),
        "T-796: membership is the document's answer, not a prefix test on the id"
    );
}

/// **`move_comment` is one core transaction ⇒ one Ctrl+Z, so a single-note drag is ONE undo
/// step.** The ticket's "moving must be ONE step" is the single-comment case (what
/// `compute_move_ids` produces for a drag of an unselected note); a multi-note drag is the SAME
/// accepted per-txn class the `delete_selection` comment loop documents — never lost work. This
/// pins the mutator's one-txn contract at its source.
#[test]
fn move_comment_is_one_transaction() {
    let ops = live_code(include_str!("../state/operations/entity.rs"));
    let body = only_body(&ops, "pub fn move_comment(");
    assert!(
        body.contains("set_comment_position("),
        "T-796: move_comment must write through the core's set_comment_position"
    );
    let store = include_str!("../../../../../../crates/map-engine-core/src/doc/store.rs");
    // set_comment_position delegates the write to set_comment_field (the shared read-modify-write
    // for all three comment field edits), which is where the SINGLE transaction is opened.
    let sp = only_body(store, "pub fn set_comment_position(");
    assert!(
        sp.contains("set_comment_field("),
        "T-796: set_comment_position must route through the shared one-txn field writer"
    );
    let field = only_body(store, "fn set_comment_field(");
    assert_eq!(
        field.matches("self.begin()").count(),
        1,
        "T-796: the comment field write opens exactly one transaction — one drag, one undo step"
    );
}

/// **Every non-commit exit re-binds the comment lane to the authored positions.** A zero-delta
/// release, a wrong-button release, and a pointercancel each ran the preview offset; without a
/// committed move re-binding the lane (which `after_doc_change` does after a real move), a
/// dragged-then-abandoned note would stay parked at the last preview offset — the same lie the
/// vehicle lane's `clear_drag_preview` exists to prevent, applied to the comment lane.
#[test]
fn abandoned_drags_re_bind_the_comment_lane() {
    let code = page();
    // The preview arm binds the OFFSET lane; the non-commit exits bind the AUTHORED lane.
    assert!(
        code.contains("comment_drag_lane_xy("),
        "T-796: the pointermove preview must re-pack the comment lane with the live offset"
    );
    // Count the authored re-binds at the non-commit exits: zero-delta else, wrong-button Move,
    // pointercancel Move. All three call comments_bind_ids(comment_lane_xy(...), …).
    //
    // T-808 widened the needle from `comments_bind` to `comments_bind_ids`: the lane's entry
    // point is now the one that carries the id column, and an exit that dropped back to the
    // id-less `comments_bind` would restore the note's POSITION while stripping its selection
    // ring — the T-796 defect with a different last step. `comments_bind_ids` delegates to
    // `comments_bind`, so this is the same claim about the same upload, one call deeper.
    let authored = ["e.comments_bind", "_ids(&cxy, cids)"].concat();
    assert!(
        code.matches("comment_lane_xy(&c.comments_json())").count() >= 3,
        "T-796: the three non-commit exits (zero delta, wrong button, cancel) must each re-bind \
         the authored comment lane; found fewer than three"
    );
    assert!(
        code.contains(&authored),
        "T-796/T-808: the restore must upload through comments_bind_ids, the lane's only entry \
         point — and it must pass the ids, or the restored note loses its selection ring"
    );
}
