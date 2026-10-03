//! Callout goldens: every callout kind, from a GitHub alert and from a bracket marker, and the
//! quotes that stay quotes.

use serde_json::{Value, json};

use crate::services::wiki_markup::tests::golden_blocks;

/// A paragraph holding one text run.
fn paragraph(text: &str) -> Value {
    json!({ "type": "paragraph", "inlines": [{ "type": "text", "text": text }] })
}

/// The one callout `markdown` reads to, as `(kind, blocks)`.
fn callout_of(markdown: &str) -> (String, Value) {
    let blocks = golden_blocks(markdown);
    let [block] = blocks.as_array().expect("blocks array").as_slice() else {
        panic!("{markdown:?} did not read to exactly one block: {blocks:#}");
    };
    assert_eq!(block["type"], "callout", "{markdown:?}: {blocks:#}");
    (
        block["kind"].as_str().expect("kind").to_string(),
        block["blocks"].clone(),
    )
}

#[test]
fn wiki_markup_callout_github_alerts_of_every_kind() {
    for (marker, kind) in [
        ("NOTE", "note"),
        ("TIP", "tip"),
        ("IMPORTANT", "important"),
        ("WARNING", "warning"),
        ("CAUTION", "caution"),
        ("note", "note"),
    ] {
        let (read_kind, blocks) = callout_of(&format!("> [!{marker}]\n> Body text.\n"));
        assert_eq!(read_kind, kind, "{marker}");
        assert_eq!(blocks, json!([paragraph("Body text.")]), "{marker}");
    }
}

#[test]
fn wiki_markup_callout_bracket_markers_of_every_kind() {
    for (marker, kind) in [
        ("CRITICAL", "critical"),
        ("CAUTION", "caution"),
        ("WARNING", "warning"),
        ("TIP", "tip"),
        ("NOTE", "note"),
        ("INFO", "info"),
        ("Critical", "critical"),
        ("info", "info"),
    ] {
        let (read_kind, blocks) = callout_of(&format!("> [!{marker}] Rule text\n> continues.\n"));
        assert_eq!(read_kind, kind, "{marker}");
        assert_eq!(
            blocks,
            json!([paragraph("Rule text\ncontinues.")]),
            "{marker}"
        );
    }
}

#[test]
fn wiki_markup_callout_bracket_marker_alone_on_its_line() {
    let (kind, blocks) = callout_of("> [!CRITICAL]\n> No team-killing.\n");
    assert_eq!(kind, "critical");
    assert_eq!(blocks, json!([paragraph("No team-killing.")]));

    let (kind, blocks) = callout_of("> [!INFO]\n");
    assert_eq!(kind, "info");
    assert_eq!(blocks, json!([]));
}

#[test]
fn wiki_markup_callout_bracket_marker_before_markup_keeps_the_markup() {
    let (kind, blocks) = callout_of("> [!CRITICAL] **Live fire**\n>\n> Clear the range.\n");
    assert_eq!(kind, "critical");
    assert_eq!(
        blocks,
        json!([
            {
                "type": "paragraph",
                "inlines": [{
                    "type": "strong",
                    "children": [{ "type": "text", "text": "Live fire" }],
                }],
            },
            paragraph("Clear the range."),
        ])
    );
}

#[test]
fn wiki_markup_callout_quotes_without_a_marker_stay_quotes() {
    for (markdown, text) in [
        ("> [!DANGER] Unknown tag\n", "[!DANGER] Unknown tag"),
        ("> [!NOTE]glued\n", "[!NOTE]glued"),
        ("> Intro\n> [!NOTE]\n", "Intro\n[!NOTE]"),
        ("> [!] empty\n", "[!] empty"),
    ] {
        assert_eq!(
            golden_blocks(markdown),
            json!([{ "type": "quote", "blocks": [paragraph(text)] }]),
            "{markdown:?}"
        );
    }
}

#[test]
fn wiki_markup_callout_kinds_serialize_snake_case() {
    use crate::services::wiki_markup::WikiCalloutKind;
    for (kind, wire) in [
        (WikiCalloutKind::Note, "note"),
        (WikiCalloutKind::Tip, "tip"),
        (WikiCalloutKind::Important, "important"),
        (WikiCalloutKind::Info, "info"),
        (WikiCalloutKind::Warning, "warning"),
        (WikiCalloutKind::Caution, "caution"),
        (WikiCalloutKind::Critical, "critical"),
    ] {
        assert_eq!(serde_json::to_value(kind).expect("kind serializes"), wire);
    }
}
