//! Callout golden: every callout kind read from a GitHub alert.

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
