//! The nodes the tree builder holds open while it reads a body, and what each becomes when it
//! closes.
//!
//! **Role:** one [`OpenFrame`] per open block or inline container, collecting its children, and
//! [`OpenFrame::close`], which turns a finished frame into the node, or the safe replacement, its
//! parent receives.
//! **Position:** owned by [`super::tree_builder::TreeBuilder`], which pushes a frame on each
//! start event it keeps and closes it on the matching end event.
//! **Signals & state:** each frame owns the children read so far; nothing is shared.
//! **Invariants:** a closed frame never yields an unsafe link or image (an unsafe link yields its
//! children, an unsafe image its alt text) and never an empty paragraph; adjacent text runs of
//! every inline list are merged into one; a quote becomes a callout when it carries a GitHub
//! alert kind or opens with a bracket marker.

use pulldown_cmark::Alignment;

use super::callout_markers::take_bracket_marker;
use super::document_tree::{
    WikiBlock, WikiCalloutKind, WikiInline, WikiListItem, WikiTableAlignment,
};
use super::heading_anchors::HeadingAnchors;
use super::markup_findings::WikiMarkupFinding;
use api_foundation::text::content_url_policy::is_external_link;

/// The style of an emphasis-like inline container.
#[derive(Debug, Clone, Copy)]
pub(super) enum InlineStyle {
    Strong,
    Emphasis,
    Strikethrough,
}

/// An open node of the tree.
#[derive(Debug)]
pub(super) enum OpenFrame {
    /// The page itself, the bottom of the stack.
    Document { blocks: Vec<WikiBlock> },
    /// A paragraph; `implicit` when the builder opened it for inline content read directly in a
    /// block container (a tight list item).
    Paragraph {
        implicit: bool,
        inlines: Vec<WikiInline>,
    },
    /// A heading of `level` 1 to 6.
    Heading { level: u8, inlines: Vec<WikiInline> },
    /// A block quote; `alert` is the GitHub alert kind pulldown-cmark recognised, if any.
    Quote {
        alert: Option<WikiCalloutKind>,
        blocks: Vec<WikiBlock>,
    },
    /// A list; `start` is the first number of a numbered list.
    List {
        start: Option<u64>,
        items: Vec<WikiListItem>,
    },
    /// A list item.
    Item {
        checked: Option<bool>,
        blocks: Vec<WikiBlock>,
    },
    /// A code block.
    Code {
        language: Option<String>,
        text: String,
    },
    /// A raw HTML block starting on `line`, kept as literal text.
    HtmlBlock { line: usize, html: String },
    /// A table.
    Table {
        alignments: Vec<WikiTableAlignment>,
        header: Vec<Vec<WikiInline>>,
        rows: Vec<Vec<Vec<WikiInline>>>,
    },
    /// The header row (`is_header`) or one body row of a table.
    TableRow {
        is_header: bool,
        cells: Vec<Vec<WikiInline>>,
    },
    /// One table cell.
    TableCell { inlines: Vec<WikiInline> },
    /// Strong emphasis, emphasis or strikethrough.
    Styled {
        style: InlineStyle,
        children: Vec<WikiInline>,
    },
    /// A link; `safe` records the URL policy's verdict on `href`.
    Link {
        href: String,
        safe: bool,
        children: Vec<WikiInline>,
    },
    /// An image; its children are its alt text.
    Image {
        src: String,
        title: Option<String>,
        safe: bool,
        alt: Vec<WikiInline>,
    },
}

/// What a closed frame hands to its parent.
#[derive(Debug)]
pub(super) enum ClosedFrame {
    Block(WikiBlock),
    Inlines(Vec<WikiInline>),
    Item(WikiListItem),
    Cell(Vec<WikiInline>),
    Row {
        is_header: bool,
        cells: Vec<Vec<WikiInline>>,
    },
    Nothing,
}

impl OpenFrame {
    /// Whether inline content goes straight into this frame.
    pub(super) fn holds_inlines(&self) -> bool {
        matches!(
            self,
            Self::Paragraph { .. }
                | Self::Heading { .. }
                | Self::TableCell { .. }
                | Self::Styled { .. }
                | Self::Link { .. }
                | Self::Image { .. }
        )
    }

    /// Whether blocks go straight into this frame.
    pub(super) fn holds_blocks(&self) -> bool {
        matches!(
            self,
            Self::Document { .. } | Self::Quote { .. } | Self::Item { .. }
        )
    }

    /// Whether this is a paragraph the builder opened itself.
    pub(super) fn is_implicit_paragraph(&self) -> bool {
        matches!(self, Self::Paragraph { implicit: true, .. })
    }

    /// Appends `inline` when this frame holds inlines; any other frame drops it.
    pub(super) fn push_inline(&mut self, inline: WikiInline) {
        match self {
            Self::Paragraph { inlines, .. }
            | Self::Heading { inlines, .. }
            | Self::TableCell { inlines } => inlines.push(inline),
            Self::Styled { children, .. } | Self::Link { children, .. } => children.push(inline),
            Self::Image { alt, .. } => alt.push(inline),
            _ => {}
        }
    }

    /// Appends `block` when this frame holds blocks; any other frame drops it.
    pub(super) fn push_block(&mut self, block: WikiBlock) {
        if let Self::Document { blocks } | Self::Quote { blocks, .. } | Self::Item { blocks, .. } =
            self
        {
            blocks.push(block);
        }
    }

    /// The node this frame becomes, recording in `findings` what the conversion refuses and
    /// claiming heading anchors from `anchors`.
    pub(super) fn close(
        self,
        anchors: &mut HeadingAnchors,
        findings: &mut Vec<WikiMarkupFinding>,
    ) -> ClosedFrame {
        match self {
            Self::Document { .. } => ClosedFrame::Nothing,
            Self::Paragraph { inlines, .. } => {
                let inlines = merge_text_runs(inlines);
                if inlines.is_empty() {
                    ClosedFrame::Nothing
                } else {
                    ClosedFrame::Block(WikiBlock::Paragraph { inlines })
                }
            }
            Self::Heading { level, inlines } => {
                let inlines = merge_text_runs(inlines);
                let anchor = anchors.claim(&plain_text(&inlines));
                ClosedFrame::Block(WikiBlock::Heading {
                    level,
                    anchor,
                    inlines,
                })
            }
            Self::Quote { alert, mut blocks } => {
                let kind = alert.or_else(|| take_bracket_marker(&mut blocks));
                ClosedFrame::Block(match kind {
                    Some(kind) => WikiBlock::Callout { kind, blocks },
                    None => WikiBlock::Quote { blocks },
                })
            }
            Self::List { start, items } => ClosedFrame::Block(WikiBlock::List {
                ordered: start.is_some(),
                start,
                items,
            }),
            Self::Item { checked, blocks } => ClosedFrame::Item(WikiListItem { checked, blocks }),
            Self::Code { language, text } => ClosedFrame::Block(WikiBlock::Code { language, text }),
            Self::HtmlBlock { line, html } => {
                findings.push(WikiMarkupFinding::raw_html(line, &html));
                let text = html.trim_end_matches(['\r', '\n']).to_string();
                ClosedFrame::Block(WikiBlock::Paragraph {
                    inlines: vec![WikiInline::Text { text }],
                })
            }
            Self::Table {
                alignments,
                header,
                rows,
            } => ClosedFrame::Block(WikiBlock::Table {
                alignments,
                header,
                rows,
            }),
            Self::TableRow { is_header, cells } => ClosedFrame::Row { is_header, cells },
            Self::TableCell { inlines } => ClosedFrame::Cell(merge_text_runs(inlines)),
            Self::Styled { style, children } => {
                let children = merge_text_runs(children);
                ClosedFrame::Inlines(vec![match style {
                    InlineStyle::Strong => WikiInline::Strong { children },
                    InlineStyle::Emphasis => WikiInline::Emphasis { children },
                    InlineStyle::Strikethrough => WikiInline::Strikethrough { children },
                }])
            }
            Self::Link {
                href,
                safe,
                children,
            } => {
                let children = merge_text_runs(children);
                if safe {
                    let external = is_external_link(&href);
                    ClosedFrame::Inlines(vec![WikiInline::Link {
                        href,
                        external,
                        children,
                    }])
                } else {
                    ClosedFrame::Inlines(children)
                }
            }
            Self::Image {
                src,
                title,
                safe,
                alt,
            } => {
                let alt = plain_text(&alt);
                if safe {
                    ClosedFrame::Inlines(vec![WikiInline::Image { src, alt, title }])
                } else if alt.is_empty() {
                    ClosedFrame::Nothing
                } else {
                    ClosedFrame::Inlines(vec![WikiInline::Text { text: alt }])
                }
            }
        }
    }
}

/// The column alignment of the wiki tree for a pulldown-cmark table alignment.
pub(super) fn table_alignment(alignment: Alignment) -> WikiTableAlignment {
    match alignment {
        Alignment::None => WikiTableAlignment::None,
        Alignment::Left => WikiTableAlignment::Left,
        Alignment::Center => WikiTableAlignment::Center,
        Alignment::Right => WikiTableAlignment::Right,
    }
}

/// `inlines` with every run of adjacent text nodes joined into one and empty text dropped.
pub(super) fn merge_text_runs(inlines: Vec<WikiInline>) -> Vec<WikiInline> {
    let mut merged: Vec<WikiInline> = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match (merged.last_mut(), inline) {
            (_, WikiInline::Text { text }) if text.is_empty() => {}
            (Some(WikiInline::Text { text: previous }), WikiInline::Text { text }) => {
                previous.push_str(&text);
            }
            (_, inline) => merged.push(inline),
        }
    }
    merged
}

/// The text a reader sees in `inlines`, markup removed: a line break reads as a space.
pub(super) fn plain_text(inlines: &[WikiInline]) -> String {
    let mut text = String::new();
    append_plain_text(inlines, &mut text);
    text
}

/// Appends the plain text of `inlines` to `text`; the depth of the recursion is bounded by the
/// builder's nesting limit.
fn append_plain_text(inlines: &[WikiInline], text: &mut String) {
    for inline in inlines {
        match inline {
            WikiInline::Text { text: run } | WikiInline::Code { text: run } => text.push_str(run),
            WikiInline::Strong { children }
            | WikiInline::Emphasis { children }
            | WikiInline::Strikethrough { children }
            | WikiInline::Link { children, .. } => append_plain_text(children, text),
            WikiInline::Image { alt, .. } => text.push_str(alt),
            WikiInline::LineBreak => text.push(' '),
        }
    }
}
