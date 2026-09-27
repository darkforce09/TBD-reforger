//! Refusal goldens: every finding code, its 1-based line, its wording, and the safe form the
//! blocks carry in place of the refused construct.

use serde_json::{Value, json};

use super::{EXCERPT_CHARACTER_LIMIT, SourceLines, WikiMarkupFindingCode};
use crate::community_content::services::wiki_markup::tests::assert_matches_definition;
use crate::community_content::services::wiki_markup::{MAX_NESTING_DEPTH, read_markup};

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
fn wiki_markup_refusal_unsafe_image_without_alt_leaves_nothing() {
    let markdown = "![](javascript:alert(1))\n";
    assert_eq!(
        finding_lines(markdown),
        [(1, WikiMarkupFindingCode::UnsafeImageUrl)]
    );
    assert_eq!(blocks_of(markdown), json!([]));
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

#[test]
fn wiki_markup_refusal_findings_carry_one_based_lines_in_order() {
    let markdown = "fine\n\n[a](data:text/html,x)\n\n![b](http://insecure.example.com/b.png)\n\n\
                    <div>\nx\n</div>\n\ntext <span>y</span>\n";
    assert_eq!(
        finding_lines(markdown),
        [
            (3, WikiMarkupFindingCode::UnsafeLinkUrl),
            (5, WikiMarkupFindingCode::UnsafeImageUrl),
            (7, WikiMarkupFindingCode::RawHtml),
            (11, WikiMarkupFindingCode::RawHtml),
            (11, WikiMarkupFindingCode::RawHtml),
        ]
    );
    assert_eq!(
        blocks_of(markdown),
        json!([
            paragraph("fine"),
            paragraph("a"),
            paragraph("b"),
            paragraph("<div>\nx\n</div>"),
            paragraph("text <span>y</span>"),
        ])
    );
}

#[test]
fn wiki_markup_refusal_details_name_the_construct() {
    let reading = read_markup("[a](javascript:x)\n\n![b](http://h.example/b.png)\n\n<b>c</b>\n");
    let details: Vec<(&str, String)> = reading
        .findings
        .iter()
        .map(|finding| {
            let value = serde_json::to_value(finding).expect("finding serializes");
            (
                match finding.code {
                    WikiMarkupFindingCode::UnsafeLinkUrl => "unsafe_link_url",
                    WikiMarkupFindingCode::UnsafeImageUrl => "unsafe_image_url",
                    WikiMarkupFindingCode::RawHtml => "raw_html",
                    WikiMarkupFindingCode::NestingTooDeep => "nesting_too_deep",
                },
                value["detail"].as_str().expect("detail").to_string(),
            )
        })
        .collect();
    assert_eq!(
        details,
        [
            (
                "unsafe_link_url",
                "link target \"javascript:x\" is not an https, http or mailto URL, a site path \
                 starting with / or a #fragment"
                    .to_string()
            ),
            (
                "unsafe_image_url",
                "image source \"http://h.example/b.png\" is not an https URL or a site path \
                 starting with /"
                    .to_string()
            ),
            (
                "raw_html",
                "raw HTML \"<b>\" is not allowed; write it as markdown".to_string()
            ),
            (
                "raw_html",
                "raw HTML \"</b>\" is not allowed; write it as markdown".to_string()
            ),
        ]
    );
    let wire = serde_json::to_value(&reading.findings[0]).expect("finding serializes");
    assert_eq!(wire["code"], "unsafe_link_url");
    assert_eq!(wire["line"], 1);
}

#[test]
fn wiki_markup_refusal_detail_quotes_a_bounded_excerpt() {
    let target = format!("javascript:{}", "a".repeat(200));
    let reading = read_markup(&format!("[x]({target})\n"));
    let detail = &reading.findings[0].detail;
    let quoted: String = target.chars().take(EXCERPT_CHARACTER_LIMIT).collect();
    assert!(detail.contains(&format!("\"{quoted}…\"")), "{detail}");
    assert!(!detail.contains(&target), "{detail}");
}

/// `levels` nested quotes around a paragraph reading `text`, as the builder keeps them.
fn nested_quotes(levels: usize, text: &str) -> Value {
    (0..levels).fold(
        paragraph(text),
        |inner, _| json!({ "type": "quote", "blocks": [inner] }),
    )
}

#[test]
fn wiki_markup_refusal_quotes_nested_to_the_limit_are_kept() {
    let markdown = format!("{} deep\n", ">".repeat(MAX_NESTING_DEPTH));
    assert_eq!(finding_lines(&markdown), []);
    assert_eq!(
        blocks_of(&markdown),
        json!([nested_quotes(MAX_NESTING_DEPTH, "deep")])
    );
}

#[test]
fn wiki_markup_refusal_quotes_nested_past_the_limit_are_flattened_once() {
    for levels in [MAX_NESTING_DEPTH + 1, MAX_NESTING_DEPTH + 40] {
        let markdown = format!("intro\n\n{} deep\n", ">".repeat(levels));
        assert_eq!(
            finding_lines(&markdown),
            [(3, WikiMarkupFindingCode::NestingTooDeep)],
            "{levels} levels"
        );
        assert_eq!(
            blocks_of(&markdown),
            json!([paragraph("intro"), nested_quotes(MAX_NESTING_DEPTH, "deep")]),
            "{levels} levels"
        );
    }
}

#[test]
fn wiki_markup_refusal_inline_container_past_the_limit_is_flattened() {
    let markdown = format!(
        "{} **bold** and [link](/wiki/x)\n",
        ">".repeat(MAX_NESTING_DEPTH)
    );
    assert_eq!(
        finding_lines(&markdown),
        [
            (1, WikiMarkupFindingCode::NestingTooDeep),
            (1, WikiMarkupFindingCode::NestingTooDeep),
        ]
    );
    assert_eq!(
        blocks_of(&markdown),
        json!([nested_quotes(MAX_NESTING_DEPTH, "bold and link")])
    );
}

#[test]
fn wiki_markup_refusal_lists_nested_past_the_limit_are_flattened() {
    let markdown: String = (0..=MAX_NESTING_DEPTH)
        .map(|level| format!("{}- level{level}\n", "  ".repeat(level)))
        .collect();
    assert_eq!(
        finding_lines(&markdown),
        [(MAX_NESTING_DEPTH + 1, WikiMarkupFindingCode::NestingTooDeep)]
    );
    let deepest = MAX_NESTING_DEPTH - 1;
    let innermost = json!({
        "type": "list",
        "ordered": false,
        "items": [{
            "blocks": [
                paragraph(&format!("level{deepest}")),
                paragraph(&format!("level{MAX_NESTING_DEPTH}")),
            ],
        }],
    });
    let expected = (0..deepest).rev().fold(innermost, |inner, level| {
        json!({
            "type": "list",
            "ordered": false,
            "items": [{ "blocks": [paragraph(&format!("level{level}")), inner] }],
        })
    });
    assert_eq!(blocks_of(&markdown), json!([expected]));
}

#[test]
fn wiki_markup_refusal_flattened_task_item_keeps_its_box_as_text() {
    let mut markdown: String = (0..MAX_NESTING_DEPTH)
        .map(|level| format!("{}- level{level}\n", "  ".repeat(level)))
        .collect();
    markdown.push_str(&format!("{}- [x] ticked\n", "  ".repeat(MAX_NESTING_DEPTH)));
    let blocks = blocks_of(&markdown);
    let rendered = blocks.to_string();
    assert!(rendered.contains("\"[x] ticked\""), "{blocks:#}");
    assert!(!rendered.contains("\"checked\""), "{blocks:#}");
}

#[test]
fn wiki_markup_refusal_every_code_serializes_snake_case() {
    for (code, wire) in [
        (WikiMarkupFindingCode::UnsafeLinkUrl, "unsafe_link_url"),
        (WikiMarkupFindingCode::UnsafeImageUrl, "unsafe_image_url"),
        (WikiMarkupFindingCode::RawHtml, "raw_html"),
        (WikiMarkupFindingCode::NestingTooDeep, "nesting_too_deep"),
    ] {
        assert_eq!(serde_json::to_value(code).expect("code serializes"), wire);
    }
}

#[test]
fn wiki_markup_source_lines_are_one_based_and_count_newlines() {
    let lines = SourceLines::new("ab\ncd\r\n\nef");
    assert_eq!(lines.line_of(0), 1);
    assert_eq!(lines.line_of(2), 1);
    assert_eq!(lines.line_of(3), 2);
    assert_eq!(lines.line_of(6), 2);
    assert_eq!(lines.line_of(7), 3);
    assert_eq!(lines.line_of(8), 4);
    assert_eq!(SourceLines::new("").line_of(0), 1);
}
