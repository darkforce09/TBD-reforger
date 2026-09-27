# Audit logs page

The `/admin/audit` page: administrators read the [audit logs](/documentation_v2/glossary/a_to_f.md#audit-logs),
the trail of administrative actions, newest first, with new entries arriving live over the
[SSE](/documentation_v2/glossary/n_to_z.md#sse) stream and older ones a page at a time; they filter
the loaded entries by text and inspect one entry's actor, target and metadata. Nothing on the page
writes or removes an entry.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/audit_logs/
├── filter_bar.rs   the search field, and the text each entry is matched against
├── live_merge.rs   `AuditBoard`: history pages and live rows merged by audit id, newest first
├── live_status.rs  the history load state, the status badge, the live row count, the history fallback
├── log_table.rs    the trail with its load control, the level tokens and the entry inspector
├── mod.rs          the module tree; re-exports `AuditLogsPage`
├── page.rs         `AuditLogsPage`: the gate, the stream wiring, the history reloads, the list paths
└── tests/          unit tests for the paths, the board, the badge, the goldens and the stream teardown
```

## How it works

`AuditLogsPage` renders `AuditLogsInner` inside `AdminGate`. The inner component keeps four
signals: the board (`AuditBoard`), the stream state, the history state and the latest
rejected-event note. It opens the live stream first (`connect_live_feed`, through
`core::api::audit_stream::open_audit_stream`), keeps the returned handle in a local
`StoredValue` and aborts it in `on_cleanup`, so the connection dies with the page.

The history waits for the stream. A `ready` on a connection opened without a cursor, and every
`reset`, call `reload_history`: it empties the board (`AuditBoard::restart`, which moves the board
to a new epoch), marks the history `Loading` the first time and `Reloading` after that, and fetches
the newest page. A page that lands after a later restart carries an old epoch and is dropped. When
the first connection drops or stops before any `ready` (`history_fallback_due`), the history loads
at once, so the trail never hangs on a stream that is down; a later fresh `ready` reloads it.

Live rows go straight into the board (`AuditBoard::insert_live`). The board is keyed by audit id,
so a line that arrives live and again in a history page, or twice either way, shows once, and the
rows read newest id first, so a line published late with an old id slots in at its id. "Load more"
asks for the entries below the smallest id a **history page** delivered (`AuditBoard::continuation`,
through `audit_logs_path`) and merges them under the epoch it was requested in; a live line's id
never moves that keyset. A `next_cursor` that is not a number, or an empty page, ends the history,
and the control disappears.

The master header holds the filter field and the status badge (`live_status`): `Connecting` before
the first answer, `Live` once `ready` arrived, `Reconnecting` while the stream waits out its
backoff, `Reloading` while a reload after a reset or a fresh `ready` is in flight, and `Offline`
once the stream stopped for good, with the reason under it; beside it, the count of lines the
stream delivered first ("3 live rows"). A live event that does not decode is logged and its note
shown under the badge.

Entries are typed `AuditLogEntry` rows; the severity is one of `info`, `warn` and `crit`, shown as
`INFO`, `WARN` or `CRIT` (`level_label`). The filter runs in the browser over the loaded entries
only: `haystack` joins an entry's local stamp, level, action, actor name, message and target type,
`search_matches` compares them case-insensitively, and the text is never sent to the
[API](/documentation_v2/glossary/a_to_f.md#api). An entry carries every field the inspector shows, so
opening one fetches nothing, and a filter hides lines but never the open entry. The stream and
every request run in the browser build only; a native build renders the waiting state.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/admin/audit` | `AuditLogsPage` | route tier `admin`; the body renders inside `AdminGate`, for the `admin` [role](/documentation_v2/glossary/n_to_z.md#role) only | full-bleed inside the navigation frame; breadcrumb Administration / Audit Logs; sidebar entry "Audit Logs" |

## Data

- `GET /api/v1/admin/audit-logs/stream`, opened first and reopened with `Last-Event-ID` after every
  drop: `event: ready` (`AuditStreamReady`), unnamed audit lines (`AuditLogEntry`) and
  `event: reset` (`AuditStreamReset`), decoded by `core::api::audit_stream`.
- `GET /api/v1/admin/audit-logs` after the stream's `ready` or `reset`, then
  `GET /api/v1/admin/audit-logs?before=<id>` for each further page: read as
  `CursorList<AuditLogEntry>` (`data`, `next_cursor`). The page sends no `limit`, `severity` or
  `q`, and does not call the CSV export.
- `GET /api/v1/me`, through the client, when the stream asks for a session revalidation.
- The page reads the `AuthStore` context and stores nothing in the browser.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| below `admin` | "Admin access required." |
| waiting, loading or reloading, board empty | "Loading…" |
| history failed | "Failed to load data.", alone when the board is empty and under the rows otherwise, with no retry |
| empty trail | "No audit logs." |
| trail | the field "Filter by admin, action, or keyword..." above one line per entry: the local time as `YYYY-MM-DD HH:MM:SS` (`--------- --:--:--` when unreadable), the level token (`[INFO]`, `[WARN]`, `[CRIT]`), the action and the message |
| status badge | `Connecting`, `Live`, `Reconnecting`, `Reloading` or `Offline`, beside "N live rows" |
| offline | the badge `Offline` and "The session ended; sign in again for live updates." or "Live updates need the admin role." |
| unreadable live event | "A live `<event>` event could not be read: …" under the badge |
| no match | "No entries match this filter." |
| more to load | "Load more" ("Loading…" while it runs), and "Could not load the next page." when it fails |
| nothing selected | "Select a log entry to inspect." |
| entry | the severity badge, the action, the message and the stamp; "Entry" (the id), "Actor" (the name with the id in brackets, or the id alone), "Target type" and "Target id", each only when present, and "Metadata" as formatted JSON when present |
| entry gone | "That entry is no longer in this page of the trail." (a reload emptied the board) |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `CursorList`, `dto::administration`,
  `audit_stream`), `crate::v2::core::auth` (`AuthStore`), `crate::v2::core::ui` (`AdminGate`,
  `SplitPane`, `SplitPaneEmpty`, `search_matches`, `badge_class`, `MaterialIcon`, `cn`) and
  `crate::v2::core::utils::datefmt` (`log_stamp`); over HTTP, the audit log routes of the
  [administration](/documentation_v2/glossary/a_to_f.md#administration) domain.
- Used by: the `/admin/audit` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Audit Logs" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; `audit_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins the page's sources for its
  tests; the DOM oracle's `audit` capture in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`, which answers the stream
  from `apps/website/frontend/tests/fixtures/api/GET__admin__audit-logs__stream.sse.txt`.
- Rules, all in `tests/`: the paths and the continuation (`first_page_path_has_no_before`,
  `continuation_path_forwards_cursor_as_before`, `merge_appends_and_returns_cursor`,
  `empty_page_with_null_cursor_stops`); "Load more" merges into the board under its epoch
  (`on_load_more_merges_into_the_board`); the stream is aborted on cleanup and the history
  loads only from the stream's callbacks (`the_route_aborts_its_stream_on_cleanup`,
  `the_route_loads_history_only_from_the_stream_callbacks`); dedupe, overlap, ordering, the
  keyset floor and restarts (`audit_board_*` in `tests/live_merge.rs`); the badge and the
  fallback (`audit_status_*` in `tests/live_status.rs`); the page only reads.

## Related documentation

- [Audit logs page](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md)
  — the page's behaviour, what the trail holds server-side, its design, open work and decisions.
- [Administration domain](/apps/website/api_v2/src/administration/README.md) — the audit log
  routes, the export and the live feed.
