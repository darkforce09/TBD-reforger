//! Inline goldens: one pinned JSON run per inline type the tree builder produces, each read
//! through the public entry point and checked against the contract.

use serde_json::{Value, json};

use crate::community_content::services::wiki_markup::tests::golden_blocks;

/// The inlines of the single paragraph `markdown` reads to.
fn paragraph_inlines(markdown: &str) -> Value {
    let blocks = golden_blocks(markdown);
    let [block] = blocks.as_array().expect("blocks array").as_slice() else {
        panic!("{markdown:?} did not read to exactly one block: {blocks:#}");
    };
    assert_eq!(block["type"], "paragraph", "{blocks:#}");
    block["inlines"].clone()
}

/// One text run.
fn text(text: &str) -> Value {
    json!({ "type": "text", "text": text })
}

#[test]
fn wiki_markup_golden_inline_text_merges_adjacent_runs() {
    assert_eq!(
        paragraph_inlines("An [unmatched bracket and AT&amp;T \\*escaped\\*\n"),
        json!([text("An [unmatched bracket and AT&T *escaped*")])
    );
}

#[test]
fn wiki_markup_golden_inline_strong_emphasis_and_strikethrough() {
    assert_eq!(
        paragraph_inlines("**strong** *emphasis* ~~struck~~\n"),
        json!([
            { "type": "strong", "children": [text("strong")] },
            text(" "),
            { "type": "emphasis", "children": [text("emphasis")] },
            text(" "),
            { "type": "strikethrough", "children": [text("struck")] },
        ])
    );
}

#[test]
fn wiki_markup_golden_inline_styles_nest() {
    assert_eq!(
        paragraph_inlines("***both***\n"),
        json!([{
            "type": "emphasis",
            "children": [{ "type": "strong", "children": [text("both")] }],
        }])
    );
}

#[test]
fn wiki_markup_golden_inline_code() {
    assert_eq!(
        paragraph_inlines("Say `CONTACT — direction` first\n"),
        json!([
            text("Say "),
            { "type": "code", "text": "CONTACT — direction" },
            text(" first"),
        ])
    );
}

#[test]
fn wiki_markup_golden_inline_links_internal_external_and_fragment() {
    let inlines = paragraph_inlines(
        "[manual](/wiki/field-manual) [site](https://example.com/a) [plain](http://example.com) \
         [nets](#nets) [mail](mailto:hq@example.com)\n",
    );
    let link = |href: &str, external: bool, label: &str| {
        let children = json!([text(label)]);
        json!({ "type": "link", "href": href, "external": external, "children": children })
    };
    assert_eq!(
        inlines,
        json!([
            link("/wiki/field-manual", false, "manual"),
            text(" "),
            link("https://example.com/a", true, "site"),
            text(" "),
            link("http://example.com", true, "plain"),
            text(" "),
            link("#nets", false, "nets"),
            text(" "),
            link("mailto:hq@example.com", true, "mail"),
        ])
    );
}

#[test]
fn wiki_markup_golden_inline_autolinks_gain_their_scheme() {
    assert_eq!(
        paragraph_inlines("<https://auto.example.com> <ops@example.com>\n"),
        json!([
            {
                "type": "link",
                "href": "https://auto.example.com",
                "external": true,
                "children": [text("https://auto.example.com")],
            },
            text(" "),
            {
                "type": "link",
                "href": "mailto:ops@example.com",
                "external": true,
                "children": [text("ops@example.com")],
            },
        ])
    );
}

#[test]
fn wiki_markup_golden_inline_link_children_keep_their_markup() {
    assert_eq!(
        paragraph_inlines("[**Radio** procedure](/wiki/radio-procedure)\n"),
        json!([{
            "type": "link",
            "href": "/wiki/radio-procedure",
            "external": false,
            "children": [
                { "type": "strong", "children": [text("Radio")] },
                text(" procedure"),
            ],
        }])
    );
}

#[test]
fn wiki_markup_golden_inline_image_with_and_without_title() {
    assert_eq!(
        paragraph_inlines(
            "![Map of *Everon*](/uploads/map.png \"Everon\") ![](https://cdn.example.com/a.png)\n"
        ),
        json!([
            {
                "type": "image",
                "src": "/uploads/map.png",
                "alt": "Map of Everon",
                "title": "Everon",
            },
            text(" "),
            { "type": "image", "src": "https://cdn.example.com/a.png", "alt": "" },
        ])
    );
}

#[test]
fn wiki_markup_golden_inline_image_inside_a_link() {
    assert_eq!(
        paragraph_inlines("[![Badge](/uploads/badge.png)](/wiki/server-rules)\n"),
        json!([{
            "type": "link",
            "href": "/wiki/server-rules",
            "external": false,
            "children": [{ "type": "image", "src": "/uploads/badge.png", "alt": "Badge" }],
        }])
    );
}

#[test]
fn wiki_markup_golden_inline_hard_line_breaks() {
    assert_eq!(
        paragraph_inlines("one  \ntwo\\\nthree\n"),
        json!([
            text("one"),
            { "type": "line_break" },
            text("two"),
            { "type": "line_break" },
            text("three"),
        ])
    );
}

#[test]
fn wiki_markup_golden_heading_inlines_and_anchor() {
    let blocks = golden_blocks("## Head `code` *em*\n");
    assert_eq!(
        blocks,
        json!([{
            "type": "heading",
            "level": 2,
            "anchor": "head-code-em",
            "inlines": [
                text("Head "),
                { "type": "code", "text": "code" },
                text(" "),
                { "type": "emphasis", "children": [text("em")] },
            ],
        }])
    );
}

#[test]
fn wiki_markup_golden_disabled_extensions_stay_text() {
    assert_eq!(
        paragraph_inlines("Footnote[^1] and $math$ and ^sup^\n"),
        json!([text("Footnote[^1] and $math$ and ^sup^")])
    );
}
