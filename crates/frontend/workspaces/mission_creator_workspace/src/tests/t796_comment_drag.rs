use super::{comment_drag_lane_xy, comment_points, dragged_comment_points};

/// Two notes; a hydrated mission whose ids were NOT minted with the `cmt-` prefix, to prove
/// membership is asked of the document, never of the id text.
fn comments() -> String {
    serde_json::json!({
        "note-a": { "title": "A", "position": { "x": 100.0, "z": 10.0 } },
        "note-b": { "title": "B", "position": { "x": 300.0, "z": -30.0 } },
    })
    .to_string()
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
