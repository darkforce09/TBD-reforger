# Doctrine wiki page

The `/wiki` and `/wiki/:slug` page: the community's standard operating procedures and manuals, a
category-grouped index beside the manual being read and its revision history. An administrator can
edit a manual's Markdown, save it against the revision the edit started from, and restore an older
revision.

## Contents

```text
apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/
├── api_paths.rs     the paths of every wiki request and the revision page count
├── article/         the open manual: its fetch, header, body or editor, and per-manual state
├── blocks/          the block renderer: typed blocks to a render tree to text-node views
├── category_nav.rs  the index pane: the search box and the manual links grouped by category
├── display_text.rs  the calendar day, a revision's byline and the load-failure sentences
├── mod.rs           the module tree; re-exports `WikiPage`
├── page.rs          `WikiPage`: the page-list fetch, slug resolution, shared state, split view
├── page_state.rs    `WikiPageState`, the read/edit mode and the draft with its base revision
├── revisions/       the revision panel, the older revision on view and its restore
├── saving/          the save and restore bodies, the `PUT`, and the refusal notice
└── tests/           unit tests for the paths, drafts, texts, slug fallback and source guards
```

## How it works

`WikiPage` renders inside `AuthGate`. `page.rs` fetches the page summaries (`GET /wiki`) into a
`LocalResource` and builds one `WikiPageState` — the `AuthStore`, the `is_admin` memo, the search
text, the read/edit mode, the per-slug drafts and the page-list resource — that every pane reads.
A memo over the list decides between the loading line, the failure line and the board, so a
refetch after a save keeps the board and the open manual mounted. The route's `:slug` picks the
open manual when it names one in the list; otherwise the first row opens, which the
[API](/documentation_v2/glossary/a_to_f.md#api) orders by `nav_order`, then title, then slug. A click
in the index navigates to `/wiki/<slug>`; a change of manual mounts a fresh article pane and
returns the mode to reading.

```text
GET /wiki ──> page.rs (WikiPageState) ──> category_nav.rs (index)
                    │
                    └── slug ──> article/ ── GET /wiki/{slug} ──> header, body (blocks/ or editor)
                                    │                         └─> revisions/ ── GET …/revisions
                                    └── save / restore ──> saving/ ── PUT /wiki/{slug}
```

The article pane fetches the manual on its own and renders its `blocks` — the server's parse of
`body_md` — through `blocks/`, which maps every block and inline to a render tree of elements,
attributes and text, then builds views from text nodes only. Headings carry their anchor as `id`,
so the router's in-page `#anchor` navigation scrolls to them; every `href` and `src` is re-checked
with `core::utils::safe_url` and an unsafe one renders as text. The revision panel beside the
article pages through the history ten at a time; choosing a revision shows its blocks in place of
the current text.

`is_admin` is a memo over `has_min_role_authed` and the session's
[role](/documentation_v2/glossary/n_to_z.md#role), so "[ READ ]", "[ EDIT ]", "Save" and "Restore
this revision" appear only for a signed-in administrator. Typing starts a draft that records the
revision it began from; "Save" sends that `base_revision`, so a manual that moved on answers 409
and the notice offers to discard the draft and load the latest revision. A restore sends the old
revision's fields over the current revision after a confirmation. A refused save shows the
conflict, each refused markup construct with its line (422), the body-size refusal (400) or the
request-size refusal (413). There is no client-side Markdown parser: reading a manual that has an
unsaved draft shows the saved text under a notice that the draft exists.

Callouts keep three colour families: note, tip and info in primary blue ("NOTE", "PRO-TIP",
"INFO"), important and warning in tactical yellow ("IMPORTANT", "WARNING"), caution and critical in
red ("CAUTION", "CRITICAL RULE").

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/wiki` | `WikiPage` | route tier `none`; the manuals render only for a signed-in viewer, editing and restoring only for `admin` | full-bleed inside the navigation frame; breadcrumb Doctrine & Info / SOPs & Manuals |
| `/wiki/:slug` | `WikiPage` | as `/wiki` | as `/wiki` |

## Data

- `GET /api/v1/wiki`: `DataEnvelope<dto::wiki::WikiPageSummary>`; the index and the slug
  resolution read `slug`, `category` and `title`.
- `GET /api/v1/wiki/{slug}`: `dto::wiki::WikiArticle`; the pane shows `updated_at`, `revision`,
  `category`, `title` and the rendered `blocks`, the editor starts from `body_md`, and a save
  resends `category`, `title`, `icon` and `nav_order`.
- `GET /api/v1/wiki/{slug}/revisions?page=&per_page=10`: `dto::wiki::WikiRevisionPage`, newest
  first; each row shows `revision`, `title`, `created_at` and `author_id`.
- `GET /api/v1/wiki/{slug}/revisions/{revision}`: `dto::wiki::WikiRevision`; its `blocks` render
  in the article area and a restore sends its fields.
- `PUT /api/v1/wiki/{slug}`: `dto::wiki::WikiSaveRequest` with a numeric `base_revision`, through
  `api_put_keeping_refusal`; the answer is the saved `WikiArticle`, and a refusal's `details`
  parse as `dto::wiki::WikiSaveRefusal`.
- The page reads the route's `:slug` and the `AuthStore` and `Toasts` from context. The requests
  run in the browser build only; a native build shows the failure lines.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load wiki." |
| no manuals | "No manuals yet." |
| no manual resolves | "Select a manual." |
| manual loading | "Loading…" in the detail pane |
| manual failed | "Failed to load this manual.", "Could not find this manual." on 404, or "Your session has ended. Sign in again to read the manuals." |
| reading | "SOPs & Manuals" and the "Search manuals..." box over the category groups; the manual with "Last updated <YYYY-MM-DD>" ("—" without a date), "Revision <n>", its category, title and rendered body; "Revision history" with "Revision <n>", a "current" mark, the title, "saved <day> by <editor>" and "Page <p> of <q>" between "Newer" and "Older" |
| history failed | "Failed to load the revision history." |
| revision on view | "Revision <n> · saved <day> by <editor> · <title>", "Back to the current revision", and the revision's text |
| administrator | also "[ READ ]" and "[ EDIT ]"; on an older revision "Restore this revision" |
| editing | the raw Markdown in a text area, and "Save" ("Saving…" while it runs) |
| unsaved draft while reading | "You have an unsaved draft of this manual, started from revision <b>. Below is the saved revision <n>; …" |
| restore confirmation | "Restore this revision?", "Cancel" and "Restore revision <n>" |
| saved or restored | the notice "Saved as revision <n>" or "Revision <old> restored as revision <n>" |
| save refused | the alert with the conflict sentence and "Discard my draft and load revision <n>" (a restore: "Load revision <n>"), "Not saved: the markup has <k> problems." with "Line <l>: <detail>" rows, the size sentences, or the API's sentence |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `api_put_keeping_refusal`, `ApiRefusal`,
  `Fetched`, `ApiFailure`, `DataEnvelope`, `dto::wiki`), `crate::v2::core::auth`
  (`has_min_role_authed`, `Role`, the `AuthStore` context), `crate::v2::core::utils::safe_url`,
  `crate::v2::core::ui` (`AuthGate`, `Dialog`, the `split_pane` primitives, the toasts) and
  `leptos_router` (the route parameters and navigation).
- Used by: the `/wiki` and `/wiki/:slug` routes in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "SOPs & Manuals" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; `wiki_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins every production file of
  the page for its guard tests; the DOM oracle's `wiki` and `wikislug` captures in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`, answered from
  `GET__wiki.json`, `GET__wiki__field-manual.json` and `GET__wiki__field-manual__revisions.json`.
- Rules: the edit, save and restore controls are gated by the `is_admin` memo over
  `has_min_role_authed`, never by the browse-mode `has_min_role`
  (`admin_affordance_uses_authed_reactive_role` in `tests/wiki.rs`); no file writes inner HTML
  (`wiki_source_never_writes_inner_html`); an unknown slug opens the first manual
  (`slug_resolution_falls_back_to_first`); categories keep their first-seen order
  (`categories_preserve_first_seen_order`); a draft keeps the revision it started from
  (`wiki_draft_keeps_its_base_revision_when_the_article_moves_on` in `tests/page_state.rs`); a slug
  is one percent-encoded path segment (`wiki_paths_keep_a_slug_to_one_segment`); view! attribute
  values are braced unless literal, path or closure (`view_attributes_*` in
  `tests/view_attributes.rs`, which also scans the vehicles, personnel and audit-log pages).

## Related documentation

- [Doctrine wiki page](/documentation_v2/website/frontend/pages/doctrine_and_info/wiki/wiki_page.md)
  — the page's behaviour and design.
- [Community content domain](/apps/website/api_v2/src/community_content/README.md) — the wiki
  routes.
- [Administration and community content](/documentation_v2/website/api_v2/verification_evidence/administration_and_content.md#wiki-markup-and-revisions)
  — the markup service, the revision storage and the save refusals.
