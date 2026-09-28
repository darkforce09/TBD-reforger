//! The board's pure helpers: the text contract, the preview fallback and the thumbnail sink.

use super::super::article_feed::preview_text;
use super::*;

/// A published dispatch with the given preview line and body, every other field filled in.
fn dispatch(snippet: &str, body: &str) -> Announcement {
    Announcement {
        id: "00000000-0000-4000-1000-000000000001".into(),
        title: "Dispatch".into(),
        body: body.into(),
        snippet: snippet.into(),
        tag: "update".into(),
        thumbnail_url: String::new(),
        author_id: "000000000000000001".into(),
        status: "published".into(),
        is_pinned: false,
        pushed_to_discord: false,
        discord_message_id: String::new(),
        published_at: Some("2026-07-22T17:00:00Z".into()),
        created_at: "2026-07-22T16:41:03Z".into(),
        updated_at: "2026-07-22T17:00:00Z".into(),
    }
}

/// The strings fed to the view's text nodes must still contain bare `<` and `&`.
/// RED: pretreat with HTML-escaping before split — this fails on `&lt;`.
#[test]
fn body_paragraphs_preserve_bare_angle_brackets() {
    let authored = "Damage: a < b & c > d\n\nSecond paragraph.";
    let paras = body_paragraph_texts(authored);
    assert_eq!(paras.len(), 2);
    assert_eq!(paras[0], "Damage: a < b & c > d");
    assert!(!paras[0].contains("&lt;"));
    assert!(!paras[0].contains("&amp;"));
}

#[test]
fn preview_prefers_snippet_but_falls_back_to_body_without_entities() {
    let with_snip = dispatch("teaser < ok", "ignored");
    assert_eq!(preview_text(&with_snip), "teaser < ok");

    let from_body = dispatch("", "a < b\n\nmore");
    assert_eq!(preview_text(&from_body), "a < b");
    assert!(!preview_text(&from_body).contains("&lt;"));
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../shared/is_http_url_cases.rs"
));

#[test]
fn announcement_thumbnail_emits_src_only_for_http_urls() {
    let mut wrong = Vec::new();
    for (input, should_img) in IS_HTTP_URL_CASES {
        match (thumbnail_img_src(input), should_img) {
            (Some(_), false) => wrong.push(format!("  RENDERED AN IMG FOR {input:?}")),
            (None, true) => wrong.push(format!("  refused a legitimate thumb {input:?}")),
            _ => {}
        }
    }
    assert!(
        wrong.is_empty(),
        "announcement thumbnail sink wrong on {} of {} cases:\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
}
