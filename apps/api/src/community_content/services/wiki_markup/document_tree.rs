//! The typed tree of a wiki page: the blocks and inlines the markup service builds from markdown.
//!
//! **Role:** the one shape of a parsed wiki page, serialized as the `blocks` of an article or a
//! revision.
//! **Position:** built by [`super::read_markup`]; embedded in
//! [`crate::community_content::models::wiki::WikiArticle`] and
//! [`crate::community_content::models::wiki::WikiRevision`], which the doctrine wiki page of the
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
        level: u8,
        anchor: String,
        inlines: Vec<WikiInline>,
    },
    /// A paragraph.
    Paragraph { inlines: Vec<WikiInline> },
    /// A bulleted list (`ordered` false, no `start`) or a numbered one, which carries the
    /// number of its first item as `start`.
    List {
        ordered: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<u64>,
        items: Vec<WikiListItem>,
    },
    /// A table: one alignment per column, the header cells and the body rows of cells; a cell
    /// is a list of inlines.
    Table {
        alignments: Vec<WikiTableAlignment>,
        header: Vec<Vec<WikiInline>>,
        rows: Vec<Vec<Vec<WikiInline>>>,
    },
    /// A callout box of one [`WikiCalloutKind`].
    Callout {
        kind: WikiCalloutKind,
        blocks: Vec<WikiBlock>,
    },
    /// A block quote that carries no callout marker.
    Quote { blocks: Vec<WikiBlock> },
    /// A code block; `language` is the fence's trimmed info string, absent when it has none.
    Code {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
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
    Text { text: String },
    /// Strong emphasis.
    Strong { children: Vec<WikiInline> },
    /// Emphasis.
    Emphasis { children: Vec<WikiInline> },
    /// Struck-through text.
    Strikethrough { children: Vec<WikiInline> },
    /// Inline code.
    Code { text: String },
    /// A link to a safe target; `external` is true for an absolute `http`, `https` or `mailto`
    /// target.
    Link {
        href: String,
        external: bool,
        children: Vec<WikiInline>,
    },
    /// An image from a safe source; `title` is absent when the markup gives none.
    Image {
        src: String,
        alt: String,
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
    pub blocks: Vec<WikiBlock>,
}

/// The alignment of one table column.
/// @contract wiki-page.schema.json#/definitions/WikiTableAlignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiTableAlignment {
    None,
    Left,
    Center,
    Right,
}

/// The kind of a callout. The GitHub alerts give note, tip, important, warning and caution; the
/// bracket markers add info and critical.
/// @contract wiki-page.schema.json#/definitions/WikiCalloutKind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiCalloutKind {
    Note,
    Tip,
    Important,
    Info,
    Warning,
    Caution,
    Critical,
}
