//! The tree the wiki renderer builds before any view exists: elements, their attributes, text.
//!
//! **Role:** the plain-data description of a rendered manual — which element each block and
//! inline becomes, which attributes it carries and what it holds — so the rendering decisions
//! (tags, classes, link and image safety) are made and tested without a browser.
//! **Position:** built by the block, table and inline mappers of this module from the article's
//! typed [`WikiBlock`](crate::foundation::transport::dto::wiki::WikiBlock) tree; turned into Leptos views
//! by the element views; read directly by the unit tests.
//! **Signals & state:** none; plain data.
//! **Invariants:** authored content reaches the page only as a [`RenderNode::Text`], which becomes
//! a DOM text node, or as an attribute value. Attribute names are literals written by the
//! mappers, never content, and every tag comes from the closed [`ElementTag`] set, so no node can
//! carry markup or a handler.

/// The HTML elements a manual renders to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ElementTag {
    /// `<h1>`.
    Heading1,
    /// `<h2>`.
    Heading2,
    /// `<h3>`.
    Heading3,
    /// `<h4>`.
    Heading4,
    /// `<h5>`.
    Heading5,
    /// `<h6>`.
    Heading6,
    /// `<p>`.
    Paragraph,
    /// `<ul>`.
    BulletList,
    /// `<ol>`.
    NumberedList,
    /// `<li>`.
    ListItem,
    /// `<input type="checkbox">`, a void element.
    Checkbox,
    /// `<table>`.
    Table,
    /// `<thead>`.
    TableHead,
    /// `<tbody>`.
    TableBody,
    /// `<tr>`.
    TableRow,
    /// `<th>`.
    HeaderCell,
    /// `<td>`.
    DataCell,
    /// `<div>`.
    Division,
    /// `<blockquote>`.
    Blockquote,
    /// `<pre>`.
    Preformatted,
    /// `<code>`.
    Code,
    /// `<hr>`, a void element.
    HorizontalRule,
    /// `<strong>`.
    Strong,
    /// `<em>`.
    Emphasis,
    /// `<s>`.
    Strikethrough,
    /// `<a>`.
    Anchor,
    /// `<img>`, a void element.
    Image,
    /// `<br>`, a void element.
    LineBreak,
}

/// One node of a rendered manual: text, or an element holding more nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RenderNode {
    /// Text shown exactly as written; it becomes a DOM text node.
    Text(String),
    /// An element with its attributes and children.
    Element(RenderElement),
}

/// An element: its tag, its attributes in the order they are set, and its children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RenderElement {
    /// Which element this is.
    pub(super) tag: ElementTag,
    /// `(name, value)` pairs; a present boolean attribute carries the empty string.
    pub(super) attributes: Vec<(&'static str, String)>,
    /// The nodes inside the element; always empty for a void element.
    pub(super) children: Vec<RenderNode>,
}

impl RenderElement {
    /// An element of `tag` whose `class` attribute is `class`; an empty `class` sets none.
    pub(super) fn new(tag: ElementTag, class: &str) -> Self {
        let attributes = if class.is_empty() {
            Vec::new()
        } else {
            vec![("class", class.to_string())]
        };
        Self {
            tag,
            attributes,
            children: Vec::new(),
        }
    }

    /// The element with one more attribute set.
    pub(super) fn with_attribute(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.attributes.push((name, value.into()));
        self
    }

    /// The element holding `children`.
    pub(super) fn with_children(mut self, children: Vec<RenderNode>) -> Self {
        self.children = children;
        self
    }

    /// The element as a node.
    pub(super) fn into_node(self) -> RenderNode {
        RenderNode::Element(self)
    }
}

impl RenderNode {
    /// A text node holding `text`.
    pub(super) fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Every piece of text under this node, joined in document order.
    pub(super) fn text_content(&self) -> String {
        let mut out = String::new();
        self.push_text(&mut out);
        out
    }

    /// Appends the text under this node to `out`.
    fn push_text(&self, out: &mut String) {
        match self {
            Self::Text(text) => out.push_str(text),
            Self::Element(element) => {
                for child in &element.children {
                    child.push_text(out);
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/render_tree.rs"]
pub(super) mod tests;
