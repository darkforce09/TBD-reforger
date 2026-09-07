//! T-936.7 — the tactical-graphics lane must be bound at BOTH sites, not just the edit one.
//!
//! Filed by the wave-255 adversarial verify as a BLOCKER. The slice bound the lane from
//! `after_doc_change` only, which is reached from undo, redo and `after_local_edit` — i.e. from an
//! EDIT. Rows reach a document by a different route entirely: the IDB restore
//! (`mission_editor.rs`), the server hydrate and conflict resolution (`state/hydrate.rs`), and
//! T-190's peer merge (`state/persist.rs`), all of which land in `rebind_engine_from_doc`.
//!
//! The consequence was not subtle. `begin_tactical_draw` has no caller yet, so a hydrated payload
//! is currently the ONLY way a tactical graphic can exist at all — which made the slice's headline
//! acceptance ("the canvas draws all four kinds") fail on 100% of live openings, while
//! `live_tactical_graphics` still answered picks off the document. An invisible graphic that is
//! nonetheless clickable and deletable is precisely the "what is drawn and what a click can find
//! are one set" invariant the slice cites in its own module header.
//!
//! This is the same pin the repo already keeps for every other lane — `t760_markers_bind_feed`,
//! `t780_connection_line`, `t819_crewed_render_hide` — and the reason they exist is that a lane
//! bound at one site is indistinguishable from a lane bound at both until someone opens a document
//! that was not just edited.

use crate::editor::arsenal::class_r_scrub::{live_code, only_body};

fn history_src() -> String {
    live_code(include_str!("../state/history.rs"))
}

/// Both binders call it. Neither is allowed to drift back to one.
#[test]
fn both_bind_sites_upload_the_tactical_lane() {
    let src = history_src();
    let needle = "upload_tactical_graphics(";

    // Non-vacuity first: if the scrub or the rename ate the whole haystack, every `contains`
    // below would be false and this test would "fail correctly" for entirely the wrong reason.
    // `only_body` panics on zero or multiple matches, so a rename is loud rather than silent.
    let edit = only_body(&src, "fn after_doc_change(").to_string();
    let restore = only_body(&src, "pub fn rebind_engine_from_doc(").to_string();
    assert!(
        !edit.is_empty() && !restore.is_empty(),
        "both binder bodies must be found in the scrubbed source"
    );

    assert!(
        edit.contains(needle),
        "after_doc_change must bind the tactical lane — the undo/redo/local-edit half"
    );
    assert!(
        restore.contains(needle),
        "rebind_engine_from_doc must bind the tactical lane — the IDB-restore / hydrate / \
         conflict-resolution / peer-merge half. This is the half T-936.7 shipped without, and it \
         is the ONLY half that can put rows on screen while `begin_tactical_draw` has no caller."
    );
}

/// The lane the restore path forgot is bound beside the ones it did not forget.
///
/// Guards the shape rather than the single call: if a future slice adds a lane to
/// `after_doc_change` and not to `rebind_engine_from_doc`, this test does not catch it — but it
/// does catch anyone deleting the tactical call from the restore half while leaving its siblings,
/// which is exactly how the defect arrived.
#[test]
fn the_restore_binder_binds_every_lane_the_edit_binder_does() {
    let src = history_src();
    let restore = only_body(&src, "pub fn rebind_engine_from_doc(").to_string();
    for lane in [
        "upload_squad_links(",
        "vehicles_bind_symbology(",
        "markers_bind(",
        "comments_bind_ids(",
        "upload_tactical_graphics(",
    ] {
        assert!(
            restore.contains(lane),
            "rebind_engine_from_doc must bind `{lane}` — a lane missing here draws nothing on a \
             document that was opened rather than edited"
        );
    }
}
