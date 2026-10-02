# Wiki markup service

Reads a doctrine wiki page's markdown into the typed tree the wiki page renders, and lists every
construct a save refuses. The same reading serves the article and revision reads, where each
refused construct is replaced by a safe form, and the save, which answers 422 when the list is not
empty.

## Contents

```text
apps/api/src/community_content/services/wiki_markup/
├── callout_markers.rs  GitHub alert kinds and the `[!KIND]` bracket markers that make a quote a callout
├── document_tree.rs    `WikiBlock`, `WikiInline`, `WikiListItem`, `WikiTableAlignment`, `WikiCalloutKind`
├── heading_anchors.rs  heading slugs and the `-2`, `-3` suffixes that keep a page's anchors unique
├── markup_findings.rs  `WikiMarkupFinding`, its codes and wording, and the byte-offset-to-line index
├── mod.rs              `read_markup`, `MarkupReading` and `MAX_NESTING_DEPTH`
├── open_frames.rs      the nodes held open while reading, and what each becomes when it closes
├── tests/              block, inline, refusal, anchor and callout goldens, and the formatting guide seed check
└── tree_builder.rs     the pulldown-cmark event loop that builds the tree and records the findings
```

## How it works

`read_markup(body_md)` runs pulldown-cmark 0.13 with tables, task lists, strikethrough and GitHub
blockquote alerts enabled; footnotes, heading attributes, math, metadata blocks and smart
punctuation stay off, so their syntax reads as text. The tree builder keeps a stack of open nodes
and never recurses per event, so no body can exhaust the stack while it is read.

| Block | Fields |
|---|---|
| `heading` | `level` 1 to 6, `anchor`, `inlines` |
| `paragraph` | `inlines`; a soft line break is a `\n` in the text |
| `list` | `ordered`, `start` on a numbered list, `items` of `{checked?, blocks}` |
| `table` | `alignments` (`none`, `left`, `center`, `right`), `header` cells, `rows` of cells |
| `callout` | `kind` (`note`, `tip`, `important`, `info`, `warning`, `caution`, `critical`), `blocks` |
| `quote` | `blocks` |
| `code` | `language` (the fence's trimmed info string, absent when empty), `text` |
| `rule` | none |

Inlines are `text`, `strong`, `emphasis`, `strikethrough`, `code`, `link` (`href`, `external`,
`children`), `image` (`src`, `alt`, `title?`) and `line_break`; adjacent text runs are merged.

- **Callouts.** A GitHub alert (`> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]` or
  `[!CAUTION]` alone on the quote's first line) gives its kind. A plain quote whose first
  paragraph opens with `[!CRITICAL]`, `[!CAUTION]`, `[!WARNING]`, `[!TIP]`, `[!NOTE]` or `[!INFO]`
  in any letter case, followed by whitespace or nothing, becomes that callout with the marker
  removed.
- **Anchors.** A heading's anchor is its text with letters and digits lowercased and every other
  run collapsed to one hyphen (`section` when nothing is left); a repeated anchor takes the first
  free `-2`, `-3`, … suffix, in document order across the whole page.
- **Links and images.** Their URLs pass `core::text::content_url_policy`: a link to `https://`,
  `http://`, `mailto:`, a site path `/…` or a `#fragment`, an image from `https://` or a site path.
  An email autolink gains `mailto:`. `external` is true for absolute `http`, `https` and `mailto`
  targets.

| Refused construct | Finding `code` | Safe form in the blocks |
|---|---|---|
| a link whose target the policy refuses | `unsafe_link_url` | the link's children |
| an image whose source the policy refuses | `unsafe_image_url` | its alt text, or nothing when empty |
| inline or block HTML | `raw_html` | the HTML as literal text |
| a list, quote, callout, strong, emphasis, strikethrough or link deeper than 16 | `nesting_too_deep` | its children, spliced into its parent |

Each finding carries the 1-based line on which the construct starts and a `detail` quoting at most
80 characters of it; a run of over-deep containers yields one finding, at its first flattened
level; findings are ordered by line.

## Boundaries

- Depends on: `pulldown-cmark` (`default-features = false`); `core::text::content_url_policy`
  for the URL rules; serde.
- Used by: the wiki handlers in `community_content/handlers/wiki_knowledgebase/`, which serialize
  the blocks into the article and the revision and answer the findings as
  `details.findings` of `422 wiki_markup_refused`; `community_content/models/wiki.rs`, whose
  article and revision embed `WikiBlock` and whose refusal embeds `WikiMarkupFinding`.
- Rules: the serialized tree and findings match `WikiBlock`, `WikiInline` and `WikiMarkupFinding`
  in `contracts/definitions/wiki-page.schema.json`, which the goldens under `tests/` check
  every output against; the blocks never carry a construct a save refuses; the formatting guide
  page in `apps/api/seeds/wiki_pages.sql` reads with no finding and shows every block
  type, inline type and callout kind (`tests/wiki_markup.rs` checks it).

## Related documentation

- [Administration and community content](/documentation/apps/api/verification_evidence/administration_and_content.md)
  — the wiki markup, revision and save design.
- [Wiki page](/documentation/apps/frontend/pages/doctrine_and_info/wiki/wiki_page.md) — the
  page that renders the blocks.
