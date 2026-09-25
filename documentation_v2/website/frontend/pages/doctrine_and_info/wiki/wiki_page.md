**Status:** live

# Doctrine wiki page

The `/wiki` and `/wiki/:slug` page, titled "SOPs & Manuals" in the sidebar: signed-in members read
the community's standard operating procedures and manuals from a category-grouped index beside the
manual being read, and an administrator edits a manual's Markdown in place.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/`](/apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/):
  `page.rs` holds the route component `WikiPage`, the list fetch, the slug resolution and the
  drafts; `category_nav.rs` the index; `markdown_article.rs` the reading pane and the editor;
  `markdown.rs` the Markdown renderer. The folder's
  [README](/apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/README.md) describes each
  file.
- Entry: both routes, their tier and their layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/README.md#routes).
- Related: the [vehicle database page](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md),
  which holds vehicle identification apart from the manuals; the
  [API](/documentation_v2/glossary.md#api)'s
  [community content domain](/apps/website/api_v2/src/community_content/README.md), which owns
  the wiki routes.

## Behaviour

The page body sits in `AuthGate`; the session, loading, failure and empty texts are in the
README's [States](/apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/README.md#states).

### Reading

1. The page fetches every manual, bodies included, in one request, and both panes read that list;
   it never fetches a single manual.
2. The index, headed "SOPs & Manuals", groups the manuals by category in the order each category
   first appears in the list, which the API sorts by navigation order and then title. A manual
   with no category stays out of the index.
3. "Search manuals..." narrows the index to the manuals whose title or category matches.
4. The route's `:slug` opens the manual it names. `/wiki`, or a slug that names no manual, opens
   the first manual of the list.
5. A click in the index navigates to `/wiki/<slug>`, so every manual has a shareable address and
   the browser's back button walks the reading history.
6. The reading pane shows "Last updated <YYYY-MM-DD>", the category, the title and the rendered
   body.

### The Markdown subset

The renderer knows a fixed subset, and anything else renders as text:

- `#` and `##` headings, `- ` and `* ` bullet runs, and paragraphs separated by blank lines;
- `**bold**`, `*italic*` and `` `code` `` inside a line;
- a run of `>` lines as one callout, whose leading tag picks its style and label: `[!CRITICAL]` or
  `[!CAUTION]` "CRITICAL RULE", `[!WARNING]` "WARNING", `[!TIP]` "PRO-TIP", and `[!NOTE]`,
  `[!INFO]` or no tag "NOTE". Text after a known tag replaces the label; an unknown tag renders a
  note and stays in the text.

Every piece becomes a text node, so HTML inside a manual shows as written and never runs.

### Editing

1. A signed-in administrator sees "[ READ ]" and "[ EDIT ]" above the manual; the switch reads
   the signed-in, reactive role, so it never shows while the session restores.
2. "[ EDIT ]" swaps the rendered body for the raw Markdown in a text area. Each manual keeps its
   own unsaved draft for as long as the page stays open: switching back to reading renders the
   draft, and opening another manual keeps it.
3. "Save" ("Saving…" while it runs) writes the draft with the manual's stored category, title,
   icon and navigation order, then drops the draft and refetches the list. A refusal shows the
   API's sentence, or "Failed to save wiki page", under the button.
4. Opening another manual returns the pane to reading and clears the save error.
5. The page creates no manual and edits no title, category, icon or order: the API's write can,
   but the page offers only the body.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/README.md#data)
lists each call with the fields it reads or sends. Server-side:

- `GET /api/v1/wiki` (`list_wiki` in
  `apps/website/api_v2/src/community_content/handlers/wiki_knowledgebase.rs`): any signed-in
  member; every manual with its body, ordered by `nav_order` and then title.
- `PUT /api/v1/wiki/{slug}` (`upsert_wiki_page`, same file): `admin` only; creates the manual or
  replaces every field of it, recording the administrator and the time. A blank slug or one with
  leading or trailing whitespace is refused with 400 rather than trimmed, so a typo never
  overwrites another manual; `icon` and `nav_order` must be present, and an empty category, title
  or body answers 400 "category, title and body_md are required".
- `GET /api/v1/wiki/{slug}` exists for one manual; the page does not call it.

## Design

- A `GlassSplit` with a 17rem index beside the reading pane; the index groups carry their
  category as a heading, and the selected manual is highlighted.
- Callouts are coloured boxes with the label on top; the manual's `icon` is stored and resent on
  save but not shown.
- Design target: the archived platform spec's
  [SOPs & Manuals section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#6-sops--manuals-the-wiki),
  a design-phase specification; no visual reference set exists. The built page differs:
  - the index lists manuals grouped by their stored category, not a fixed list of topics, and the
    vehicle database is a page of its own;
  - the renderer has no tables; headings stop at level two;
  - the index is a fixed 17rem column rather than a quarter of the width;
  - editing is built, which the spec did not describe.

## Open work

- [T-940.9 — Wiki: headings, links, images, tables, checklists, revisions](/documentation_v2/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-940_9_plan.md)): the renderer gains deeper
  headings, links, images, tables and checklists, and every save keeps a revision the page lists.
- [T-085 — Wiki markdown renderer](/.ai/tickets/T-085.toml) (deferred, no plan): Markdown
  rendering at `/wiki`, which the fixed subset above already provides.

## Decisions

- One fetch holds every manual: the index and the reading pane read the same list, so switching
  manuals costs no request and the two panes never disagree.
- A manual is chosen by the URL, not local state: `/wiki/<slug>` links and the back button work.
- The renderer builds text nodes, never HTML: a manual cannot inject markup into a page every
  member loads.
- A slug is refused, not trimmed, on write: the write replaces a whole row, and trimming would
  turn a typo into an overwrite of another manual.
- Editing is for administrators only and keeps per-manual drafts until a save succeeds.
