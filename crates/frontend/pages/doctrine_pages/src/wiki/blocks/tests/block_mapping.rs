//! Block mapping, driven by the formatting-guide capture: every block kind, heading anchors,
//! checklists, table alignments, the seven callout kinds, quotes, code and rules.

use super::super::callout_style::callout_style;
use super::super::render_tree::tests::{ElementReading, NodeWalking, TagNaming};
use super::super::render_tree::{ElementTag, RenderElement, RenderNode};
use super::super::table_mapping::alignment_class;
use super::*;
use frontend_api_dtos::wiki::{WikiArticle, WikiTableAlignment};
use frontend_test_support::fixtures::golden;

/// The formatting guide as the server answers it.
fn formatting_guide() -> WikiArticle {
    serde_json::from_str(golden!("GET__wiki__wiki-formatting-guide.json")).unwrap()
}

/// The element a node is; panics on a text node.
fn element(node: &RenderNode) -> &RenderElement {
    match node {
        RenderNode::Element(element) => element,
        RenderNode::Text(text) => panic!("expected an element, found the text {text:?}"),
    }
}

/// The element children of `element`.
fn element_children(element: &RenderElement) -> Vec<&RenderElement> {
    element
        .children
        .iter()
        .filter_map(|child| match child {
            RenderNode::Element(inner) => Some(inner),
            RenderNode::Text(_) => None,
        })
        .collect()
}

/// Checks the node one block became against the block, recursing into nested blocks.
fn assert_block_rendered(block: &WikiBlock, node: &RenderNode) {
    let rendered = element(node);
    match block {
        WikiBlock::Heading { level, anchor, .. } => {
            assert_eq!(rendered.tag, heading_tag(*level));
            assert_eq!(rendered.tag.tag_name(), format!("h{level}"));
            assert_eq!(rendered.attribute("id"), Some(anchor.as_str()));
        }
        WikiBlock::Paragraph { .. } => assert_eq!(rendered.tag, ElementTag::Paragraph),
        WikiBlock::List { ordered, items, .. } => {
            let expected = if *ordered {
                ElementTag::NumberedList
            } else {
                ElementTag::BulletList
            };
            assert_eq!(rendered.tag, expected);
            assert_eq!(rendered.children.len(), items.len());
            for (item, item_node) in items.iter().zip(&rendered.children) {
                let li = element(item_node);
                assert_eq!(li.tag, ElementTag::ListItem);
                let content = match item.checked {
                    None => {
                        assert!(
                            li.children.iter().all(|child| !matches!(
                                child,
                                RenderNode::Element(inner) if inner.tag == ElementTag::Checkbox
                            )),
                            "a plain item has no checkbox"
                        );
                        &li.children
                    }
                    Some(_) => &element(&li.children[1]).children,
                };
                for (inner, inner_node) in item.blocks.iter().zip(content) {
                    assert_block_rendered(inner, inner_node);
                }
            }
        }
        WikiBlock::Table { header, rows, .. } => {
            assert_eq!(rendered.tag, ElementTag::Division);
            let table = element_children(rendered)[0];
            assert_eq!(table.tag, ElementTag::Table);
            let sections = element_children(table);
            assert_eq!(sections[0].tag, ElementTag::TableHead);
            assert_eq!(sections[1].tag, ElementTag::TableBody);
            assert_eq!(
                element_children(element_children(sections[0])[0]).len(),
                header.len()
            );
            assert_eq!(element_children(sections[1]).len(), rows.len());
        }
        WikiBlock::Callout { kind, blocks } => {
            let style = callout_style(*kind);
            assert_eq!(rendered.tag, ElementTag::Division);
            assert_eq!(rendered.attribute("class"), Some(style.box_class));
            assert_eq!(rendered.attribute("role"), Some("note"));
            assert_eq!(rendered.children[0].text_content(), style.label);
            let text = element(&rendered.children[1]);
            for (inner, inner_node) in blocks.iter().zip(&text.children) {
                assert_block_rendered(inner, inner_node);
            }
        }
        WikiBlock::Quote { blocks } => {
            assert_eq!(rendered.tag, ElementTag::Blockquote);
            for (inner, inner_node) in blocks.iter().zip(&rendered.children) {
                assert_block_rendered(inner, inner_node);
            }
        }
        WikiBlock::Code { language, text } => {
            assert_eq!(rendered.tag, ElementTag::Preformatted);
            let code = element(&rendered.children[0]);
            assert_eq!(code.tag, ElementTag::Code);
            assert_eq!(code.attribute("data-language"), language.as_deref());
            assert_eq!(code.children, vec![RenderNode::text(text.as_str())]);
        }
        WikiBlock::Rule => {
            assert_eq!(rendered.tag, ElementTag::HorizontalRule);
            assert!(rendered.children.is_empty());
        }
    }
}

#[test]
fn wiki_blocks_formatting_guide_maps_every_block_to_its_element() {
    let article = formatting_guide();
    let nodes = block_nodes(&article.blocks);
    assert_eq!(nodes.len(), article.blocks.len());
    for (block, node) in article.blocks.iter().zip(&nodes) {
        assert_block_rendered(block, node);
    }
    let tags: std::collections::BTreeSet<&str> = nodes
        .iter()
        .flat_map(NodeWalking::elements)
        .map(|element| element.tag.tag_name())
        .collect();
    for tag in [
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "p",
        "ul",
        "li",
        "input",
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        "div",
        "blockquote",
        "pre",
        "code",
        "hr",
        "strong",
        "em",
        "s",
        "a",
        "img",
        "br",
    ] {
        assert!(
            tags.contains(tag),
            "the formatting guide renders no <{tag}>"
        );
    }
}

#[test]
fn wiki_blocks_headings_carry_their_anchor_as_id_so_fragment_links_reach_them() {
    let article = formatting_guide();
    let nodes = block_nodes(&article.blocks);
    let ids: Vec<&str> = nodes
        .iter()
        .flat_map(NodeWalking::elements)
        .filter_map(|element| element.attribute("id"))
        .collect();
    let fragments: Vec<String> = nodes
        .iter()
        .flat_map(NodeWalking::elements)
        .filter_map(|element| element.attribute("href"))
        .filter_map(|href| href.strip_prefix('#'))
        .map(str::to_string)
        .collect();
    assert!(!fragments.is_empty(), "the guide links to its own headings");
    for fragment in fragments {
        assert!(
            ids.contains(&fragment.as_str()),
            "#{fragment} names no heading id"
        );
    }
}

#[test]
fn wiki_blocks_heading_level_is_clamped_and_an_empty_anchor_sets_no_id() {
    assert_eq!(heading_tag(0), ElementTag::Heading1);
    assert_eq!(heading_tag(9), ElementTag::Heading6);
    let node = block_node(&WikiBlock::Heading {
        level: 9,
        anchor: String::new(),
        inlines: vec![WikiInline::Text {
            text: "Deep".into(),
        }],
    });
    assert_eq!(element(&node).tag, ElementTag::Heading6);
    assert_eq!(element(&node).attribute("id"), None);
}

#[test]
fn wiki_blocks_checklist_items_render_disabled_checkboxes_named_by_their_text() {
    let article = formatting_guide();
    let checklist = article
        .blocks
        .iter()
        .find(|block| {
            matches!(block, WikiBlock::List { items, .. } if items.iter().any(|i| i.checked.is_some()))
        })
        .expect("the guide holds a checklist");
    let list = block_node(checklist);
    let boxes: Vec<(bool, String)> = element(&list)
        .children
        .iter()
        .map(|item| {
            let checkbox = element(&element(item).children[0]);
            assert_eq!(checkbox.tag, ElementTag::Checkbox);
            assert_eq!(checkbox.attribute("type"), Some("checkbox"));
            assert_eq!(checkbox.attribute("disabled"), Some(""));
            (
                checkbox.attribute("checked").is_some(),
                checkbox
                    .attribute("aria-label")
                    .unwrap_or_default()
                    .to_string(),
            )
        })
        .collect();
    assert_eq!(
        boxes,
        vec![
            (true, "Radio checked".to_string()),
            (true, "Batteries packed".to_string()),
            (false, "Map marked".to_string()),
        ]
    );
}

#[test]
fn wiki_blocks_numbered_list_keeps_its_start_and_a_bulleted_list_ignores_it() {
    let item = WikiListItem {
        checked: None,
        blocks: vec![WikiBlock::Paragraph {
            inlines: vec![WikiInline::Text {
                text: "Halt".into(),
            }],
        }],
    };
    let numbered = block_node(&WikiBlock::List {
        ordered: true,
        start: Some(3),
        items: vec![item.clone()],
    });
    assert_eq!(element(&numbered).tag, ElementTag::NumberedList);
    assert_eq!(element(&numbered).attribute("start"), Some("3"));
    let bulleted = block_node(&WikiBlock::List {
        ordered: false,
        start: Some(3),
        items: vec![item],
    });
    assert_eq!(element(&bulleted).tag, ElementTag::BulletList);
    assert_eq!(element(&bulleted).attribute("start"), None);
}

#[test]
fn wiki_blocks_table_columns_keep_their_alignments() {
    let article = formatting_guide();
    let table = article
        .blocks
        .iter()
        .find(|block| matches!(block, WikiBlock::Table { .. }))
        .expect("the guide holds a table");
    let node = block_node(table);
    let expected: Vec<&str> = [
        WikiTableAlignment::Left,
        WikiTableAlignment::Center,
        WikiTableAlignment::Right,
        WikiTableAlignment::None,
    ]
    .into_iter()
    .map(alignment_class)
    .collect();
    let rows: Vec<&RenderElement> = node
        .elements()
        .into_iter()
        .filter(|element| element.tag == ElementTag::TableRow)
        .collect();
    assert_eq!(rows.len(), 4, "one header row and three body rows");
    for row in rows {
        let classes: Vec<&str> = element_children(row)
            .iter()
            .map(|cell| {
                let class = cell.attribute("class").unwrap_or_default();
                class.rsplit(' ').next().unwrap_or_default()
            })
            .collect();
        assert_eq!(classes, expected);
    }
    let header = node
        .elements()
        .into_iter()
        .find(|element| element.tag == ElementTag::HeaderCell)
        .unwrap();
    assert_eq!(header.attribute("scope"), Some("col"));
}

#[test]
fn wiki_blocks_a_cell_past_the_alignment_list_starts_at_the_reading_edge() {
    let cell = |text: &str| {
        vec![WikiInline::Text {
            text: text.to_string(),
        }]
    };
    let node = block_node(&WikiBlock::Table {
        alignments: vec![WikiTableAlignment::Right],
        header: vec![cell("A"), cell("B")],
        rows: vec![],
    });
    let cells: Vec<&RenderElement> = node
        .elements()
        .into_iter()
        .filter(|element| element.tag == ElementTag::HeaderCell)
        .collect();
    assert!(
        cells[0]
            .attribute("class")
            .unwrap()
            .ends_with(" text-right")
    );
    assert!(
        cells[1]
            .attribute("class")
            .unwrap()
            .ends_with(" text-start")
    );
}

#[test]
fn wiki_blocks_callout_kinds_keep_three_colour_families_and_seven_labels() {
    let article = formatting_guide();
    let callouts: Vec<(WikiCalloutKind, String)> = article
        .blocks
        .iter()
        .filter_map(|block| match block {
            WikiBlock::Callout { kind, .. } => Some((
                *kind,
                block_node(block).elements()[0].children[0].text_content(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        callouts,
        vec![
            (WikiCalloutKind::Note, "NOTE".to_string()),
            (WikiCalloutKind::Tip, "PRO-TIP".to_string()),
            (WikiCalloutKind::Important, "IMPORTANT".to_string()),
            (WikiCalloutKind::Info, "INFO".to_string()),
            (WikiCalloutKind::Warning, "WARNING".to_string()),
            (WikiCalloutKind::Caution, "CAUTION".to_string()),
            (WikiCalloutKind::Critical, "CRITICAL RULE".to_string()),
        ]
    );
    let family = |kind| callout_style(kind).box_class;
    assert_eq!(family(WikiCalloutKind::Note), family(WikiCalloutKind::Tip));
    assert_eq!(family(WikiCalloutKind::Note), family(WikiCalloutKind::Info));
    assert_eq!(
        family(WikiCalloutKind::Important),
        family(WikiCalloutKind::Warning)
    );
    assert_eq!(
        family(WikiCalloutKind::Caution),
        family(WikiCalloutKind::Critical)
    );
    assert!(family(WikiCalloutKind::Note).contains("border-primary"));
    assert!(family(WikiCalloutKind::Warning).contains("border-tactical-yellow"));
    assert!(family(WikiCalloutKind::Critical).contains("border-error"));
}

#[test]
fn wiki_blocks_field_manual_renders_all_of_its_text() {
    let article: WikiArticle =
        serde_json::from_str(golden!("GET__wiki__field-manual.json")).unwrap();
    let rendered: String = block_nodes(&article.blocks)
        .iter()
        .map(RenderNode::text_content)
        .collect();
    for phrase in [
        "TBD Field Manual",
        "1. Chain of command",
        "Default formation is a staggered column on roads, wedge in the open.",
    ] {
        assert!(
            rendered.contains(phrase),
            "the field manual lost {phrase:?}"
        );
    }
}
