//! Anchor goldens: the slug of a heading's text and the de-duplication of a page's anchors.

use serde_json::json;

use super::{EMPTY_HEADING_ANCHOR, HeadingAnchors, slugify};
use crate::services::wiki_markup::tests::golden_blocks;

#[test]
fn wiki_markup_anchor_slug_lowercases_and_joins_words_with_single_hyphens() {
    for (heading, anchor) in [
        ("Radio Procedure", "radio-procedure"),
        ("1. Chain of command", "1-chain-of-command"),
        ("Radio — Nets & Channels!", "radio-nets-channels"),
        ("  padded   heading  ", "padded-heading"),
        ("snake_case and kebab-case", "snake-case-and-kebab-case"),
        ("Straße ÜBER Brücke", "straße-über-brücke"),
        ("M16A2 / M203", "m16a2-m203"),
    ] {
        assert_eq!(slugify(heading), anchor, "{heading:?}");
    }
}

#[test]
fn wiki_markup_anchor_heading_without_letters_or_digits_takes_the_fallback() {
    assert_eq!(slugify(""), EMPTY_HEADING_ANCHOR);
    assert_eq!(slugify("!!! — ???"), EMPTY_HEADING_ANCHOR);
}

#[test]
fn wiki_markup_anchor_repeats_take_numbered_suffixes() {
    let mut anchors = HeadingAnchors::default();
    let claimed: Vec<String> = ["Nets", "Nets", "nets!", "Nets"]
        .into_iter()
        .map(|heading| anchors.claim(heading))
        .collect();
    assert_eq!(claimed, ["nets", "nets-2", "nets-3", "nets-4"]);
}

#[test]
fn wiki_markup_anchor_suffix_skips_an_anchor_an_earlier_heading_holds() {
    let mut anchors = HeadingAnchors::default();
    let claimed: Vec<String> = ["Step", "Step 2", "Step", "Step"]
        .into_iter()
        .map(|heading| anchors.claim(heading))
        .collect();
    assert_eq!(claimed, ["step", "step-2", "step-3", "step-4"]);
}

#[test]
fn wiki_markup_anchor_dedupe_spans_the_whole_page_in_document_order() {
    let blocks = golden_blocks("# Format\n\n> ## Format\n\n- ### Format\n\n#### !!!\n##### ???\n");
    let anchors: Vec<&str> = [
        &blocks[0]["anchor"],
        &blocks[1]["blocks"][0]["anchor"],
        &blocks[2]["items"][0]["blocks"][0]["anchor"],
        &blocks[3]["anchor"],
        &blocks[4]["anchor"],
    ]
    .into_iter()
    .map(|anchor| anchor.as_str().expect("anchor is a string"))
    .collect();
    assert_eq!(
        anchors,
        ["format", "format-2", "format-3", "section", "section-2"]
    );
    assert_eq!(
        blocks[3]["inlines"],
        json!([{ "type": "text", "text": "!!!" }])
    );
}
