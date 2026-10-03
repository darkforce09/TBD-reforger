//! The wiki markup service: a page's markdown read into the typed tree the doctrine wiki renders,
//! and the constructs a save refuses.
//!
//! **Role:** [`read_markup`] parses one `body_md` with pulldown-cmark (tables, task lists,
//! strikethrough, GitHub blockquote alerts and the bracket callout markers) into
//! [`WikiBlock`]s that are safe to render as they stand, and lists every
//! [`WikiMarkupFinding`] a save refuses.
//! **Position:** called by the wiki handlers in
//! [`crate::handlers::wiki_knowledgebase`]: the reads serialize
//! [`MarkupReading::blocks`] into the article and the revision, and the save answers
//! `422 wiki_markup_refused` when [`MarkupReading::findings`] is not empty. The URL rules come
//! from [`api_foundation::text::content_url_policy`].
//! **Signals & state:** none; each call builds and drops its own tree builder.
//! **Invariants:** the blocks never carry a link or image the content URL policy refuses, raw
//! HTML or a container nested deeper than [`MAX_NESTING_DEPTH`]; each such construct in the
//! source yields exactly the safe form described on [`read_markup`] and a finding; the same body
//! always reads to the same blocks and findings.

mod callout_markers;
mod document_tree;
mod heading_anchors;
mod markup_findings;
mod open_frames;
mod tree_builder;

pub use document_tree::{WikiBlock, WikiCalloutKind, WikiInline, WikiListItem, WikiTableAlignment};
pub use heading_anchors::slugify;
pub use markup_findings::{WikiMarkupFinding, WikiMarkupFindingCode};

/// The deepest a list, quote, callout, strong, emphasis, strikethrough or link may sit inside
/// others of these; a deeper one is flattened into its parent.
pub const MAX_NESTING_DEPTH: usize = 16;

/// One markdown body, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkupReading {
    /// The page's blocks, safe to render.
    pub blocks: Vec<WikiBlock>,
    /// Every construct a save refuses, in line order; empty when the body may be saved.
    pub findings: Vec<WikiMarkupFinding>,
}

/// Reads `markdown` into its safe blocks and its refusal findings.
///
/// The blocks replace each refused construct with its safe form: a link whose target the
/// content URL policy refuses becomes its children, an image whose source it refuses becomes
/// its alt text, raw HTML becomes literal text, and a container deeper than
/// [`MAX_NESTING_DEPTH`] is replaced by its children.
pub fn read_markup(markdown: &str) -> MarkupReading {
    let (blocks, findings) = tree_builder::TreeBuilder::build(markdown);
    MarkupReading { blocks, findings }
}

#[cfg(test)]
#[path = "tests/wiki_markup.rs"]
mod tests;
