//! Doctrine wiki payloads: the navigation list, the article with its typed blocks, the
//! administrator's save and its refusal, and the revision history.
//!
//! **Role:** the page summary `GET /wiki` lists, the article `GET /wiki/:slug` and an accepted save
//! answer, the tree of blocks and inlines the backend parses from a page's markdown, the save body
//! and the `details` of a refused save, and the revision list and single revision.
//! **Position:** deserialised straight from the backend's JSON and handed to the doctrine wiki
//! page; the save body is serialised for `PUT /wiki/:slug`; re-serialised unchanged by the
//! round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** every block and inline is an object tagged by a snake_case `type`, and its
//! optional fields (`start`, `checked`, `language`, `title`) are absent, never `null`. A link
//! `href` and an image `src` already passed the backend's content URL policy, which
//! `frontend_ui::safe_url` repeats in the app. `icon` is an empty string for a
//! page without one and absent on the wire; an unknown editor or author is absent. A save names
//! every field, `base_revision` included, which is `null` exactly when the save creates the page.
//! @contract wiki-page.schema.json#/definitions/WikiArticle
//! @contract wiki-page.schema.json#/definitions/WikiBlock
//! @contract wiki-page.schema.json#/definitions/WikiInline

use super::identifiers::{DiscordUserId, WikiPageId};
use serde::{Deserialize, Serialize};

/// One page in the doctrine navigation, ordered by `nav_order`, then `title`, then `slug`.
/// @contract wiki-page.schema.json#/definitions/WikiPageSummary
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WikiPageSummary {
    /// The page's URL key.
    pub slug: String,
    /// The navigation section the page sits in.
    pub category: String,
    /// The page title.
    pub title: String,
    /// The icon name; empty when the page has none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// The position within the category, ascending.
    pub nav_order: i64,
    /// The page's current revision.
    pub revision: i64,
    /// When the page was last saved.
    pub updated_at: String,
}

/// One page: its summary fields, its markdown source and the blocks parsed from it.
/// @contract wiki-page.schema.json#/definitions/WikiArticle
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WikiArticle {
    /// The page's key.
    pub id: WikiPageId,
    /// The page's URL key.
    pub slug: String,
    /// The navigation section the page sits in.
    pub category: String,
    /// The page title.
    pub title: String,
    /// The icon name; empty when the page has none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// The position within the category, ascending.
    pub nav_order: i64,
    /// The page's current revision, the `base_revision` of the next save.
    pub revision: i64,
    /// When the page was last saved.
    pub updated_at: String,
    /// The Discord id of the last editor; absent when unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    /// The markdown source.
    pub body_md: String,
    /// The block tree read from `body_md`.
    pub blocks: Vec<WikiBlock>,
}

/// One block of a page.
/// @contract wiki-page.schema.json#/definitions/WikiBlock
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WikiBlock {
    /// A heading of level 1 to 6; `anchor` is the slugified heading text, unique within the page.
    Heading {
        /// The heading level, 1 to 6.
        level: u8,
        /// The slugified heading text, unique within the page.
        anchor: String,
        /// The heading's text runs.
        inlines: Vec<WikiInline>,
    },
    /// A paragraph.
    Paragraph {
        /// The paragraph's text runs.
        inlines: Vec<WikiInline>,
    },
    /// A bulleted list, or a numbered one that carries the number of its first item as `start`.
    List {
        /// Whether the list is numbered.
        ordered: bool,
        /// The number of a numbered list's first item; absent for a bulleted list.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<u64>,
        /// The list's items.
        items: Vec<WikiListItem>,
    },
    /// A table: one alignment per column, the header cells and the body rows of cells; a cell is
    /// a list of inlines.
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
        /// The quote's content.
        blocks: Vec<WikiBlock>,
    },
    /// A code block; `language` is the fence's info string, absent when it has none.
    Code {
        /// The fence's info string; absent when it has none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        /// The code itself.
        text: String,
    },
    /// A thematic break.
    Rule,
}

/// One inline run.
/// @contract wiki-page.schema.json#/definitions/WikiInline
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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
        /// The code itself.
        text: String,
    },
    /// A link; `external` is true for an absolute `http`, `https` or `mailto` target.
    Link {
        /// The link target.
        href: String,
        /// Whether the target is an absolute `http`, `https` or `mailto` link.
        external: bool,
        /// The link's text runs.
        children: Vec<WikiInline>,
    },
    /// An image; `title` is absent when the markup gives none.
    Image {
        /// The image source.
        src: String,
        /// The alternative text.
        alt: String,
        /// The image title; absent when the markup gives none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// A hard line break.
    LineBreak,
}

/// One list item.
/// @contract wiki-page.schema.json#/definitions/WikiListItem
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiListItem {
    /// Present on task-list items only: true when the box is ticked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// The item's content.
    pub blocks: Vec<WikiBlock>,
}

/// The alignment of one table column.
/// @contract wiki-page.schema.json#/definitions/WikiTableAlignment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiTableAlignment {
    /// No alignment given.
    None,
    /// Left-aligned.
    Left,
    /// Centred.
    Center,
    /// Right-aligned.
    Right,
}

/// The kind of a callout: the GitHub alerts give note, tip, important, warning and caution, and
/// the bracket markers add info and critical.
/// @contract wiki-page.schema.json#/definitions/WikiCalloutKind
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiCalloutKind {
    /// A note.
    Note,
    /// A tip.
    Tip,
    /// Something important.
    Important,
    /// Background information.
    Info,
    /// A warning.
    Warning,
    /// A caution.
    Caution,
    /// Something critical.
    Critical,
}

/// The `PUT /wiki/:slug` body. Every field is sent; the backend refuses any other.
/// @contract wiki-page.schema.json#/definitions/WikiSaveRequest
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WikiSaveRequest {
    /// The navigation section the page goes in.
    pub category: String,
    /// The page title.
    pub title: String,
    /// Empty for a page without an icon.
    pub icon: String,
    /// The navigation position; `0` is a real position, so the field has no default.
    pub nav_order: i64,
    /// The markdown source.
    pub body_md: String,
    /// The revision the edit starts from, or `null` to create the page; always sent.
    pub base_revision: Option<i64>,
}

/// The `details.code` of a refused save; each wire code carries the `wiki_` prefix the variant
/// name leaves out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WikiSaveRefusalCode {
    /// `409 wiki_revision_conflict`: the page's current revision is not the save's base;
    /// `current_revision` names it.
    #[serde(rename = "wiki_revision_conflict")]
    RevisionConflict,
    /// `400 wiki_body_too_large`: `body_md` is over 262 144 bytes.
    #[serde(rename = "wiki_body_too_large")]
    BodyTooLarge,
    /// `422 wiki_markup_refused`: the markup holds refused constructs; `findings` lists every one.
    #[serde(rename = "wiki_markup_refused")]
    MarkupRefused,
}

/// The `details` of a refused save, under the error envelope.
/// @contract wiki-page.schema.json#/definitions/WikiSaveRefusal
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WikiSaveRefusal {
    /// Why the save was refused.
    pub code: WikiSaveRefusalCode,
    /// The page's current revision, answered on a revision conflict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_revision: Option<i64>,
    /// The refused constructs, answered on a markup refusal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<Vec<WikiMarkupFinding>>,
}

/// The rule a refused construct breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WikiMarkupFindingCode {
    /// A link whose target the content URL policy refuses.
    UnsafeLinkUrl,
    /// An image whose source the content URL policy refuses.
    UnsafeImageUrl,
    /// Inline or block HTML.
    RawHtml,
    /// A list, quote, callout, link or emphasis nested deeper than 16.
    NestingTooDeep,
}

/// One construct a save refuses.
/// @contract wiki-page.schema.json#/definitions/WikiMarkupFinding
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiMarkupFinding {
    /// The 1-based line of `body_md` on which the construct starts.
    pub line: u64,
    /// The rule the construct breaks.
    pub code: WikiMarkupFindingCode,
    /// The explanation shown to the author.
    pub detail: String,
}

/// One entry of a page's revision history.
/// @contract wiki-page.schema.json#/definitions/WikiRevisionSummary
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WikiRevisionSummary {
    /// The revision number.
    pub revision: i64,
    /// The page title the revision saved.
    pub title: String,
    /// The Discord id of the editor; absent when unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<DiscordUserId>,
    /// When the revision was saved.
    pub created_at: String,
}

/// `GET /wiki/:slug/revisions`: one page of the history, newest first.
/// @contract wiki-page.schema.json#/definitions/WikiRevisionPage
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WikiRevisionPage {
    /// The revisions on this page of the history, newest first.
    pub items: Vec<WikiRevisionSummary>,
    /// The 1-based number of this history page.
    pub page: i64,
    /// The revisions per history page.
    pub per_page: i64,
    /// The number of revisions the page has.
    pub total: i64,
}

/// `GET /wiki/:slug/revisions/:revision`: the page as one revision saved it, its blocks parsed and
/// made safe as an article's are. Restoring it is a save of its fields with the page's current
/// revision as `base_revision`.
/// @contract wiki-page.schema.json#/definitions/WikiRevision
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WikiRevision {
    /// The page's URL key.
    pub slug: String,
    /// The revision number.
    pub revision: i64,
    /// The navigation section the revision saved.
    pub category: String,
    /// The page title the revision saved.
    pub title: String,
    /// The icon name; empty when the revision has none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// The position within the category, ascending.
    pub nav_order: i64,
    /// The markdown source.
    pub body_md: String,
    /// The Discord id of the editor; absent when unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<DiscordUserId>,
    /// When the revision was saved.
    pub created_at: String,
    /// The block tree read from `body_md`.
    pub blocks: Vec<WikiBlock>,
}

#[cfg(test)]
#[path = "tests/wiki.rs"]
mod tests;
