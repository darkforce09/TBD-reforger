**Status:** live

# Doctrine wiki page

The `/wiki` and `/wiki/:slug` page, titled "SOPs & Manuals" in the sidebar: signed-in members read
the community's standard operating procedures and manuals from a category-grouped index beside the
manual being read and its revision history, and an administrator edits a manual's Markdown in
place and restores older revisions.

## Where it lives

- Code: [`apps/frontend/src/v2/pages/doctrine_and_info/wiki/`](/apps/frontend/src/v2/pages/doctrine_and_info/wiki/):
  `page.rs` holds the route component `WikiPage`, the page-list fetch, the slug resolution and the
  shared state; `category_nav.rs` the index; `article/` the open manual's fetch, header, body and
  editor; `blocks/` the renderer of the server's typed blocks; `revisions/` the history panel,
  the older revision on view and its restore; `saving/` the save and restore requests and their
  refusal notice. The folder's
  [README](/apps/frontend/src/v2/pages/doctrine_and_info/wiki/README.md) describes each
  file.
- Entry: both routes, their tier and their layout are in the README's
  [Routes](/apps/frontend/src/v2/pages/doctrine_and_info/wiki/README.md#routes).
- Related: the [vehicle database page](/documentation/apps/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md),
  which holds vehicle identification apart from the manuals; the
  [API](/documentation/glossary/a_to_f.md#api)'s
  [community content domain](/apps/api/src/community_content/README.md), which owns
  the wiki routes and the markup service.

## Behaviour

The page body sits in `AuthGate`; the session, loading, failure and empty texts are in the
README's [States](/apps/frontend/src/v2/pages/doctrine_and_info/wiki/README.md#states).

### Reading

1. The page fetches the page summaries (slug, category, title, icon, order, revision, update
   time) for the index, and fetches the open manual on its own when it opens.
2. The index, headed "SOPs & Manuals", groups the manuals by category in the order each category
   first appears in the list, which the API sorts by navigation order, then title, then slug. A
   manual with no category stays out of the index.
3. "Search manuals..." narrows the index to the manuals whose title or category matches.
4. The route's `:slug` opens the manual it names. `/wiki`, or a slug that names no manual, opens
   the first manual of the list.
5. A click in the index navigates to `/wiki/<slug>`, so every manual has a shareable address and
   the browser's back button walks the reading history.
6. The reading pane shows "Last updated <YYYY-MM-DD>", "Revision <n>", the category, the title and
   the rendered body.

### Rendering

The server parses each manual's Markdown into typed blocks, and the page renders those blocks; it
holds no Markdown parser of its own. Everything a manual can hold — and the Markdown that makes it
— is shown by the "Wiki Formatting Guide" manual:

- headings of levels one to six, each with an anchor made from its text, so a `#anchor` link on
  the page scrolls to it;
- paragraphs, **bold**, *italic*, ~~struck-through~~ text, inline code and line breaks;
- bulleted and numbered lists (a numbered list keeps its first number) and checklists, whose
  boxes render ticked or empty and cannot be clicked;
- links: a link to another site opens in a new tab with `rel="noopener noreferrer nofollow"`, a
  link to a site path or a heading stays in the tab;
- images from an `https://` address or a site path such as `/uploads/…`, loaded lazily, sending
  no referrer, with their alt text;
- tables whose columns keep their left, centred, right or unset alignment;
- callouts in three colours: "NOTE", "PRO-TIP" and "INFO" in blue, "IMPORTANT" and "WARNING" in
  yellow, "CAUTION" and "CRITICAL RULE" in red;
- block quotes, code blocks and horizontal rules.

The page builds every piece as a DOM text node or a named attribute, never as HTML. It re-checks
each link and image address against the same allowlist the server applies: a link whose address
fails renders as its text, and an image whose address fails renders as its alt text.

### History

1. "Revision history", beside the manual (below it on narrow screens), lists the manual's
   revisions newest first, ten to a page, each with its number, title, day and editor; the
   current revision carries a "current" mark. "Newer" and "Older" page through the list.
2. Choosing a revision shows it in place of the current text, under a banner naming it, with
   "Back to the current revision".
3. An administrator viewing any revision but the current one sees "Restore this revision". It
   asks first ("Restore this revision?"), then saves that revision's text, title, category, icon
   and order as a new revision over the current one; every earlier revision stays in the history.

### Editing

1. A signed-in administrator sees "[ READ ]" and "[ EDIT ]" above the manual; the switch reads
   the signed-in, reactive role, so it never shows while the session restores.
2. "[ EDIT ]" swaps the rendered body for the raw Markdown in a text area. The first keystroke
   starts a draft that remembers the revision the edit began from; each manual keeps its own draft
   for as long as the page stays open. Reading a manual that has a draft shows the saved text
   under a notice that the draft exists, since only a save produces rendered blocks.
3. "Save" ("Saving…" while it runs) sends the draft with the revision it began from and the
   manual's stored category, title, icon and navigation order. An accepted save drops the draft,
   returns to reading, reloads the manual, its history and the index, and shows "Saved as revision
   <n>".
4. A refused save or restore shows an alert above the body:
   - the manual changed since the edit began (409): both revision numbers and "Discard my draft
     and load revision <n>" ("Load revision <n>" after a restore, which keeps any draft);
   - the markup holds refused constructs (422): "Not saved: the markup has <k> problems." and one
     "Line <l>: <detail>" per construct — raw HTML, an unsafe link or image address, or nesting
     deeper than 16;
   - the body is over 262 144 bytes (400) or the request over the server's limit (413);
   - otherwise the API's sentence, or "Failed to save wiki page".
5. Opening another manual returns the pane to reading.
6. The page creates no manual and edits no title, category, icon or order except through a
   restore: the API's write can, but the page offers only the body.

## Data

The README's [Data](/apps/frontend/src/v2/pages/doctrine_and_info/wiki/README.md#data)
lists each call with the fields it reads or sends. Server-side, the handlers are in
`apps/api/src/community_content/handlers/wiki_knowledgebase/` and the parse in
`apps/api/src/community_content/services/wiki_markup/`:

- `GET /api/v1/wiki` (`list_wiki`): any signed-in member; every page's summary, ordered by
  `nav_order`, then title, then slug.
- `GET /api/v1/wiki/{slug}` (`get_wiki_page`): one article — the summary fields, `id`,
  `body_md`, `updated_by` and the `blocks` parsed from the body, made safe to render.
- `GET /api/v1/wiki/{slug}/revisions` (`list_wiki_revisions`): one page of the history, newest
  first, `page` from 1 and `per_page` defaulting to 20.
- `GET /api/v1/wiki/{slug}/revisions/{revision}` (`get_wiki_revision`): one revision with its
  fields and parsed blocks.
- `PUT /api/v1/wiki/{slug}` (`save_wiki_page`): `admin` only. `base_revision` must equal the
  page's current revision (`null` creates a page); the save updates the page, appends the
  revision and an audit line in one transaction, and answers the article. The refusals are the
  ones the page words above: 409 `wiki_revision_conflict` with `current_revision`, 422
  `wiki_markup_refused` with `findings`, 400 `wiki_body_too_large`, and 413 for an oversized
  request.

The save rules, the revision storage and the markup service are set out in
[Administration and community content](/documentation/apps/api/verification_evidence/administration_and_content.md#wiki-markup-and-revisions).

## Design

- A `GlassSplit` with a 17rem index beside the detail pane; the index groups carry their category
  as a heading, and the selected manual is highlighted. The detail pane puts the revision panel in
  an 18rem column beside the manual on wide screens and under it on narrow ones.
- Callouts are coloured boxes with the label on top; the manual's `icon` is stored and resent on
  save but not shown.
- Design target: the archived platform spec's
  [SOPs & Manuals section](/documentation/archive/go_and_react_era_design/platform_context_handoff.md#6-sops--manuals-the-wiki),
  a design-phase specification; no visual reference set exists. The built page differs:
  - the index lists manuals grouped by their stored category, not a fixed list of topics, and the
    vehicle database is a page of its own;
  - the index is a fixed 17rem column rather than a quarter of the width;
  - editing, the revision history and restoring are built, which the spec did not describe.

## Open work

- [T-940.9 — Wiki: headings, links, images, tables, checklists, revisions](/documentation/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation/tickets/plans/t-940_9_plan.md)): the rendering, the revision
  history and the restore described above are its scope; the ticket's status is kept in the
  registry.
- [T-085 — Wiki markdown renderer](/.ai/tickets/T-085.toml) (deferred, no plan): Markdown
  rendering at `/wiki`, which the server's parse and the block renderer above provide.

## Decisions

- One parser: the server parses the Markdown and the page renders its typed blocks, so what a
  member reads is exactly what the save validated, and the page carries no second parser to drift.
- The index reads the summaries and each manual is fetched when it opens, so the list stays small
  however long the manuals grow.
- A manual is chosen by the URL, not local state: `/wiki/<slug>` links and the back button work.
- The renderer builds text nodes, never HTML, and re-checks every address: a manual cannot inject
  markup or a script address into a page every member loads, even if the server's check slipped.
- A save names the revision its edit began from, so two administrators editing at once never
  overwrite each other silently; the later save is refused and offered a reload.
- A restore is a save of the old revision over the current one: the history only grows, and a
  restore over a page that moved on is refused like any stale save.
- Editing is for administrators only and keeps per-manual drafts until a save succeeds.
