# Announcements page

The `/announcements` and `/announcements/:id` pages in the
[command center](/documentation_v2/glossary/a_to_f.md#command-center): the unit's published announcements
in a master list, with the one the address names open in a reading pane beside it.

## Contents

```text
apps/website/frontend/src/v2/pages/command_center/announcements/
├── article_feed.rs    the master list, its order and tag badges, and the split pane with the reader
├── article_viewer.rs  the reading pane: chips, headline, byline, thumbnail and body paragraphs
├── mod.rs             the module tree; re-exports `AnnouncementsPage`
├── page.rs            the route component: the announcement list fetch
└── tests/             unit tests for the body paragraphs, the preview line and the thumbnail sink
```

## How it works

`AnnouncementsPage` renders inside `AuthGate`. The signed-in half fetches the list once; the list
payload carries every body, so opening an announcement never fetches again and the reading pane
never shows a stale one. `article_feed.rs` parks the rows in a stored value that both halves of
the `SplitPane` read, sorts pinned rows first and keeps the server's order within each group, and
draws each row with its tag badge (the tag in capitals with underscores as spaces, `NOTICE` when
there is none), headline, date and preview (the `snippet`, else the first paragraph of the
`body`). The filter icon in the "Comms Link" header does nothing: there are no category filters.

The address bar decides which announcement is open: the selection is a memo over `:id`, and a row
click navigates to `/announcements/{id}` instead of setting a signal, so a bare `/announcements`
opens nothing and a deep link opens exactly one. The reading pane shows the tag and pin chips, a
"Pushed to Discord" chip, the headline, the author or "Command", the local publish time, the
thumbnail when its address is an `http(s)` URL, and the body. The body is stored as plain text and
unsanitised, so it renders as one text node per blank-line separated paragraph, escaped once; inline
markup shows as written, and rendering it as markup would need a sanitiser on the write path first.
An optional field the [API](/documentation_v2/glossary/a_to_f.md#api) leaves out is not drawn.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/announcements` | `AnnouncementsPage` | route tier `none`; the data renders only for a signed-in viewer | full-bleed inside the navigation frame; breadcrumb Command Center / Announcements |
| `/announcements/:id` | `AnnouncementsPage` | as above | as above; `:id` opens that announcement |

## Data

- `GET /api/v1/announcements`: the list, read as `Paginated<serde_json::Value>`; the rows carry
  `id`, `is_pinned`, `tag`, `title`, `published_at`, `snippet`, `body`, `author_id`,
  `thumbnail_url` and `pushed_to_discord`.
- The page reads the session from the `AuthStore` context and the `:id` parameter from the router,
  and writes nothing. The fetch runs in the browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load data." |
| no announcements | "No announcements yet." beside "Select a broadcast to read." |
| nothing opened | the list beside "Select a broadcast to read." |
| unknown id | "That broadcast is no longer in the feed." |
| opened | the announcement, titled "Untitled Post" when it has no title |

## Boundaries

- Depends on: `crate::v2::core::api` (the `api_get` client, `Paginated`), `crate::v2::core::ui`
  (`AuthGate`, `SplitPane`, `SplitPaneEmpty`, `ListDetailItem`, `MaterialIcon`, `badge_class`),
  `crate::v2::core::auth::url_guard`, `crate::v2::core::utils` (short and local date formatting),
  the `AuthStore` context and the router's `use_params_map` and `use_navigate`.
- Used by: the `/announcements` and `/announcements/:id` routes in
  `apps/website/frontend/src/app_routes.rs`; the dashboard's Recent Intelligence rows in
  `apps/website/frontend/src/v2/pages/command_center/dashboard/` link to `/announcements/{id}`.
- Rules: a body paragraph keeps bare `<` and `&` for the one escape the view applies
  (`body_paragraphs_preserve_bare_angle_brackets` in `tests/announcements.rs`); the preview falls
  back to the body's first paragraph
  (`preview_prefers_snippet_but_falls_back_to_body_without_entities`); a thumbnail `src` is only
  ever an `http(s)` URL (`announcement_thumbnail_emits_src_only_for_http_urls`).

## Related documentation

- [Announcements page](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md)
  — the page's behaviour and design.
