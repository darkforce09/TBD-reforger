//! Inline mapping: every inline kind, the link and image safety re-check, and the attributes an
//! external link and an image carry.

use super::super::render_tree::tests::{ElementReading, NodeWalking, TagNaming};
use super::super::render_tree::{ElementTag, RenderNode};
use super::*;
use frontend_api_dtos::wiki::{WikiArticle, WikiBlock};
use frontend_test_support::fixtures::golden;

/// A text inline.
fn text(value: &str) -> WikiInline {
    WikiInline::Text {
        text: value.to_string(),
    }
}

/// Every element of every node in `nodes`.
fn all_elements(nodes: &[RenderNode]) -> Vec<&super::super::render_tree::RenderElement> {
    nodes.iter().flat_map(NodeWalking::elements).collect()
}

/// Link targets and image sources the content URL policy refuses.
const UNSAFE_URLS: &[&str] = &[
    "javascript:alert(1)",
    "JAVASCRIPT:alert(1)",
    "java\tscript:alert(1)",
    " javascript:alert(1)",
    "data:text/html,<script>alert(1)</script>",
    "vbscript:msgbox(1)",
    "file:///etc/passwd",
    "//evil.example/steal",
    "/\\evil.example",
    "https:///evil.example",
    "field-manual",
    "",
];

#[test]
fn wiki_inlines_unsafe_href_renders_its_children_as_plain_text() {
    for href in UNSAFE_URLS {
        for external in [false, true] {
            let nodes = link_nodes(href, external, &[text("Field Manual")]);
            assert_eq!(
                nodes,
                vec![RenderNode::text("Field Manual")],
                "an unsafe href {href:?} must render its children without an anchor"
            );
            assert!(
                all_elements(&nodes)
                    .iter()
                    .all(|element| element.attribute("href").is_none()),
                "no element may carry the unsafe href {href:?}"
            );
        }
    }
}

#[test]
fn wiki_inlines_unsafe_link_keeps_its_nested_formatting() {
    let nodes = link_nodes(
        "javascript:alert(1)",
        false,
        &[WikiInline::Strong {
            children: vec![text("bold")],
        }],
    );
    assert_eq!(nodes.len(), 1);
    let RenderNode::Element(strong) = &nodes[0] else {
        panic!("the strong child must survive as an element");
    };
    assert_eq!(strong.tag, ElementTag::Strong);
    assert_eq!(nodes[0].text_content(), "bold");
}

#[test]
fn wiki_inlines_unsafe_src_renders_the_alt_text() {
    let mut refused: Vec<&str> = UNSAFE_URLS.to_vec();
    // An image is stricter than a link: no plain http, no mailto, no fragment, no bare root.
    refused.extend([
        "http://example.com/map.png",
        "mailto:staff@example.com",
        "#map",
        "/",
    ]);
    for src in refused {
        assert_eq!(
            image_node(src, "Everon grid map", Some("Everon")),
            RenderNode::text("Everon grid map"),
            "an unsafe src {src:?} must render as its alt text"
        );
    }
}

#[test]
fn wiki_inlines_external_link_opens_a_new_tab_without_opener_or_referrer() {
    for (href, external) in [
        ("https://reforger.armaplatform.com", true),
        ("http://example.com/briefing", true),
        ("mailto:staff@example.com", true),
        // The app's own check wins when the server's flag says otherwise.
        ("https://reforger.armaplatform.com", false),
    ] {
        let nodes = link_nodes(href, external, &[text("Arma Reforger")]);
        let RenderNode::Element(anchor) = &nodes[0] else {
            panic!("{href} must render an anchor");
        };
        assert_eq!(anchor.tag, ElementTag::Anchor);
        assert_eq!(anchor.attribute("href"), Some(href));
        assert_eq!(anchor.attribute("target"), Some("_blank"));
        assert_eq!(
            anchor.attribute("rel"),
            Some("noopener noreferrer nofollow")
        );
        assert_eq!(nodes[0].text_content(), "Arma Reforger");
    }
}

#[test]
fn wiki_inlines_site_and_fragment_links_stay_in_the_tab() {
    for href in ["/wiki/field-manual", "#contents", "/"] {
        let nodes = link_nodes(href, false, &[text("here")]);
        let RenderNode::Element(anchor) = &nodes[0] else {
            panic!("{href} must render an anchor");
        };
        assert_eq!(anchor.attribute("href"), Some(href));
        assert_eq!(anchor.attribute("target"), None);
        assert_eq!(anchor.attribute("rel"), None);
    }
}

#[test]
fn wiki_inlines_image_loads_lazily_sends_no_referrer_and_keeps_its_alt_text() {
    let RenderNode::Element(image) = image_node(
        "/uploads/wiki-formatting-guide-map.png",
        "Everon grid map",
        Some("Everon grid map"),
    ) else {
        panic!("a safe image must render an element");
    };
    assert_eq!(image.tag, ElementTag::Image);
    assert!(image.children.is_empty());
    assert_eq!(
        image.attribute("src"),
        Some("/uploads/wiki-formatting-guide-map.png")
    );
    assert_eq!(image.attribute("alt"), Some("Everon grid map"));
    assert_eq!(image.attribute("loading"), Some("lazy"));
    assert_eq!(image.attribute("referrerpolicy"), Some("no-referrer"));
    assert_eq!(image.attribute("title"), Some("Everon grid map"));

    let RenderNode::Element(untitled) = image_node("https://cdn.example.com/a.webp", "", None)
    else {
        panic!("an https image must render an element");
    };
    assert_eq!(untitled.attribute("alt"), Some(""));
    assert_eq!(untitled.attribute("title"), None);
}

#[test]
fn wiki_inlines_markup_in_text_stays_text() {
    let hostile = "<script>alert(1)</script><img src=x onerror=alert(1)>";
    assert_eq!(
        inline_nodes(&[text(hostile)]),
        vec![RenderNode::text(hostile)]
    );
    assert_eq!(
        inline_nodes(&[WikiInline::Code {
            text: hostile.to_string()
        }])[0]
            .text_content(),
        hostile
    );
}

/// Every inline in `blocks`, depth first.
fn collect_inlines<'a>(blocks: &'a [WikiBlock], out: &mut Vec<&'a WikiInline>) {
    fn walk<'a>(inlines: &'a [WikiInline], out: &mut Vec<&'a WikiInline>) {
        for inline in inlines {
            out.push(inline);
            match inline {
                WikiInline::Strong { children }
                | WikiInline::Emphasis { children }
                | WikiInline::Strikethrough { children }
                | WikiInline::Link { children, .. } => walk(children, out),
                _ => {}
            }
        }
    }
    for block in blocks {
        match block {
            WikiBlock::Heading { inlines, .. } | WikiBlock::Paragraph { inlines } => {
                walk(inlines, out)
            }
            WikiBlock::List { items, .. } => {
                for item in items {
                    collect_inlines(&item.blocks, out);
                }
            }
            WikiBlock::Table { header, rows, .. } => {
                for cell in header.iter().chain(rows.iter().flatten()) {
                    walk(cell, out);
                }
            }
            WikiBlock::Callout { blocks, .. } | WikiBlock::Quote { blocks } => {
                collect_inlines(blocks, out)
            }
            WikiBlock::Code { .. } | WikiBlock::Rule => {}
        }
    }
}

#[test]
fn wiki_inlines_formatting_guide_maps_every_inline_kind() {
    let article: WikiArticle =
        serde_json::from_str(golden!("GET__wiki__wiki-formatting-guide.json")).unwrap();
    let mut inlines = Vec::new();
    collect_inlines(&article.blocks, &mut inlines);
    let mut seen = std::collections::BTreeSet::new();
    for inline in inlines {
        let nodes = inline_nodes(std::slice::from_ref(inline));
        let first = nodes.first();
        let (kind, expected): (&str, Option<ElementTag>) = match inline {
            WikiInline::Text { .. } => ("text", None),
            WikiInline::Strong { .. } => ("strong", Some(ElementTag::Strong)),
            WikiInline::Emphasis { .. } => ("emphasis", Some(ElementTag::Emphasis)),
            WikiInline::Strikethrough { .. } => ("strikethrough", Some(ElementTag::Strikethrough)),
            WikiInline::Code { .. } => ("code", Some(ElementTag::Code)),
            WikiInline::Link { .. } => ("link", Some(ElementTag::Anchor)),
            WikiInline::Image { .. } => ("image", Some(ElementTag::Image)),
            WikiInline::LineBreak => ("line_break", Some(ElementTag::LineBreak)),
        };
        seen.insert(kind);
        match (expected, first) {
            (None, Some(RenderNode::Text(_))) => {}
            (Some(tag), Some(RenderNode::Element(element))) => {
                assert_eq!(element.tag, tag, "{kind} renders as {}", tag.tag_name())
            }
            other => panic!("{kind} rendered unexpectedly: {other:?}"),
        }
    }
    for kind in [
        "text",
        "strong",
        "emphasis",
        "strikethrough",
        "code",
        "link",
        "image",
        "line_break",
    ] {
        assert!(
            seen.contains(kind),
            "the golden no longer shows a {kind} inline"
        );
    }
}
