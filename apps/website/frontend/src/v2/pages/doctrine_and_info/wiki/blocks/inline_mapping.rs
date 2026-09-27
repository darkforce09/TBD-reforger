//! Inline runs of a manual — text, emphasis, code, links, images, line breaks — as render nodes.
//!
//! **Role:** maps each [`WikiInline`] to the nodes it renders as, re-checking every link target
//! and image source against the content URL policy before it becomes an attribute.
//! **Position:** called by the block and table mappers for the inlines of headings, paragraphs
//! and cells; its nodes become views in the element views.
//! **Signals & state:** none; pure functions.
//! **Invariants:** an `href` is written only when
//! [`safe_link_href`] accepts it, otherwise the link's children render as plain inlines; a `src`
//! is written only when [`safe_image_src`] accepts it, otherwise the image renders as its alt
//! text. A link that leaves the site opens in a new tab with `rel="noopener noreferrer
//! nofollow"`; every image loads lazily and sends no referrer.

use super::render_tree::{ElementTag, RenderElement, RenderNode};
use crate::v2::core::api::dto::wiki::WikiInline;
use crate::v2::core::utils::safe_url::{is_external_link, safe_image_src, safe_link_href};

/// Strong emphasis.
const STRONG_CLASS: &str = "font-semibold text-on-surface";
/// Emphasis.
const EMPHASIS_CLASS: &str = "italic";
/// Struck-through text.
const STRIKETHROUGH_CLASS: &str = "line-through opacity-80";
/// Inline code.
const INLINE_CODE_CLASS: &str =
    "rounded bg-black/40 px-1.5 py-0.5 font-mono text-[0.85em] text-primary";
/// A link.
const LINK_CLASS: &str =
    "text-primary underline decoration-primary/40 underline-offset-2 hover:decoration-primary";
/// An image.
const IMAGE_CLASS: &str = "my-4 h-auto max-w-full rounded-xl border border-white/10";
/// The `rel` of a link that leaves the site.
pub(super) const EXTERNAL_LINK_REL: &str = "noopener noreferrer nofollow";

/// The nodes of a run of inlines, in order.
pub(super) fn inline_nodes(inlines: &[WikiInline]) -> Vec<RenderNode> {
    inlines.iter().flat_map(inline_node).collect()
}

/// The nodes one inline renders as; a refused link yields its children, so there may be several.
fn inline_node(inline: &WikiInline) -> Vec<RenderNode> {
    match inline {
        WikiInline::Text { text } => vec![RenderNode::text(text.as_str())],
        WikiInline::Strong { children } => {
            vec![wrapped(ElementTag::Strong, STRONG_CLASS, children)]
        }
        WikiInline::Emphasis { children } => {
            vec![wrapped(ElementTag::Emphasis, EMPHASIS_CLASS, children)]
        }
        WikiInline::Strikethrough { children } => {
            vec![wrapped(
                ElementTag::Strikethrough,
                STRIKETHROUGH_CLASS,
                children,
            )]
        }
        WikiInline::Code { text } => vec![RenderElement::new(ElementTag::Code, INLINE_CODE_CLASS)
            .with_children(vec![RenderNode::text(text.as_str())])
            .into_node()],
        WikiInline::Link {
            href,
            external,
            children,
        } => link_nodes(href, *external, children),
        WikiInline::Image { src, alt, title } => vec![image_node(src, alt, title.as_deref())],
        WikiInline::LineBreak => vec![RenderElement::new(ElementTag::LineBreak, "").into_node()],
    }
}

/// An element of `tag` and `class` around the nodes of `children`.
fn wrapped(tag: ElementTag, class: &str, children: &[WikiInline]) -> RenderNode {
    RenderElement::new(tag, class)
        .with_children(inline_nodes(children))
        .into_node()
}

/// A link to `href` around `children`.
///
/// `external` is the server's verdict; the link opens in a new tab when either it or the app's
/// own [`is_external_link`] says the target leaves the site. A target the policy refuses renders
/// the children alone, with no anchor.
pub(super) fn link_nodes(href: &str, external: bool, children: &[WikiInline]) -> Vec<RenderNode> {
    let inner = inline_nodes(children);
    let Some(safe_href) = safe_link_href(href) else {
        return inner;
    };
    let mut anchor =
        RenderElement::new(ElementTag::Anchor, LINK_CLASS).with_attribute("href", safe_href);
    if external || is_external_link(safe_href) {
        anchor = anchor
            .with_attribute("target", "_blank")
            .with_attribute("rel", EXTERNAL_LINK_REL);
    }
    vec![anchor.with_children(inner).into_node()]
}

/// An image of `src` described by `alt`, with its optional `title`.
///
/// A source the policy refuses renders as the alt text instead.
pub(super) fn image_node(src: &str, alt: &str, title: Option<&str>) -> RenderNode {
    let Some(safe_src) = safe_image_src(src) else {
        return RenderNode::text(alt);
    };
    let mut image = RenderElement::new(ElementTag::Image, IMAGE_CLASS)
        .with_attribute("src", safe_src)
        .with_attribute("alt", alt)
        .with_attribute("loading", "lazy")
        .with_attribute("decoding", "async")
        .with_attribute("referrerpolicy", "no-referrer");
    if let Some(title) = title.filter(|title| !title.is_empty()) {
        image = image.with_attribute("title", title);
    }
    image.into_node()
}

#[cfg(test)]
#[path = "tests/inline_mapping.rs"]
mod tests;
