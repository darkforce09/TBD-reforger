//! Render nodes as Leptos views: one element builder per tag, text as text nodes.
//!
//! **Role:** turns the render tree the mappers build into views — each element through the
//! Leptos builder of its tag, with its attributes set one by one, and each text node as a DOM
//! text node.
//! **Position:** the last step of the block renderer, called by the module's entry with the
//! mapped nodes of an article or a revision.
//! **Signals & state:** none; the views it returns are static.
//! **Invariants:** content is never parsed as HTML: text becomes a text node and an attribute is
//! set by name, and no element here receives inner HTML or an event handler. A void element
//! (`input`, `hr`, `img`, `br`) is built without children.

use super::render_tree::{ElementTag, RenderElement, RenderNode};
use leptos::html;
use leptos::prelude::*;
use leptos::tachys::html::attribute::any_attribute::AnyAttribute;
use leptos::tachys::html::attribute::custom::custom_attribute;

/// The views of `nodes`, in order.
pub(super) fn node_views(nodes: Vec<RenderNode>) -> Vec<AnyView> {
    nodes.into_iter().map(node_view).collect()
}

/// The view of one node.
fn node_view(node: RenderNode) -> AnyView {
    match node {
        RenderNode::Text(text) => text.into_any(),
        RenderNode::Element(element) => element_view(element),
    }
}

/// The view of one element: its tag's builder, its attributes, and its children unless void.
fn element_view(element: RenderElement) -> AnyView {
    let RenderElement {
        tag,
        attributes,
        children,
    } = element;
    let attributes: Vec<AnyAttribute> = attributes
        .into_iter()
        .map(|(name, value)| custom_attribute(name, value).into_any_attr())
        .collect();
    let children = node_views(children);
    // One arm per tag: the builders are distinct types, so each arm finishes its own element.
    macro_rules! container {
        ($builder:expr) => {
            $builder.add_any_attr(attributes).child(children).into_any()
        };
    }
    macro_rules! void {
        ($builder:expr) => {{
            let _ = children;
            $builder.add_any_attr(attributes).into_any()
        }};
    }
    match tag {
        ElementTag::Heading1 => container!(html::h1()),
        ElementTag::Heading2 => container!(html::h2()),
        ElementTag::Heading3 => container!(html::h3()),
        ElementTag::Heading4 => container!(html::h4()),
        ElementTag::Heading5 => container!(html::h5()),
        ElementTag::Heading6 => container!(html::h6()),
        ElementTag::Paragraph => container!(html::p()),
        ElementTag::BulletList => container!(html::ul()),
        ElementTag::NumberedList => container!(html::ol()),
        ElementTag::ListItem => container!(html::li()),
        ElementTag::Checkbox => void!(html::input()),
        ElementTag::Table => container!(html::table()),
        ElementTag::TableHead => container!(html::thead()),
        ElementTag::TableBody => container!(html::tbody()),
        ElementTag::TableRow => container!(html::tr()),
        ElementTag::HeaderCell => container!(html::th()),
        ElementTag::DataCell => container!(html::td()),
        ElementTag::Division => container!(html::div()),
        ElementTag::Blockquote => container!(html::blockquote()),
        ElementTag::Preformatted => container!(html::pre()),
        ElementTag::Code => container!(html::code()),
        ElementTag::HorizontalRule => void!(html::hr()),
        ElementTag::Strong => container!(html::strong()),
        ElementTag::Emphasis => container!(html::em()),
        ElementTag::Strikethrough => container!(html::s()),
        ElementTag::Anchor => container!(html::a()),
        ElementTag::Image => void!(html::img()),
        ElementTag::LineBreak => void!(html::br()),
    }
}
