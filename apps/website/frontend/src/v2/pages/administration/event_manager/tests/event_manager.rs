//! Guards on the operations calendar: the edit form's wiring, and the delete confirmation's copy.

/// Strip `//` and `/* */` comments so the assertions below read live code only.
///
/// The pins that matter here are identifiers and request bodies. Matching comment prose instead
/// would let a comment-only edit keep the guard green over code that no longer does the thing.
fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2;
            } else {
                i = bytes.len();
            }
            continue;
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

#[test]
fn edit_dialog_reattach_and_empty_string_clear_are_wired() {
    let src = crate::v2::core::test_support::pins::event_manager_source();
    let code = strip_rust_comments(&src);

    assert!(
        code.contains("edit_attach_open"),
        "Edit dialog must own an attach dropdown distinct from create's attach_open \
         (perturbation: remove edit_attach_open)"
    );
    assert!(
        code.contains("data-testid=\"edit-attach-mission\""),
        "Edit Attach Mission control must be present \
         (perturbation: remove data-testid=\"edit-attach-mission\")"
    );
    assert!(
        code.contains("on_attach_mission") && code.contains("&format!(\"/events/{id}/missions\")"),
        "Edit attach must POST /events/{{id}}/missions \
         (perturbation: drop on_attach_mission or the missions path)"
    );
    // Create also POSTs that path; require the Edit-specific success toast so a create-only
    // caller cannot false-green the re-attach pin.
    assert!(
        code.contains("Attached {title}"),
        "Edit attach success toast must be distinct from create's publish toast \
         (perturbation: remove Attached {{title}} toast)"
    );
    // Blessed clear: blanking the form diffs against unwrap_or_default() and inserts
    // the (possibly empty) string — key present with "" clears server-side.
    assert!(
        code.contains("edit_briefing.get_untracked()")
            && code.contains("orig.briefing.clone().unwrap_or_default()")
            && code.contains("body.insert(\"briefing\".into(), br.into())"),
        "Edit save must POST briefing when changed — including \"\" clears \
         (perturbation: stop inserting empty briefing / drop unwrap_or_default diff)"
    );
    assert!(
        code.contains("edit_banner.get_untracked()")
            && code.contains("orig.banner_image_url.clone().unwrap_or_default()")
            && code.contains("body.insert(\"banner_image_url\".into(), bn.into())"),
        "Edit save must POST banner_image_url when changed — including \"\" clears \
         (perturbation: stop inserting empty banner / drop unwrap_or_default diff)"
    );
}

/// The delete confirmation must describe the delete the endpoint actually performs.
///
/// `DELETE /api/v1/events/:id` marks the row deleted and touches nothing else, which the API crate
/// proves against real database state. This asserts the operator is told that, and — the part that
/// matters — that the copy cannot quietly slide back to promising a cascade.
///
/// The false claims are banned by shape rather than by exact sentence, so a reworded one fails
/// too: any form of "cannot be undone" / "permanent" / "erased", and any claim that the
/// registrations or ORBATs are removed.
#[test]
fn delete_confirm_copy_matches_the_soft_delete_handler() {
    let copy = super::DELETE_EVENT_CONFIRM_DESC.to_ascii_lowercase();

    // The promises the handler does not keep.
    for lie in [
        "cannot be undone",
        "can't be undone",
        "permanently",
        "irreversible",
        "deleted forever",
    ] {
        assert!(
            !copy.contains(lie),
            "the delete dialog must not claim `{lie}`: delete_event sets deleted_at and \
             nothing else, so the operation and every child row are still there. \
             If the handler became a hard cascade, fix the handler's test first."
        );
    }
    for destroyed in ["registrations are removed", "registrations are deleted"] {
        assert!(
            !copy.contains(destroyed),
            "the delete dialog must not claim `{destroyed}`: no cascade fires — \
             `event_registrations` rows survive `DELETE /api/v1/events/:id` untouched"
        );
    }

    // And the truth it must state instead.
    assert!(
        copy.contains("nothing is erased"),
        "the copy must say plainly that nothing is destroyed; it is a soft delete"
    );
    assert!(
        copy.contains("kept") || copy.contains("retained"),
        "the copy must say the missions / ORBATs / registrations are kept"
    );
    assert!(
        copy.contains("restore") || copy.contains("recover"),
        "a soft delete is recoverable and the operator should be told so"
    );

    // And the dialog must actually render it. `live_source` keeps string literals, because the
    // user-visible copy is the thing being pinned, but folds comments and any dead branch a decoy
    // could hide in.
    let prod = crate::v2::core::test_support::class_r_scrub::live_source(
        &crate::v2::core::test_support::pins::event_manager_source(),
    );
    assert!(
        prod.contains("description=DELETE_EVENT_CONFIRM_DESC"),
        "the delete confirm Dialog must render DELETE_EVENT_CONFIRM_DESC — a constant nothing \
         displays is not user-visible copy, and asserting on it would be the very defect \
         this guard exists to prevent"
    );
    assert!(
        prod.contains("title=DELETE_EVENT_CONFIRM_TITLE"),
        "…and DELETE_EVENT_CONFIRM_TITLE"
    );
    // The sibling detach dialog at the bottom of the tree guards a real hard delete —
    // registrations, ORBAT slots and the attachment itself, in one transaction — so its
    // "This cannot be undone." is accurate and must survive this ban.
    assert!(
        prod.contains("The mission's ORBAT slots and every registration on it are deleted."),
        "the detach confirm is accurate about a genuine cascade and must not be softened \
         along with the event-delete copy"
    );
}
