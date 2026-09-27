//! The tree builder: pulldown-cmark's event stream in, the safe typed tree and the refusal
//! findings out.
//!
//! **Role:** reads one markdown body with the wiki's parser options and builds its
//! [`WikiBlock`] list, replacing every construct a save would refuse with its safe form and
//! recording a [`WikiMarkupFinding`] for it.
//! **Position:** driven by [`super::read_markup`]; the frames it holds open are
//! [`super::open_frames::OpenFrame`]s.
//! **Signals & state:** one builder per body: the stack of open frames, one closing rule per
//! start event, the current container depth, the heading anchors given out, the findings, and
//! whether a task-list box may still tick the item just opened.
//! **Invariants:** the builder never recurses per event, so no body can exhaust the stack while
//! it is read; a container (list, quote, callout, strong, emphasis, strikethrough, link) that
//! would sit deeper than [`super::MAX_NESTING_DEPTH`] is flattened into its parent, and the
//! first container of each such run is recorded as `nesting_too_deep`; raw HTML is kept as
//! literal text and recorded; a link or image whose URL the content URL policy refuses is
//! recorded where it starts; findings are ordered by line.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, CowStr, Event, LinkType, Options, Parser, Tag, TagEnd};

use super::MAX_NESTING_DEPTH;
use super::callout_markers::alert_callout_kind;
use super::document_tree::{WikiBlock, WikiInline};
use super::heading_anchors::HeadingAnchors;
use super::markup_findings::{SourceLines, WikiMarkupFinding};
use super::open_frames::{ClosedFrame, InlineStyle, OpenFrame, table_alignment};
use crate::core::text::content_url_policy::{is_safe_image_url, is_safe_link_url};

/// The markdown extensions the wiki reads: tables, task lists, strikethrough and GitHub
/// blockquote alerts. Footnotes, heading attributes, math, metadata blocks and smart
/// punctuation stay off.
pub fn wiki_parser_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_GFM
}

/// What the end event matching one start event does.
#[derive(Debug, Clone, Copy)]
struct Closing {
    /// The start event pushed a frame, which the end event closes; otherwise the start event was
    /// flattened or ignored and its children went to the enclosing frame.
    closes_frame: bool,
    /// The start event opened a container and raised the depth.
    is_container: bool,
}

/// The builder state for one body.
pub struct TreeBuilder {
    lines: SourceLines,
    frames: Vec<OpenFrame>,
    closings: Vec<Closing>,
    depth: usize,
    anchors: HeadingAnchors,
    findings: Vec<WikiMarkupFinding>,
    /// Whether the last list item start opened an item frame that no other content has
    /// reached yet, so a task-list box read now belongs to it.
    awaiting_task_marker: bool,
}

impl TreeBuilder {
    /// The blocks and the findings of `source`.
    pub fn build(source: &str) -> (Vec<WikiBlock>, Vec<WikiMarkupFinding>) {
        let mut builder = Self {
            lines: SourceLines::new(source),
            frames: vec![OpenFrame::Document { blocks: Vec::new() }],
            closings: Vec::new(),
            depth: 0,
            anchors: HeadingAnchors::default(),
            findings: Vec::new(),
            awaiting_task_marker: false,
        };
        for (event, range) in Parser::new_ext(source, wiki_parser_options()).into_offset_iter() {
            builder.read(event, range);
        }
        builder.finish()
    }

    /// Applies one event.
    fn read(&mut self, event: Event<'_>, range: Range<usize>) {
        let keeps_task_marker_slot = matches!(
            event,
            Event::Start(Tag::Item | Tag::Paragraph) | Event::TaskListMarker(_)
        );
        if !keeps_task_marker_slot {
            self.awaiting_task_marker = false;
        }
        match event {
            Event::Start(tag) => self.start(tag, range.start),
            Event::End(tag_end) => self.end(tag_end),
            Event::Text(text) => self.text(text),
            Event::Code(text) => self.push_inline(WikiInline::Code {
                text: text.into_string(),
            }),
            Event::Html(html) => self.html(html, range.start),
            Event::InlineHtml(html) => self.inline_html(html, range.start),
            Event::InlineMath(text) | Event::DisplayMath(text) => self.text(text),
            Event::FootnoteReference(label) => self.push_inline(WikiInline::Text {
                text: format!("[^{label}]"),
            }),
            Event::SoftBreak => self.push_inline(WikiInline::Text {
                text: "\n".to_string(),
            }),
            Event::HardBreak => self.push_inline(WikiInline::LineBreak),
            Event::Rule => {
                self.close_implicit_paragraph();
                self.push_block(WikiBlock::Rule);
            }
            Event::TaskListMarker(checked) => self.task_list_marker(checked),
        }
    }

    /// Opens the frame of one start event, or records why it is not opened.
    fn start(&mut self, tag: Tag<'_>, offset: usize) {
        match tag {
            Tag::Paragraph => self.open_block(OpenFrame::Paragraph {
                implicit: false,
                inlines: Vec::new(),
            }),
            Tag::Heading { level, .. } => self.open_block(OpenFrame::Heading {
                level: level as u8,
                inlines: Vec::new(),
            }),
            Tag::BlockQuote(kind) => {
                self.close_implicit_paragraph();
                self.open_container(
                    offset,
                    OpenFrame::Quote {
                        alert: kind.map(alert_callout_kind),
                        blocks: Vec::new(),
                    },
                );
            }
            Tag::CodeBlock(kind) => self.open_block(OpenFrame::Code {
                language: match kind {
                    CodeBlockKind::Fenced(info) => Some(info.trim().to_string()),
                    CodeBlockKind::Indented => None,
                }
                .filter(|language| !language.is_empty()),
                text: String::new(),
            }),
            Tag::HtmlBlock => self.open_block(OpenFrame::HtmlBlock {
                line: self.lines.line_of(offset),
                html: String::new(),
            }),
            Tag::List(start) => {
                self.close_implicit_paragraph();
                self.open_container(
                    offset,
                    OpenFrame::List {
                        start,
                        items: Vec::new(),
                    },
                );
            }
            Tag::Item => {
                self.close_implicit_paragraph();
                let in_list = matches!(self.frames.last(), Some(OpenFrame::List { .. }));
                if in_list {
                    self.open(OpenFrame::Item {
                        checked: None,
                        blocks: Vec::new(),
                    });
                } else {
                    self.skip(false);
                }
                self.awaiting_task_marker = in_list;
            }
            Tag::Table(alignments) => self.open_block(OpenFrame::Table {
                alignments: alignments.into_iter().map(table_alignment).collect(),
                header: Vec::new(),
                rows: Vec::new(),
            }),
            Tag::TableHead | Tag::TableRow => self.open(OpenFrame::TableRow {
                is_header: matches!(tag, Tag::TableHead),
                cells: Vec::new(),
            }),
            Tag::TableCell => self.open(OpenFrame::TableCell {
                inlines: Vec::new(),
            }),
            Tag::Emphasis => self.open_styled(offset, InlineStyle::Emphasis),
            Tag::Strong => self.open_styled(offset, InlineStyle::Strong),
            Tag::Strikethrough => self.open_styled(offset, InlineStyle::Strikethrough),
            Tag::Link {
                link_type,
                dest_url,
                ..
            } => self.open_link(offset, link_type, dest_url),
            Tag::Image {
                dest_url, title, ..
            } => self.open_image(offset, dest_url, title),
            Tag::Superscript
            | Tag::Subscript
            | Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::MetadataBlock(_) => self.skip(false),
        }
    }

    /// Closes what the start event matching `tag_end` opened.
    fn end(&mut self, tag_end: TagEnd) {
        let ends_an_inline = matches!(
            tag_end,
            TagEnd::Emphasis
                | TagEnd::Strong
                | TagEnd::Strikethrough
                | TagEnd::Link
                | TagEnd::Image
                | TagEnd::Superscript
                | TagEnd::Subscript
        );
        if !ends_an_inline {
            self.close_implicit_paragraph();
        }
        let Some(closing) = self.closings.pop() else {
            return;
        };
        if closing.is_container {
            self.depth -= 1;
        }
        if closing.closes_frame {
            self.close_top_frame();
        }
    }

    /// Text: the content of an open code or HTML block, else an inline text run.
    fn text(&mut self, text: CowStr<'_>) {
        match self.frames.last_mut() {
            Some(OpenFrame::Code { text: code, .. }) => code.push_str(&text),
            Some(OpenFrame::HtmlBlock { html, .. }) => html.push_str(&text),
            _ => self.push_inline(WikiInline::Text {
                text: text.into_string(),
            }),
        }
    }

    /// One line of an HTML block; outside one, the same as inline HTML.
    fn html(&mut self, html: CowStr<'_>, offset: usize) {
        match self.frames.last_mut() {
            Some(OpenFrame::HtmlBlock { html: block, .. }) => block.push_str(&html),
            _ => self.inline_html(html, offset),
        }
    }

    /// Inline HTML: kept as literal text and recorded.
    fn inline_html(&mut self, html: CowStr<'_>, offset: usize) {
        self.findings.push(WikiMarkupFinding::raw_html(
            self.lines.line_of(offset),
            &html,
        ));
        self.push_inline(WikiInline::Text {
            text: html.into_string(),
        });
    }

    /// A task-list box: ticks the item whose start it follows, or reads as literal text where
    /// that item was flattened away.
    fn task_list_marker(&mut self, checked: bool) {
        if std::mem::take(&mut self.awaiting_task_marker) {
            let item_slot = self
                .frames
                .iter_mut()
                .rev()
                .take(2)
                .find_map(|frame| match frame {
                    OpenFrame::Item { checked: slot, .. } => Some(slot),
                    _ => None,
                });
            if let Some(slot) = item_slot {
                *slot = Some(checked);
                return;
            }
        }
        let text = if checked { "[x] " } else { "[ ] " };
        self.push_inline(WikiInline::Text {
            text: text.to_string(),
        });
    }

    /// Opens a block frame after closing any implicit paragraph.
    fn open_block(&mut self, frame: OpenFrame) {
        self.close_implicit_paragraph();
        self.open(frame);
    }

    /// Opens an emphasis-like container.
    fn open_styled(&mut self, offset: usize, style: InlineStyle) {
        self.ensure_inline_host();
        self.open_container(
            offset,
            OpenFrame::Styled {
                style,
                children: Vec::new(),
            },
        );
    }

    /// Opens a link container, recording its target when the policy refuses it. An email
    /// autolink's target is the bare address, so it gains the `mailto:` scheme first.
    fn open_link(&mut self, offset: usize, link_type: LinkType, destination: CowStr<'_>) {
        self.ensure_inline_host();
        let href = match link_type {
            LinkType::Email => format!("mailto:{destination}"),
            _ => destination.into_string(),
        };
        let safe = is_safe_link_url(&href);
        if !safe {
            self.findings.push(WikiMarkupFinding::unsafe_link(
                self.lines.line_of(offset),
                &href,
            ));
        }
        self.open_container(
            offset,
            OpenFrame::Link {
                href,
                safe,
                children: Vec::new(),
            },
        );
    }

    /// Opens an image, recording its source when the policy refuses it.
    fn open_image(&mut self, offset: usize, source: CowStr<'_>, title: CowStr<'_>) {
        self.ensure_inline_host();
        let src = source.into_string();
        let safe = is_safe_image_url(&src);
        if !safe {
            self.findings.push(WikiMarkupFinding::unsafe_image(
                self.lines.line_of(offset),
                &src,
            ));
        }
        self.open(OpenFrame::Image {
            src,
            title: Some(title.into_string()).filter(|title| !title.is_empty()),
            safe,
            alt: Vec::new(),
        });
    }

    /// Opens `frame` as a container one level deeper, or flattens it when that level is past
    /// the limit, recording the first flattened level of the run.
    fn open_container(&mut self, offset: usize, frame: OpenFrame) {
        self.depth += 1;
        if self.depth <= MAX_NESTING_DEPTH {
            self.frames.push(frame);
            self.closings.push(Closing {
                closes_frame: true,
                is_container: true,
            });
            return;
        }
        if self.depth == MAX_NESTING_DEPTH + 1 {
            self.findings.push(WikiMarkupFinding::nesting_too_deep(
                self.lines.line_of(offset),
            ));
        }
        self.skip(true);
    }

    /// Pushes a frame that the matching end event closes.
    fn open(&mut self, frame: OpenFrame) {
        self.frames.push(frame);
        self.closings.push(Closing {
            closes_frame: true,
            is_container: false,
        });
    }

    /// Opens nothing for a start event: its children go to the enclosing frame.
    fn skip(&mut self, is_container: bool) {
        self.closings.push(Closing {
            closes_frame: false,
            is_container,
        });
    }

    /// Makes the top frame one that holds inlines, opening an implicit paragraph in a block
    /// container.
    fn ensure_inline_host(&mut self) {
        if let Some(top) = self.frames.last()
            && !top.holds_inlines()
            && top.holds_blocks()
        {
            self.frames.push(OpenFrame::Paragraph {
                implicit: true,
                inlines: Vec::new(),
            });
        }
    }

    /// Closes the top frame when it is an implicit paragraph.
    fn close_implicit_paragraph(&mut self) {
        if self
            .frames
            .last()
            .is_some_and(OpenFrame::is_implicit_paragraph)
        {
            self.close_top_frame();
        }
    }

    /// Adds an inline to the top frame.
    fn push_inline(&mut self, inline: WikiInline) {
        self.ensure_inline_host();
        if let Some(top) = self.frames.last_mut() {
            top.push_inline(inline);
        }
    }

    /// Adds a block to the nearest frame that holds blocks.
    fn push_block(&mut self, block: WikiBlock) {
        if let Some(host) = self
            .frames
            .iter_mut()
            .rev()
            .find(|frame| frame.holds_blocks())
        {
            host.push_block(block);
        }
    }

    /// Pops the top frame and hands what it becomes to the frame below.
    fn close_top_frame(&mut self) {
        if self.frames.len() <= 1 {
            return;
        }
        let Some(frame) = self.frames.pop() else {
            return;
        };
        match frame.close(&mut self.anchors, &mut self.findings) {
            ClosedFrame::Block(block) => self.push_block(block),
            ClosedFrame::Inlines(inlines) => {
                for inline in inlines {
                    self.push_inline(inline);
                }
            }
            ClosedFrame::Item(item) => match self.frames.last_mut() {
                Some(OpenFrame::List { items, .. }) => items.push(item),
                _ => item
                    .blocks
                    .into_iter()
                    .for_each(|block| self.push_block(block)),
            },
            ClosedFrame::Cell(cell) => {
                if let Some(OpenFrame::TableRow { cells, .. }) = self.frames.last_mut() {
                    cells.push(cell);
                }
            }
            ClosedFrame::Row { is_header, cells } => {
                if let Some(OpenFrame::Table { header, rows, .. }) = self.frames.last_mut() {
                    if is_header {
                        *header = cells;
                    } else {
                        rows.push(cells);
                    }
                }
            }
            ClosedFrame::Nothing => {}
        }
    }

    /// Closes every frame still open and returns the page's blocks and its findings in line
    /// order.
    fn finish(mut self) -> (Vec<WikiBlock>, Vec<WikiMarkupFinding>) {
        while self.frames.len() > 1 {
            self.close_top_frame();
        }
        let blocks = match self.frames.pop() {
            Some(OpenFrame::Document { blocks }) => blocks,
            _ => Vec::new(),
        };
        self.findings.sort_by_key(|finding| finding.line);
        (blocks, self.findings)
    }
}

#[cfg(test)]
#[path = "tests/tree_builder.rs"]
mod tests;
