**Status:** live

# Announcements page

The `/announcements` and `/announcements/:id` pages in the
[command center](/documentation_v2/glossary/a_to_f.md#command-center), headed "Comms Link": a signed-in
member reads the unit's published announcements in a master list and opens one in a reading pane
beside it. The `:id` in the address decides which announcement is open, so a link to one
announcement opens exactly that one.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/command_center/announcements/`](/apps/website/frontend/src/v2/pages/command_center/announcements/):
  `page.rs` holds the route component `AnnouncementsPage` and the list fetch; `article_feed.rs`
  the order of the list, its rows and the split pane; `article_viewer.rs` the reading pane. The
  folder's [README](/apps/website/frontend/src/v2/pages/command_center/announcements/README.md)
  describes each file.
- Entry: both routes render `AnnouncementsPage`; their tier and layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/command_center/announcements/README.md#routes).
  The sidebar lists the page as "Announcements" in the "Command Center" section.
- Related: the [content manager page](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md),
  where administrators write, publish, pin and push announcements to Discord; the
  [dashboard page](/documentation_v2/website/frontend/pages/command_center/dashboard/dashboard_page.md),
  whose "Recent Intelligence" rows link to `/announcements/{id}`; the
  [API](/documentation_v2/glossary/a_to_f.md#api)'s
  [community content domain](/apps/website/api_v2/src/community_content/README.md), which serves
  the feed.

## Behaviour

### The list

1. The page body sits in `AuthGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`), which
   shows the session states of the README's
   [States](/apps/website/frontend/src/v2/pages/command_center/announcements/README.md#states)
   until the viewer is signed in.
2. The signed-in half fetches the feed once and shows "Loading…", then "Failed to load data." or
   the board. The feed carries every body, so opening an announcement never fetches again.
3. The master column, headed "Comms Link", lists pinned announcements first and keeps the
   server's order within each group. A row shows the date, a yellow dot for a pinned
   announcement, the title (or "Untitled Post"), a tag badge and a two-line preview: the
   `snippet`, else the body's first paragraph.
4. The tag badge reads the tag in capitals with underscores as spaces ("MODPACK UPDATE"), or
   `NOTICE` when there is none; `modpack_update`, `event` and `important` each have a colour and
   any other tag is neutral.
5. The filter icon beside the heading does nothing: the page has no category filter.
6. With no announcements the list reads "No announcements yet."

### Opening one

1. A row click navigates to `/announcements/{id}`; it sets no signal. The open announcement is a
   memo over the route's `:id`, so the address bar is the only record of what is open, the back
   button steps through what the viewer read, and the list highlights the open row.
2. A bare `/announcements` opens nothing: the reading pane reads "Select a broadcast to read."
3. An `:id` the loaded feed does not hold reads "That broadcast is no longer in the feed."
4. The reading pane shows the tag badge, a "Pinned" chip, a "Pushed to Discord" chip when the
   announcement went to Discord, the title as the heading, the author (or "Command" when there is
   none), the local publish time, the thumbnail when its address is an `http(s)` URL, and the
   body.
5. The body is plain text: the pane splits it at blank lines into paragraphs, keeps single line
   breaks, and renders each paragraph as text, escaped once. Markdown and inline markup show as
   written.

### Known discrepancies

- The byline promises an author and shows the `author_id` field
  (`reader` in `apps/website/frontend/src/v2/pages/command_center/announcements/article_viewer.rs`),
  which the API fills with the publishing administrator's Discord id
  (`create_announcement` in
  `apps/website/api_v2/src/community_content/handlers/announcements_admin.rs`), so readers see a
  number rather than a name.
- The page asks for the feed without paging and so receives the first 20 announcements
  (`list_announcements` in
  `apps/website/api_v2/src/community_content/handlers/announcements_public.rs`). A link to an
  older announcement reads "That broadcast is no longer in the feed." although the API still
  serves it at `GET /api/v1/announcements/{id}`, which the page never calls.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/command_center/announcements/README.md#data)
lists the call and the fields the page reads. Server-side:

- `GET /api/v1/announcements` (`list_announcements` in
  `apps/website/api_v2/src/community_content/handlers/announcements_public.rs`), for any
  signed-in member: the published announcements that are not deleted, pinned first and then
  newest first, one page of `limit` (20 by default, at most 100) from `offset`, with `total`
  counting them all. Drafts never appear. The API leaves `snippet` and `thumbnail_url` out when
  they are empty, and the page draws nothing in their place.
- The snippet is written when the announcement is saved: the one the author typed, cut to 200
  characters, else one derived from the body (`snippet_from` in
  `apps/website/api_v2/src/community_content/handlers/announcements_admin.rs`). The page's own
  fallback to the body's first paragraph covers a row without one.
- `GET /api/v1/announcements/{id}` (`get_announcement`, same file): one published announcement,
  404 "announcement not found" for any other, 400 "invalid id" for an id that is not a UUID. The
  page does not call it.

The page writes nothing and stores nothing in the browser.

## Design

- A full-height `SplitPane` over the topographic backdrop: the "Comms Link" master column of
  list rows and the reading pane, an article column at most 48rem wide.
- The page is a reading board in the split-pane pattern the schedule also uses: the list stays
  in view while one item is read beside it.
- Design target: the archived platform spec's
  [Announcements section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#2-announcements-news-feed)
  and a design-phase layout sketch, a dispatch feed beside an article viewer with a byline of
  author and date. No blueprint set exists for this page. The built page differs from the spec:
  - the spec asked for a centred blog column of large news cards, each with a thumbnail, a date
    pill, a title, the author's name, a snippet and a "Read Full Briefing" button; the built page
    is the list-beside-reader layout of the sketch, with the thumbnail only in the reading pane;
  - the sketch shows the article as Markdown; the built page renders plain text;
  - the byline shows the author's Discord id, not a name (see Known discrepancies).
- The content manager's
  [announcements manager blueprint](/documentation_v2/website/frontend/pages/administration/content_manager/visual_references/announcements_manager_blueprint/README.md)
  depicts the writing side, not this page.

## Open work

- [T-087 — CMS rich text editor](/.ai/tickets/T-087.toml) (deferred, no plan): announcements gain
  rich text; the reading pane would then need a renderer for it, which the page's
  plain-text rule rules out until the write path sanitises the body.
- [T-1028 — Fix re-publishing a pinned announcement unpinning it](/.ai/tickets/T-1028.toml)
  (idea, no plan): a pinned announcement stays pinned, and so first in this list, when an
  administrator publishes it again.
- [T-1105 — Fix milestone announcement body date disagreeing with its title](/.ai/tickets/T-1105.toml)
  (idea, no plan): the pinned milestone announcement that `cargo xtask mod seed-announcement`
  inserts, and this page lists, states the same date in its title and its body.

## Decisions

- The address is the selection: a row click navigates instead of setting state, so every
  announcement has a link that opens it, and back and forward move between announcements.
- One fetch serves the list and the reader: the feed carries every body, so the reader can never
  show a stale or half-loaded announcement.
- The body renders as escaped plain text: it is stored unsanitised, so rendering it as markup
  would need a sanitiser on the write path first
  (`body_paragraphs_preserve_bare_angle_brackets` in
  `apps/website/frontend/src/v2/pages/command_center/announcements/tests/announcements.rs` keeps
  the single escape).
- A thumbnail loads only from an `http(s)` URL, checked again at render although the writer
  already checks it (`announcement_thumbnail_emits_src_only_for_http_urls`, same file).
