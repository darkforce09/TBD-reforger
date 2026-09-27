//! Shape checks for the wiki payloads beyond the captured round trips in `r_api_content.rs`: the
//! save body the app sends, the refusal details it reads back, a single revision, and proof that
//! the formatting-guide capture exercises every block, inline, callout kind and alignment.

use std::collections::BTreeSet;

use serde_json::json;

use super::*;
use crate::v2::core::test_support::fixtures::golden;

#[test]
fn a_create_sends_base_revision_as_an_explicit_null() {
    let body = WikiSaveRequest {
        category: "Doctrine".into(),
        title: "Convoy Drills".into(),
        icon: String::new(),
        nav_order: 4,
        body_md: "# Convoy Drills".into(),
        base_revision: None,
    };
    assert_eq!(
        serde_json::to_value(&body).unwrap(),
        json!({
            "category": "Doctrine",
            "title": "Convoy Drills",
            "icon": "",
            "nav_order": 4,
            "body_md": "# Convoy Drills",
            "base_revision": null
        })
    );
}

#[test]
fn an_update_sends_the_revision_it_starts_from() {
    let body = WikiSaveRequest {
        category: "Doctrine".into(),
        title: "Field Manual".into(),
        icon: "menu_book".into(),
        nav_order: 1,
        body_md: "# TBD Field Manual".into(),
        base_revision: Some(3),
    };
    let sent = serde_json::to_value(&body).unwrap();
    assert_eq!(sent["base_revision"], json!(3));
    assert_eq!(
        sent.as_object().unwrap().len(),
        6,
        "the save names exactly its six fields"
    );
}

#[test]
fn a_revision_conflict_names_the_current_revision() {
    let details: WikiSaveRefusal =
        serde_json::from_str(r#"{"code":"wiki_revision_conflict","current_revision":5}"#).unwrap();
    assert_eq!(details.code, WikiSaveRefusalCode::RevisionConflict);
    assert_eq!(details.current_revision, Some(5));
    assert!(details.findings.is_none());
}

#[test]
fn a_body_too_large_refusal_carries_only_its_code() {
    const DETAILS: &str = r#"{"code":"wiki_body_too_large"}"#;
    let details: WikiSaveRefusal = serde_json::from_str(DETAILS).unwrap();
    assert_eq!(details.code, WikiSaveRefusalCode::BodyTooLarge);
    assert_eq!(serde_json::to_string(&details).unwrap(), DETAILS);
}

#[test]
fn a_markup_refusal_lists_every_finding_code() {
    const DETAILS: &str = r#"{"code":"wiki_markup_refused","findings":[
        {"line":3,"code":"unsafe_link_url","detail":"link target \"javascript:alert(1)\" is refused"},
        {"line":5,"code":"unsafe_image_url","detail":"image source \"http://x/a.png\" is refused"},
        {"line":8,"code":"raw_html","detail":"raw HTML \"<b>\" is not allowed"},
        {"line":21,"code":"nesting_too_deep","detail":"nesting deeper than 16"}
    ]}"#;
    let details: WikiSaveRefusal = serde_json::from_str(DETAILS).unwrap();
    assert_eq!(details.code, WikiSaveRefusalCode::MarkupRefused);
    let codes: Vec<WikiMarkupFindingCode> = details
        .findings
        .expect("a markup refusal carries its findings")
        .iter()
        .map(|finding| finding.code)
        .collect();
    assert_eq!(
        codes,
        [
            WikiMarkupFindingCode::UnsafeLinkUrl,
            WikiMarkupFindingCode::UnsafeImageUrl,
            WikiMarkupFindingCode::RawHtml,
            WikiMarkupFindingCode::NestingTooDeep,
        ]
    );
}

/// Every variant name the typed tree of `blocks` holds, by an exhaustive match, so a variant
/// added to the DTO must be added here and then shown by the capture.
fn variant_names(blocks: &[WikiBlock], found: &mut BTreeSet<String>) {
    for block in blocks {
        match block {
            WikiBlock::Heading { inlines, .. } => {
                found.insert("block:heading".into());
                inline_names(inlines, found);
            }
            WikiBlock::Paragraph { inlines } => {
                found.insert("block:paragraph".into());
                inline_names(inlines, found);
            }
            WikiBlock::List { items, .. } => {
                found.insert("block:list".into());
                for item in items {
                    if let Some(checked) = item.checked {
                        found.insert(format!("task:{checked}"));
                    }
                    variant_names(&item.blocks, found);
                }
            }
            WikiBlock::Table {
                alignments,
                header,
                rows,
            } => {
                found.insert("block:table".into());
                for alignment in alignments {
                    found.insert(format!("align:{alignment:?}"));
                }
                for cell in header.iter().chain(rows.iter().flatten()) {
                    inline_names(cell, found);
                }
            }
            WikiBlock::Callout { kind, blocks } => {
                found.insert("block:callout".into());
                found.insert(format!("callout:{kind:?}"));
                variant_names(blocks, found);
            }
            WikiBlock::Quote { blocks } => {
                found.insert("block:quote".into());
                variant_names(blocks, found);
            }
            WikiBlock::Code { language, .. } => {
                found.insert(format!("block:code:{}", language.is_some()));
            }
            WikiBlock::Rule => {
                found.insert("block:rule".into());
            }
        }
    }
}

fn inline_names(inlines: &[WikiInline], found: &mut BTreeSet<String>) {
    for inline in inlines {
        let name = match inline {
            WikiInline::Text { .. } => "text",
            WikiInline::Strong { children } => {
                inline_names(children, found);
                "strong"
            }
            WikiInline::Emphasis { children } => {
                inline_names(children, found);
                "emphasis"
            }
            WikiInline::Strikethrough { children } => {
                inline_names(children, found);
                "strikethrough"
            }
            WikiInline::Code { .. } => "code",
            WikiInline::Link {
                external, children, ..
            } => {
                inline_names(children, found);
                found.insert(format!("inline:link:external:{external}"));
                "link"
            }
            WikiInline::Image { title, .. } => {
                found.insert(format!("inline:image:title:{}", title.is_some()));
                "image"
            }
            WikiInline::LineBreak => "line_break",
        };
        found.insert(format!("inline:{name}"));
    }
}

#[test]
fn the_formatting_guide_capture_shows_every_construct_the_tree_can_carry() {
    let article: WikiArticle =
        serde_json::from_str(golden!("GET__wiki__wiki-formatting-guide.json")).unwrap();
    let mut found = BTreeSet::new();
    variant_names(&article.blocks, &mut found);
    let required = [
        "block:heading",
        "block:paragraph",
        "block:list",
        "block:table",
        "block:callout",
        "block:quote",
        "block:code:true",
        "block:rule",
        "task:true",
        "task:false",
        "align:None",
        "align:Left",
        "align:Center",
        "align:Right",
        "callout:Note",
        "callout:Tip",
        "callout:Important",
        "callout:Info",
        "callout:Warning",
        "callout:Caution",
        "callout:Critical",
        "inline:text",
        "inline:strong",
        "inline:emphasis",
        "inline:strikethrough",
        "inline:code",
        "inline:link",
        "inline:link:external:true",
        "inline:link:external:false",
        "inline:image",
        "inline:line_break",
    ];
    let missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|name| !found.contains(*name))
        .collect();
    assert!(
        missing.is_empty(),
        "the formatting-guide capture no longer shows {missing:?}; found {found:?}"
    );
}
