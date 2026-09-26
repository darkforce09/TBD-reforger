# Doctrine wiki page

The `/wiki` and `/wiki/:slug` page: the community's standard operating procedures and manuals, a
category-grouped index beside the manual being read. An administrator can switch the reading pane
to the raw Markdown source and save it back.

## Contents

```text
apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/
├── category_nav.rs      the index pane: the search box and the manual links grouped by category
├── helpers.rs           `vstr` and `vi64`: the total field reads over the untyped wiki rows
├── markdown.rs          `render_markdown`, the Markdown subset the manuals are written in
├── markdown_article.rs  the reading pane: stamp, category, title, rendered body or raw editor, save
├── mod.rs               the module tree; re-exports `WikiPage`
├── page.rs              `WikiPage`: the list fetch, slug resolution, mode, drafts, split view
└── tests/               unit tests for the slug fallback, category order, date stamp and admin gate
```

## How it works

`WikiPage` renders inside `AuthGate` and fetches every manual, bodies included, in one list; both
panes of the `GlassSplit` read that list, and the page never fetches a single manual. The route's
`:slug` picks the open manual when it names one in the list; otherwise the first row opens, which
the [API](/documentation_v2/glossary/a_to_f.md#api) orders by `nav_order` and then title. A click in the
index navigates to `/wiki/<slug>` instead of setting local state, and a change of manual returns
the pane to reading and clears the save error. Categories group in the order their first manual
appears, a manual without a category stays out of the index, and the search matches a manual's
title or category.

`is_admin` is a memo over `has_min_role_authed` and the session's
[role](/documentation_v2/glossary/n_to_z.md#role), so the edit switch and the save button appear only for
a signed-in administrator. Edits go into `drafts`, one unsaved body per slug; a draft outlives a
switch back to reading, which renders the draft, and only a successful save drops it and refetches
the list. `render_markdown` knows a fixed subset: `#` and `##` headings, `- ` and `* ` bullets,
runs of `>` lines as one callout, and paragraphs, with `**bold**`, `*italic*` and `` `code` ``
spans. A callout's leading tag picks its style and label: `[!CRITICAL]` or `[!CAUTION]` "CRITICAL
RULE", `[!WARNING]` "WARNING", `[!TIP]` "PRO-TIP", and `[!NOTE]`, `[!INFO]` or no tag "NOTE". Text
after a known tag replaces the label; an unknown tag renders a note and stays in its text.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/wiki` | `WikiPage` | route tier `none`; the manuals render only for a signed-in viewer, editing only for `admin` | full-bleed inside the navigation frame; breadcrumb Doctrine & Info / SOPs & Manuals |
| `/wiki/:slug` | `WikiPage` | as `/wiki` | as `/wiki` |

## Data

- `GET /api/v1/wiki`: read as `DataEnvelope<serde_json::Value>`; each row reads `slug`, `title`,
  `category`, `icon`, `nav_order`, `body_md` and `updated_at`.
- `PUT /api/v1/wiki/{slug}`: sends `category`, `title`, `icon`, `nav_order` and the edited
  `body_md`; the reply is read as `serde_json::Value` and only its success counts.
- The page reads the route's `:slug`, the `AuthStore` from context, and writes nothing besides the
  save. The requests run in the browser build only; a native build shows the failure line.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load wiki." |
| no manuals | "No manuals yet." |
| no manual resolves | "Select a manual." |
| reading | "SOPs & Manuals" and the "Search manuals..." box over the category groups; the manual with "Last updated <YYYY-MM-DD>" ("—" without a date), its category, title and rendered body |
| administrator | also "[ READ ]" and "[ EDIT ]" |
| editing | the raw Markdown in a text area, and "Save" ("Saving…" while it runs) |
| save failed | the API's message, or "Failed to save wiki page", under the save button |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `api_put`, `api_error_message`, `DataEnvelope`),
  `crate::v2::core::auth` (`has_min_role_authed`, `Role`, the `AuthStore` context),
  `crate::v2::core::ui` (`AuthGate`, the `split_pane` primitives, `cn`) and
  `leptos_router` (the route parameters and navigation).
- Used by: the `/wiki` and `/wiki/:slug` routes in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "SOPs & Manuals" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; `wiki_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins the six source files for
  the page's test; the DOM oracle's `wiki` and `wikislug` captures in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: the edit and save controls are gated by the `is_admin` memo over `has_min_role_authed`,
  never by the browse-mode `has_min_role` (`admin_affordance_uses_authed_reactive_role` in
  `tests/wiki.rs`); an unknown slug opens the first manual (`slug_resolution_falls_back_to_first`);
  categories keep their first-seen order (`categories_preserve_first_seen_order`); both panes read
  the one fetched list.

## Related documentation

- [Doctrine wiki page](/documentation_v2/website/frontend/pages/doctrine_and_info/wiki/wiki_page.md)
  — the page's behaviour and design.
- [Community content domain](/apps/website/api_v2/src/community_content/README.md) — the wiki
  routes.
