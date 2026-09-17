use super::{
    is_acceptable_thumbnail_url, presentation_failure_message, PresentationField, RowShape,
    BRIEFING_NOTE, THUMBNAIL_REJECTED_NOTE, THUMBNAIL_URL_NOTE,
};
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

fn row() -> RowShape {
    RowShape {
        game_mode: "pve_coop".into(),
        max_players: 64,
        briefing: "Hold the bridge.".into(),
        thumbnail_url: "https://cdn.example/op.jpg".into(),
    }
}

/// The two variants are the two `PatchMissionInput` members. A column name that drifts from the
/// server's spelling ships a control whose PATCH is silently ignored — the handler builds its
/// UPDATE from named `Option`s, so an unknown key is dropped, not rejected. That failure is
/// invisible: the optimistic write stays on screen and the 200 confirms nothing.
#[test]
fn presentation_columns_are_the_patch_body_keys() {
    assert_eq!(PresentationField::Briefing.column(), "briefing");
    assert_eq!(PresentationField::Thumbnail.column(), "thumbnail_url");
    for f in [PresentationField::Briefing, PresentationField::Thumbnail] {
        assert!(
            !f.what().trim().is_empty(),
            "T-671: {f:?} has no name to put in a failure toast"
        );
    }
}

/// `read`/`write` address the field they name and nothing else — the property the shared setter
/// rests on. Perturbation this catches: a copy-paste in [`PresentationField::write`] that sends
/// the thumbnail into the briefing column (or vice versa), which would silently destroy one
/// value while appearing to save the other.
#[test]
fn each_field_addresses_only_its_own_column() {
    let mut r = row();
    PresentationField::Briefing.write(&mut r, "New orders.".into());
    assert_eq!(PresentationField::Briefing.read(&r), "New orders.");
    assert_eq!(
        PresentationField::Thumbnail.read(&r),
        "https://cdn.example/op.jpg",
        "T-671: writing the briefing must not touch the thumbnail"
    );
    PresentationField::Thumbnail.write(&mut r, "https://cdn.example/two.png".into());
    assert_eq!(
        PresentationField::Briefing.read(&r),
        "New orders.",
        "T-671: writing the thumbnail must not touch the briefing"
    );
    assert_eq!(r.game_mode, "pve_coop", "T-671: neither touches the shape");
    assert_eq!(r.max_players, 64);
}

/// The pre-flight agrees with `handlers/missions.rs::validated_thumbnail_url`: empty clears the
/// column, an absolute http/https URL is stored, everything else is refused. The `/uploads/…`
/// case is the load-bearing one — that is the shape the platform's only upload endpoint returns,
/// and it is precisely what this column cannot hold.
#[test]
fn thumbnail_accept_rule_matches_the_write_boundary() {
    for ok in [
        "",
        "   ",
        "https://cdn.example/t.jpg",
        "http://cdn.example/t.jpg",
        "https://cdn.example/a/b?c=d#e",
    ] {
        assert!(
            is_acceptable_thumbnail_url(ok),
            "T-671: {ok:?} is stored by validated_thumbnail_url and must not be refused here"
        );
    }
    for bad in [
        "javascript:alert(1)",
        "data:image/png;base64,AAAA",
        "/uploads/2f1c.png",
        "//evil.example/t.jpg",
        "t.jpg",
        "ftp://cdn.example/t.jpg",
        "https://",
    ] {
        assert!(
            !is_acceptable_thumbnail_url(bad),
            "T-671: {bad:?} would be refused by the server — refuse it beside the field"
        );
    }
}

/// A refused PATCH names the 403 case separately — `PATCH /missions/:id` gates on authorship
/// while the editor route gates on role, so a non-author is refused every time and retrying
/// cannot help — and both texts say the control has been put back, because [`super::ShapeMirror`]
/// puts it back.
#[test]
fn refused_presentation_patch_explains_itself() {
    for f in [PresentationField::Briefing, PresentationField::Thumbnail] {
        let forbidden = presentation_failure_message(f, &(403, None)).to_lowercase();
        assert!(forbidden.contains("author"), "T-671: {forbidden:?}");
        assert!(forbidden.contains("put back"), "T-671: {forbidden:?}");
        // `api_error_message` sentence-cases what the server said, so compare lowercased.
        let other = presentation_failure_message(f, &(500, Some("boom".into()))).to_lowercase();
        assert!(
            other.contains("boom"),
            "T-671: a non-403 must name what the server said, got {other:?}"
        );
        assert!(other.contains("put back"), "T-671: {other:?}");
    }
}

/// Copy pins. The briefing note must keep separating the library blurb from the per-faction
/// in-game briefing (the two names are one keystroke apart and have been conflated before —
/// `compile.rs::compile_export` carries the whole argument), and the thumbnail note must keep
/// saying it is a link and not an upload. An author who cannot find the file picker that was
/// never built is the cost of trimming that sentence.
#[test]
fn the_copy_says_what_each_field_is() {
    let briefing = BRIEFING_NOTE.to_lowercase();
    assert!(
        briefing.contains("faction"),
        "T-671: the briefing note must say the in-game briefing is authored per faction"
    );
    let thumb = THUMBNAIL_URL_NOTE.to_lowercase();
    assert!(
        thumb.contains("http://") && thumb.contains("https://"),
        "T-671: the thumbnail note must state the accept rule"
    );
    assert!(
        thumb.contains("no upload") || thumb.contains("stores a link"),
        "T-671: the thumbnail note must say why there is no file picker"
    );
    assert!(
        THUMBNAIL_REJECTED_NOTE.to_lowercase().contains("not saved"),
        "T-671: a locally refused link must say it was not saved"
    );
}

/// **Both fields reach the `missions` ROW by PATCH**, which is the entire ticket: the columns and
/// the handler already existed and nothing called them. Perturbation this catches: wiring either
/// control into `author_env` (where no compile, card or dossier would ever read it), or dropping
/// the setter call and leaving a control that repaints and saves nothing.
#[test]
fn presentation_reaches_the_missions_row_by_patch() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let setter = format!("set{}", "_presentation");
    let body = only_body(&src, &format!("fn render{}", "_presentation_section"));
    assert!(
        body.contains(&format!("{setter}(")),
        "T-671: both presentation controls must call {setter}"
    );
    for variant in ["Briefing", "Thumbnail"] {
        assert!(
            body.contains(&format!("PresentationField::{variant}")),
            "T-671: the {variant} control must name its column through PresentationField"
        );
    }
    assert!(
        !body.contains(&format!("author{}", "_env")),
        "T-671: briefing and thumbnail are `missions` row columns, not meta.environment keys"
    );

    let setter_body = only_body(&src, &format!("fn {setter}"));
    assert!(
        setter_body.contains(&format!("api{}", "_patch")),
        "T-671: {setter} must PATCH the mission row"
    );
    assert!(
        setter_body.contains(&format!("is{}", "_mission_row_id")),
        "T-671: {setter} must refuse synthetic editor ids before hitting the wire"
    );
    assert!(
        setter_body.contains(&format!("is{}", "_acceptable_thumbnail_url")),
        "T-671: {setter} must pre-flight the thumbnail against the server's accept rule"
    );
}

/// **A briefing is not a scrubber.** The controls commit on `change` — one settled value per
/// edit — rather than on `input`. A textarea wired to `input` would PATCH per keystroke, and
/// wrapping that in the T-192 debounce would only be undoing the mistake; there is nothing for a
/// sequencer to re-order when the field produces one write per visit.
///
/// Perturbation this catches: swapping either handler to `on:input`, with or without a debounce.
#[test]
fn the_presentation_controls_commit_on_settle() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let body = only_body(&src, &format!("fn render{}", "_presentation_section"));
    assert!(
        body.contains(&format!("on:{}", "change")),
        "T-671: the presentation controls must commit on settle"
    );
    assert!(
        !body.contains(&format!("on:{}", "input")),
        "T-671: a PATCH per keystroke is strictly worse than either shape this ticket weighed"
    );
    // And the settle actually happens on the keyboard close path: Escape blurs first, or a
    // briefing typed and then dismissed with Esc is discarded with no message.
    let dialog = only_body(&src, "fn MissionSettingsDialog");
    assert!(
        dialog.contains(&format!("blur{}", "_focused_control")),
        "T-671: Escape must commit the focused control before the dialog unmounts"
    );
    assert!(
        dialog.contains(&format!("render{}", "_presentation_section")),
        "T-671: Mission Settings must actually mount the presentation block"
    );
}

/// **A saved briefing also moves the document's copy.** `compile_export` fills the download
/// envelope's `briefing` from `meta.briefing`, whose only writer before this ticket was boot
/// hydrate — so a briefing typed here and exported in the same session shipped the pre-edit
/// value. Perturbation this catches: dropping the mirror, or mirroring before the PATCH lands
/// (which would put a refused briefing into the export).
#[test]
fn a_saved_briefing_reaches_the_documents_meta() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let mirror = format!("mirror{}", "_briefing_into_document");
    assert!(
        only_body(&src, &format!("fn set{}", "_presentation")).contains(&format!("{mirror}(")),
        "T-671: a saved briefing must be mirrored into the live document"
    );
    assert!(
        only_body(&src, &format!("fn {mirror}")).contains(&format!("apply{}", "_row_meta")),
        "T-671: the mirror must go through MissionDocCore's row-meta mutator"
    );
}

/// The thumbnail `<img>` is guarded at the SINK as well as at the writer — T-405's rule, and the
/// same one `announcements.rs::thumbnail_img_src` applies to the same class of value. The row is
/// read from the server, and a value stored before the T-413 write guard shipped is exactly the
/// case a writer-side check cannot cover.
#[test]
fn the_preview_checks_the_url_at_the_sink() {
    let src = live_source(include_str!("../settings_modal.rs"));
    let body = only_body(&src, &format!("fn render{}", "_presentation_section"));
    let guard = format!("is{}", "_acceptable_thumbnail_url");
    let img = format!("<{}", "img");
    assert!(body.contains(&img), "T-671: the section renders a preview");
    assert!(
        body.contains(&guard),
        "T-671: the preview must be gated on the same accept rule the writer uses"
    );
}
