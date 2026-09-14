//! The guards on the new-mission dialog: the shape of the request it builds.
//!
//! Source scans, because both claims are about the shape of the request the dialog builds:
//! that the briefing is on it, and that the thumbnail is not. The needles are assembled from
//! fragments, and the scrubber truncates at the first test module, so this file cannot become
//! its own haystack.

use crate::v2::core::test_support::class_r_scrub::{live_source, only_body};

/// The briefing is typed here and it reaches `POST /missions`. `CreateMissionInput::briefing`
/// binds straight into the INSERT, so a control that does not make it onto the body is a field
/// that looks authored and stores `''`.
///
/// Perturbation this catches: adding the textarea without the body key (or removing the key and
/// leaving the box), which is exactly the defect this ticket found — an accepting API with no
/// caller.
#[test]
fn the_post_body_carries_the_authored_briefing() {
    let src = live_source(&crate::v2::core::test_support::pins::create_dialog_source());
    let submit = only_body(&src, "fn CreateMissionDialog");
    let key = format!("brief{}", "ing");
    assert!(
        submit.contains(&format!("\"{key}\":")),
        "T-671: POST /missions must carry the authored briefing"
    );
    assert!(
        submit.contains(&format!("<text{}", "area")),
        "T-671: the create dialog must offer somewhere to write it"
    );
    // Trimmed on the way out — a box that was only tabbed through must store `''`, not a
    // whitespace blurb that renders blank but is not empty to any downstream `is_empty()`.
    // Scoped to what follows the KEY: `.trim()` appears elsewhere in this body (the title
    // guard), so an unscoped needle would stay green with the briefing untrimmed.
    let at = submit.find(&format!("\"{key}\":")).expect("checked above") + key.len();
    assert!(
        submit[at..at + 80.min(submit.len() - at)].contains(&format!("tri{}", "m")),
        "T-671: the briefing must be trimmed before it is posted"
    );
}

/// **No thumbnail control here.** The create handler fixes the thumbnail to an empty
/// value and the request type has no such member, so a field on this form would post a
/// key the handler drops — a control that looks saved and saves nothing. The column has
/// another writer, and the editor's Mission Settings is where it is authored.
#[test]
fn the_create_form_offers_no_thumbnail_it_cannot_store() {
    let src = live_source(&crate::v2::core::test_support::pins::create_dialog_source());
    let body = only_body(&src, "fn CreateMissionDialog");
    assert!(
        !body.contains(&format!("thumbnail{}", "_url")),
        "T-671: POST /missions does not accept thumbnail_url — do not post a key it drops"
    );
}
