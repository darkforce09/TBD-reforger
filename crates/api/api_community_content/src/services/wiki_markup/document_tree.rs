//! The typed tree of a wiki page: the blocks and inlines the markup service builds from markdown.
//!
//! **Role:** the one shape of a parsed wiki page, serialized as the `blocks` of an article or a
//! revision.
//! **Position:** built by [`super::read_markup`]; embedded in
//! [`crate::models::wiki::WikiArticle`] and
//! [`crate::models::wiki::WikiRevision`], which the doctrine wiki page of the
//! single-page app renders node by node.
//! **Signals & state:** none; plain data.
//! **Invariants:** every node serializes as an object tagged by a snake_case `type`; optional
//! fields (`start`, `checked`, `language`, `title`) are omitted when absent, never `null`; a
//! link `href` and an image `src` always pass the content URL policy, because the builder never
//! produces an unsafe one.
//! @contract wiki-page.schema.json#/definitions/WikiBlock
//! @contract wiki-page.schema.json#/definitions/WikiInline

use serde::{Deserialize, Serialize};

/// One block of a page.
/// @contract wiki-page.schema.json#/definitions/WikiBlock
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum WikiBlock {
    /// A heading of level 1 to 6; `anchor` is unique within the page.
    Heading {
        /// The heading level, 1 to 6.
        level: u8,
        /// The page-unique fragment id the heading links as.
        anchor: String,
        /// The heading text.
        inlines: Vec<WikiInline>,
    },
    /// A paragraph.
    Paragraph {
        /// The paragraph text.
        inlines: Vec<WikiInline>,
    },
    /// A bulleted list (`ordered` false, no `start`) or a numbered one, which carries the
    /// number of its first item as `start`.
    List {
        /// Whether the items are numbered.
        ordered: bool,
        /// The number of the first item of a numbered list.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<u64>,
        /// The items, in order.
        items: Vec<WikiListItem>,
    },
    /// A table: one alignment per column, the header cells and the body rows of cells; a cell
    /// is a list of inlines.
    Table {
        /// One alignment per column.
        alignments: Vec<WikiTableAlignment>,
        /// The header cells.
        header: Vec<Vec<WikiInline>>,
        /// The body rows, each a list of cells.
        rows: Vec<Vec<Vec<WikiInline>>>,
    },
    /// A callout box of one [`WikiCalloutKind`].
    Callout {
        /// The callout's kind.
        kind: WikiCalloutKind,
        /// The callout's content.
        blocks: Vec<WikiBlock>,
    },
    /// A block quote that carries no callout marker.
    Quote {
        /// The quoted content.
        blocks: Vec<WikiBlock>,
    },
    /// A code block; `language` is the fence's trimmed info string, absent when it has none.
    Code {
        /// The fence's trimmed info string.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        /// The code, verbatim.
        text: String,
    },
    /// A thematic break.
    Rule,
}

/// One inline run.
/// @contract wiki-page.schema.json#/definitions/WikiInline
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum WikiInline {
    /// Plain text; a soft line break inside a paragraph is a `\n` in the text.
    Text {
        /// The text.
        text: String,
    },
    /// Strong emphasis.
    Strong {
        /// The emphasised runs.
        children: Vec<WikiInline>,
    },
    /// Emphasis.
    Emphasis {
        /// The emphasised runs.
        children: Vec<WikiInline>,
    },
    /// Struck-through text.
    Strikethrough {
        /// The struck-through runs.
        children: Vec<WikiInline>,
    },
    /// Inline code.
    Code {
        /// The code, verbatim.
        text: String,
    },
    /// A link to a safe target; `external` is true for an absolute `http`, `https` or `mailto`
    /// target.
    Link {
        /// The target, which passes the content URL policy.
        href: String,
        /// Whether the target is an absolute `http`, `https` or `mailto` one.
        external: bool,
        /// The link text.
        children: Vec<WikiInline>,
    },
    /// An image from a safe source; `title` is absent when the markup gives none.
    Image {
        /// The source, which passes the content URL policy.
        src: String,
        /// The alternative text.
        alt: String,
        /// The title the markup gives.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// A hard line break.
    LineBreak,
}

/// One list item.
/// @contract wiki-page.schema.json#/definitions/WikiListItem
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WikiListItem {
    /// Present on task-list items only: true when the box is ticked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// The item's content.
    pub blocks: Vec<WikiBlock>,
}

/// The alignment of one table column.
/// @contract wiki-page.schema.json#/definitions/WikiTableAlignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiTableAlignment {
    /// No alignment marker.
    None,
    /// Left-aligned.
    Left,
    /// Centred.
    Center,
    /// Right-aligned.
    Right,
}

/// The kind of a callout. The GitHub alerts give note, tip, important, warning and caution; the
/// bracket markers add info and critical.
/// @contract wiki-page.schema.json#/definitions/WikiCalloutKind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiCalloutKind {
    /// Background a reader should know.
    Note,
    /// Optional advice.
    Tip,
    /// Something a reader must not miss.
    Important,
    /// Supplementary information.
    Info,
    /// A risk to avoid.
    Warning,
    /// A consequence to weigh before acting.
    Caution,
    /// A rule whose breach is serious.
    Critical,
}
