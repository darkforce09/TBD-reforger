//! Inline goldens: styled runs and links as pinned JSON, each read through the public entry point
//! and checked against the contract.

use serde_json::{Value, json};

use crate::services::wiki_markup::tests::golden_blocks;

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
