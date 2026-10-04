//! Readers the mapping tests share over the render tree — an element's attribute, every element
//! of a subtree, a tag's HTML name — and checks of the tree's own builders.

use super::*;

/// Reads an element's attributes.
pub(in super::super) trait ElementReading {
    /// The value of the attribute `name`, when the element sets it.
    fn attribute(&self, name: &str) -> Option<&str>;
}

impl ElementReading for RenderElement {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Walks a subtree.
pub(in super::super) trait NodeWalking {
    /// Every element in this subtree, this node first, depth first.
    fn elements(&self) -> Vec<&RenderElement>;
}

impl NodeWalking for RenderNode {
    fn elements(&self) -> Vec<&RenderElement> {
        fn walk<'a>(node: &'a RenderNode, out: &mut Vec<&'a RenderElement>) {
            if let RenderNode::Element(element) = node {
                out.push(element);
                for child in &element.children {
                    walk(child, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(self, &mut out);
        out
    }
}

/// Names a tag.
pub(in super::super) trait TagNaming {
    /// The HTML tag name, as the browser spells it.
    fn tag_name(self) -> &'static str;
}

impl TagNaming for ElementTag {
    fn tag_name(self) -> &'static str {
        match self {
            Self::Heading1 => "h1",
            Self::Heading2 => "h2",
            Self::Heading3 => "h3",
            Self::Heading4 => "h4",
            Self::Heading5 => "h5",
            Self::Heading6 => "h6",
            Self::Paragraph => "p",
            Self::BulletList => "ul",
            Self::NumberedList => "ol",
            Self::ListItem => "li",
            Self::Checkbox => "input",
            Self::Table => "table",
            Self::TableHead => "thead",
            Self::TableBody => "tbody",
            Self::TableRow => "tr",
            Self::HeaderCell => "th",
            Self::DataCell => "td",
            Self::Division => "div",
            Self::Blockquote => "blockquote",
            Self::Preformatted => "pre",
            Self::Code => "code",
            Self::HorizontalRule => "hr",
            Self::Strong => "strong",
            Self::Emphasis => "em",
            Self::Strikethrough => "s",
            Self::Anchor => "a",
            Self::Image => "img",
            Self::LineBreak => "br",
        }
    }
}

#[test]
fn wiki_render_tree_class_is_the_first_attribute_and_an_empty_class_sets_none() {
    let with_class = RenderElement::new(ElementTag::Paragraph, "mt-3");
    assert_eq!(with_class.attributes, vec![("class", "mt-3".to_string())]);
    assert!(
        RenderElement::new(ElementTag::LineBreak, "")
            .attributes
            .is_empty()
    );
}

#[test]
fn wiki_render_tree_text_content_joins_nested_text_in_document_order() {
    let node = RenderElement::new(ElementTag::Paragraph, "")
        .with_children(vec![
            RenderNode::text("Write "),
            RenderElement::new(ElementTag::Strong, "")
                .with_children(vec![RenderNode::text("bold")])
                .into_node(),
            RenderNode::text(" text"),
        ])
        .into_node();
    assert_eq!(node.text_content(), "Write bold text");
    assert_eq!(node.elements().len(), 2);
}
