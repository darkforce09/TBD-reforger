//! A manual's blocks — headings, paragraphs, lists, tables, callouts, quotes, code, rules — as
//! render nodes.
//!
//! **Role:** maps each [`WikiBlock`] to the element it renders as, with the class that styles it,
//! recursing into the blocks a list item, a callout or a quote holds.
//! **Position:** the entry of the block renderer: fed an article's or a revision's blocks, its
//! nodes become views in the element views.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a heading's element matches its level (a level outside 1–6 is clamped into
//! it) and carries its anchor as `id`, so an in-page `#anchor` link scrolls to it. A task-list
//! item leads with a disabled checkbox named by the item's text; every other item has none. A
//! numbered list keeps its `start`. A code block's text is one text node, whitespace kept.

use super::callout_style::callout_style;
use super::inline_mapping::inline_nodes;
use super::render_tree::{ElementTag, RenderElement, RenderNode};
use super::table_mapping::table_node;
use crate::foundation::transport::dto::wiki::{
    WikiBlock, WikiCalloutKind, WikiInline, WikiListItem,
};

/// Heading classes, level 1 first.
const HEADING_CLASSES: [&str; 6] = [
    "mt-8 mb-4 scroll-mt-6 text-2xl font-bold tracking-tight text-white first:mt-0",
    "mt-10 mb-3 scroll-mt-6 border-b border-white/10 pb-2 text-xl font-bold tracking-tight text-white first:mt-0",
    "mt-8 mb-2 scroll-mt-6 text-lg font-bold tracking-tight text-white first:mt-0",
    "mt-6 mb-2 scroll-mt-6 text-base font-semibold text-on-surface first:mt-0",
    "mt-5 mb-1 scroll-mt-6 text-sm font-semibold tracking-wide text-on-surface uppercase first:mt-0",
    "mt-5 mb-1 scroll-mt-6 font-mono text-xs font-semibold tracking-widest text-on-surface-variant uppercase first:mt-0",
];
/// A paragraph; the first block of a list item, a callout or a quote sits flush.
const PARAGRAPH_CLASS: &str =
    "mt-3 text-body-md leading-relaxed text-on-surface-variant first:mt-0";
/// A bulleted list.
const BULLET_LIST_CLASS: &str =
    "mt-3 ml-6 list-disc space-y-2 text-body-md text-on-surface-variant marker:text-outline first:mt-0";
/// A numbered list.
const NUMBERED_LIST_CLASS: &str =
    "mt-3 ml-6 list-decimal space-y-2 text-body-md text-on-surface-variant marker:text-outline first:mt-0";
/// An ordinary list item.
const LIST_ITEM_CLASS: &str = "pl-1";
/// A task-list item: no bullet, the checkbox beside the text.
const TASK_ITEM_CLASS: &str = "-ml-6 flex list-none items-start gap-3";
/// A task-list item's checkbox.
const CHECKBOX_CLASS: &str = "mt-1 h-4 w-4 shrink-0 accent-primary";
/// The text beside a task-list item's checkbox.
const TASK_TEXT_CLASS: &str = "min-w-0 flex-1";
/// The text inside a callout box.
const CALLOUT_TEXT_CLASS: &str = "text-body-md leading-relaxed text-on-surface-variant";
/// A block quote.
const QUOTE_CLASS: &str = "my-4 border-l-2 border-outline-variant pl-4 italic";
/// The frame of a code block.
const CODE_BLOCK_CLASS: &str =
    "my-4 overflow-x-auto rounded-xl border border-white/10 bg-black/40 p-4 first:mt-0";
/// The text of a code block.
const CODE_BLOCK_TEXT_CLASS: &str = "font-mono text-sm leading-relaxed text-on-surface";
/// A thematic break.
const RULE_CLASS: &str = "my-8 border-white/10";

/// The nodes of a run of blocks, in order.
pub(super) fn block_nodes(blocks: &[WikiBlock]) -> Vec<RenderNode> {
    blocks.iter().map(block_node).collect()
}

/// The node one block renders as.
pub(super) fn block_node(block: &WikiBlock) -> RenderNode {
    match block {
        WikiBlock::Heading {
            level,
            anchor,
            inlines,
        } => heading_node(*level, anchor, inlines),
        WikiBlock::Paragraph { inlines } => paragraph_node(inlines),
        WikiBlock::List {
            ordered,
            start,
            items,
        } => list_node(*ordered, *start, items),
        WikiBlock::Table {
            alignments,
            header,
            rows,
        } => table_node(alignments, header, rows),
        WikiBlock::Callout { kind, blocks } => callout_node(*kind, blocks),
        WikiBlock::Quote { blocks } => RenderElement::new(ElementTag::Blockquote, QUOTE_CLASS)
            .with_children(block_nodes(blocks))
            .into_node(),
        WikiBlock::Code { language, text } => code_node(language.as_deref(), text),
        WikiBlock::Rule => RenderElement::new(ElementTag::HorizontalRule, RULE_CLASS).into_node(),
    }
}

/// The element of a heading at `level`, clamped into 1–6.
pub(super) fn heading_tag(level: u8) -> ElementTag {
    match level {
        0 | 1 => ElementTag::Heading1,
        2 => ElementTag::Heading2,
        3 => ElementTag::Heading3,
        4 => ElementTag::Heading4,
        5 => ElementTag::Heading5,
        _ => ElementTag::Heading6,
    }
}

/// A heading of `level` whose `id` is `anchor`; an empty anchor sets no `id`.
fn heading_node(level: u8, anchor: &str, inlines: &[WikiInline]) -> RenderNode {
    let index = usize::from(level.clamp(1, 6)) - 1;
    let mut heading = RenderElement::new(heading_tag(level), HEADING_CLASSES[index]);
    if !anchor.is_empty() {
        heading = heading.with_attribute("id", anchor);
    }
    heading.with_children(inline_nodes(inlines)).into_node()
}

/// A paragraph of `inlines`.
fn paragraph_node(inlines: &[WikiInline]) -> RenderNode {
    RenderElement::new(ElementTag::Paragraph, PARAGRAPH_CLASS)
        .with_children(inline_nodes(inlines))
        .into_node()
}

/// A bulleted list, or a numbered one that counts from `start`.
fn list_node(ordered: bool, start: Option<u64>, items: &[WikiListItem]) -> RenderNode {
    let mut list = if ordered {
        RenderElement::new(ElementTag::NumberedList, NUMBERED_LIST_CLASS)
    } else {
        RenderElement::new(ElementTag::BulletList, BULLET_LIST_CLASS)
    };
    if let (true, Some(start)) = (ordered, start) {
        list = list.with_attribute("start", start.to_string());
    }
    list.with_children(items.iter().map(list_item_node).collect())
        .into_node()
}

/// One list item; a task item leads with a disabled checkbox ticked as `checked` says.
fn list_item_node(item: &WikiListItem) -> RenderNode {
    let content = block_nodes(&item.blocks);
    let Some(checked) = item.checked else {
        return RenderElement::new(ElementTag::ListItem, LIST_ITEM_CLASS)
            .with_children(content)
            .into_node();
    };
    let label = content
        .iter()
        .map(RenderNode::text_content)
        .collect::<Vec<_>>()
        .join(" ");
    let mut checkbox = RenderElement::new(ElementTag::Checkbox, CHECKBOX_CLASS)
        .with_attribute("type", "checkbox")
        .with_attribute("disabled", "")
        .with_attribute("aria-label", label.trim());
    if checked {
        checkbox = checkbox.with_attribute("checked", "");
    }
    let text = RenderElement::new(ElementTag::Division, TASK_TEXT_CLASS).with_children(content);
    RenderElement::new(ElementTag::ListItem, TASK_ITEM_CLASS)
        .with_children(vec![checkbox.into_node(), text.into_node()])
        .into_node()
}

/// A callout of `kind`: its label, then its blocks.
fn callout_node(kind: WikiCalloutKind, blocks: &[WikiBlock]) -> RenderNode {
    let style = callout_style(kind);
    let label = RenderElement::new(ElementTag::Paragraph, style.label_class)
        .with_children(vec![RenderNode::text(style.label)])
        .into_node();
    let text = RenderElement::new(ElementTag::Division, CALLOUT_TEXT_CLASS)
        .with_children(block_nodes(blocks))
        .into_node();
    RenderElement::new(ElementTag::Division, style.box_class)
        .with_attribute("role", "note")
        .with_children(vec![label, text])
        .into_node()
}

/// A code block holding `text`, tagged with its `language` when the fence named one.
fn code_node(language: Option<&str>, text: &str) -> RenderNode {
    let mut code = RenderElement::new(ElementTag::Code, CODE_BLOCK_TEXT_CLASS);
    if let Some(language) = language.filter(|language| !language.is_empty()) {
        code = code.with_attribute("data-language", language);
    }
    RenderElement::new(ElementTag::Preformatted, CODE_BLOCK_CLASS)
        .with_children(vec![code
            .with_children(vec![RenderNode::text(text)])
            .into_node()])
        .into_node()
}

#[cfg(test)]
#[path = "tests/block_mapping.rs"]
mod tests;
