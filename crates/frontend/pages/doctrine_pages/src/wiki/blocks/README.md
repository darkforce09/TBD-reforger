# Wiki block renderer

Turns a manual's typed blocks, as the server parsed them from its Markdown, into Leptos views: every
heading, paragraph, list, checklist, table, callout, quote, code block, rule and inline run, built
from text nodes and checked attributes only.

## Contents

```text
crates/frontend/pages/doctrine_pages/src/wiki/blocks/
├── block_mapping.rs   `block_nodes`: each block to its element and class, recursing into nested blocks
├── callout_style.rs   the box colour and label of each of the seven callout kinds
├── element_views.rs   render nodes to views: one Leptos builder per tag, text as text nodes
├── inline_mapping.rs  `inline_nodes`: each inline to its nodes, with the link and image re-check
├── mod.rs             declares the renderer and exposes `render_blocks`
├── render_tree.rs     `RenderNode`, `RenderElement` and `ElementTag`: the tree before any view
├── table_mapping.rs   `table_node`: a scrollable table whose cells keep their column alignment
└── tests/             golden-driven tests of every block and inline, unsafe URLs, alignments, callouts
```

## How it works

`render_blocks` runs two steps. The mappers first build a render tree, plain data that the unit
tests read directly: `block_mapping.rs` maps each `frontend_api_dtos::wiki::WikiBlock` (handing tables to
`table_mapping.rs` and callout looks to `callout_style.rs`), and `inline_mapping.rs` maps each
`WikiInline`. `element_views.rs` then turns the tree into views, choosing the Leptos builder for
each `ElementTag`, setting each attribute by name and writing each text as a text node.

```text
[WikiBlock] ──block_mapping / table_mapping / inline_mapping──> [RenderNode] ──element_views──> views
```

- A heading's element follows its level, clamped into 1–6, and its `anchor` becomes its `id`, so
  the router's in-page `#anchor` navigation finds it.
- A link's `href` passes `safe_link_href` or the link renders its children alone; a link that
  `is_external_link` (or the server) calls external opens with `target="_blank"` and
  `rel="noopener noreferrer nofollow"`. An image's `src` passes `safe_image_src` or the image
  renders as its alt text; a kept image carries `alt`, `loading="lazy"`, `decoding="async"` and
  `referrerpolicy="no-referrer"`.
- A task-list item leads with a disabled checkbox, ticked when `checked`, named by the item's
  text; a numbered list keeps its `start`.
- A table cell carries one alignment class from its column: `text-start` for none,
  `text-left`, `text-center` or `text-right`.
- Callouts: note, tip and info in primary blue ("NOTE", "PRO-TIP", "INFO"); important and warning
  in tactical yellow ("IMPORTANT", "WARNING"); caution and critical in red ("CAUTION", "CRITICAL
  RULE"); each box has `role="note"`.
- A code block is one text node inside `<pre><code>`, tagged `data-language` when the fence names
  one.

## Boundaries

- Depends on: `frontend_api_dtos::wiki` (the block and inline types),
  `frontend_ui::safe_url` (`safe_link_href`, `safe_image_src`, `is_external_link`)
  and the Leptos element builders and `custom_attribute`.
- Used by: the article body (`../article/article_body.rs`) and the revision view
  (`../revisions/revision_view.rs`) through `render_blocks`.
- Rules: no view here is built from HTML text, and the page's source never names inner HTML; an
  unsafe `href` or `src` never reaches an attribute
  (`wiki_inlines_unsafe_href_renders_its_children_as_plain_text`,
  `wiki_inlines_unsafe_src_renders_the_alt_text`); every block, inline, alignment and callout kind
  of the formatting-guide capture maps to its element
  (`wiki_blocks_formatting_guide_maps_every_block_to_its_element`,
  `wiki_inlines_formatting_guide_maps_every_inline_kind`).

## Related documentation

- [Doctrine wiki page](/documentation/crates/frontend/pages/doctrine_pages/wiki/wiki_page.md)
  — what a manual can hold and how it renders.
- [Administration and community content](/documentation/crates/api/api_server/design_notes/administration_and_content.md#markup-service)
  — the server's parse that produces the blocks.
