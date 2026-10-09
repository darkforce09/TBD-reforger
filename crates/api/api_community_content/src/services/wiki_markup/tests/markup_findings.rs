//! Refusal goldens: unsafe links and images and raw HTML, the finding each refusal reports and
//! the safe form the blocks carry in place of the refused construct.

use serde_json::{Value, json};

use super::WikiMarkupFindingCode;
use crate::services::wiki_markup::read_markup;
use crate::services::wiki_markup::tests::assert_matches_definition;

/// The findings of `markdown` as `(line, code)` pairs, each finding checked against the
/// contract.
fn finding_lines(markdown: &str) -> Vec<(usize, WikiMarkupFindingCode)> {
    let reading = read_markup(markdown);
    for finding in &reading.findings {
        let value = serde_json::to_value(finding).expect("finding serializes");
        assert_matches_definition("WikiMarkupFinding", &value);
    }
    reading
        .findings
        .iter()
        .map(|finding| (finding.line, finding.code))
        .collect()
}

/// The blocks of `markdown` as JSON, each checked against the contract.
fn blocks_of(markdown: &str) -> Value {
    let blocks = serde_json::to_value(read_markup(markdown).blocks).expect("blocks serialize");
    for block in blocks.as_array().expect("blocks array") {
        assert_matches_definition("WikiBlock", block);
    }
    blocks
}

/// A paragraph holding one text run.
fn paragraph(text: &str) -> Value {
    json!({ "type": "paragraph", "inlines": [{ "type": "text", "text": text }] })
}

#[test]
fn wiki_markup_refusal_unsafe_link_keeps_only_its_children() {
    let markdown = "Open [the **menu**](javascript:alert(1)) now\n";
    assert_eq!(
        finding_lines(markdown),
        [(1, WikiMarkupFindingCode::UnsafeLinkUrl)]
    );
    assert_eq!(
        blocks_of(markdown),
        json!([{
            "type": "paragraph",
            "inlines": [
                { "type": "text", "text": "Open the " },
                { "type": "strong", "children": [{ "type": "text", "text": "menu" }] },
                { "type": "text", "text": " now" },
            ],
        }])
    );
}

#[test]
fn wiki_markup_refusal_unsafe_link_targets_of_every_shape() {
    for target in [
        "data:text/html,x",
        "vbscript:msgbox",
        "file:///etc/passwd",
        "//evil.example.com/x",
        "field-manual",
        "HTTPS://example.com",
        "&#106;avascript:alert(1)",
        "<java script:alert(1)>",
        "",
    ] {
        let markdown = format!("[label]({target})\n");
        assert_eq!(
            finding_lines(&markdown),
            [(1, WikiMarkupFindingCode::UnsafeLinkUrl)],
            "{markdown:?}"
        );
        assert_eq!(
            blocks_of(&markdown),
            json!([paragraph("label")]),
            "{markdown:?}"
        );
    }
}

#[test]
fn wiki_markup_refusal_unsafe_image_becomes_its_alt_text() {
    for source in [
        "http://insecure.example.com/b.png",
        "//evil.example.com/c.png",
        "javascript:alert(1)",
        "#fragment",
        "uploads/relative.png",
    ] {
        let markdown = format!("![Range card]({source})\n");
        assert_eq!(
            finding_lines(&markdown),
            [(1, WikiMarkupFindingCode::UnsafeImageUrl)],
            "{markdown:?}"
        );
        assert_eq!(
            blocks_of(&markdown),
            json!([paragraph("Range card")]),
            "{markdown:?}"
        );
    }
}

#[test]
fn wiki_markup_refusal_raw_html_inline_and_block_stay_literal_text() {
    let markdown = "Text <span class=\"x\">y</span> <!-- note -->\n\n<div>\nblock\n</div>\n";
    assert_eq!(
        finding_lines(markdown),
        [
            (1, WikiMarkupFindingCode::RawHtml),
            (1, WikiMarkupFindingCode::RawHtml),
            (1, WikiMarkupFindingCode::RawHtml),
            (3, WikiMarkupFindingCode::RawHtml),
        ]
    );
    assert_eq!(
        blocks_of(markdown),
        json!([
            paragraph("Text <span class=\"x\">y</span> <!-- note -->"),
            paragraph("<div>\nblock\n</div>"),
        ])
    );
}
