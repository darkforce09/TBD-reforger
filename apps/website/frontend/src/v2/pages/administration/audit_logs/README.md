# Audit logs page

The `/admin/audit` page: administrators read the [audit logs](/documentation_v2/glossary.md#audit-logs),
the trail of administrative actions, newest first and a page at a time, filter the loaded entries
by text, and inspect one entry's actor, target and metadata. Nothing on the page writes or removes
an entry.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/audit_logs/
├── filter_bar.rs  the search field, and the text each entry is matched against
├── log_table.rs   the trail with its load control, the level tokens and the entry inspector
├── mod.rs         the module tree; re-exports `AuditLogsPage`
├── page.rs        `AuditLogsPage`: the gate, the first fetch and the keyset paging helpers
└── tests/         unit tests for the paging paths, the cursor read and the load-more append
```

## How it works

`AuditLogsPage` renders `AuditLogsInner` inside `AdminGate`. The inner component fetches the newest
page once and hands it to `board` in `log_table.rs`, which owns the trail from then on: the
accumulated entries, the next page's cursor, the load flags, the selected entry and the filter
text. The trail is keyset-paged: "Load more" asks for the entries older than the last id loaded
(`audit_logs_path`) and appends them (`merge_audit_page`), so the trail only grows and entries
written meanwhile never shift it. A `next_cursor` that is not a number ends the trail, and the
control disappears.

Entries are read as free-form JSON values, since an entry's fields differ by action; only the
severity is a fixed vocabulary, shown as `INFO`, `WARN` or `CRIT`, `----` when empty and
upper-cased otherwise (`level_label`). The filter runs in the browser over the loaded entries
only: `haystack` joins an entry's local stamp, level, action, actor name, message and target type,
`search_matches` compares them case-insensitively, and the text is never sent to the
[API](/documentation_v2/glossary.md#api). An entry carries every field the inspector shows, so
opening one fetches nothing, and a filter hides lines but never the open entry. Every request runs
in the browser build only; a native build renders the failure branch.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/admin/audit` | `AuditLogsPage` | route tier `admin`; the body renders inside `AdminGate`, for the `admin` role only | full-bleed inside the navigation frame; breadcrumb Administration / Audit Logs; sidebar entry "Audit Logs" |

## Data

- `GET /api/v1/admin/audit-logs`, then `GET /api/v1/admin/audit-logs?before=<id>` for each further
  page: read as `CursorList<Value>` (`data`, `next_cursor`); each entry reads `id`, `severity`,
  `created_at`, `action`, `message`, `actor_name`, `actor_id`, `target_type`, `target_id` and
  `metadata`. The page sends no `limit`, `severity` or `q`, and calls neither the CSV export nor
  the live feed.
- The page reads the `AuthStore` context and stores nothing in the browser.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| below `admin` | "Admin access required." |
| loading | "Loading…" |
| failed | "Failed to load data.", with no retry |
| empty trail | "No audit logs." |
| trail | the field "Filter by admin, action, or keyword..." above one line per entry: the local time as `YYYY-MM-DD HH:MM:SS` (`--------- --:--:--` when unreadable), the level token (`[INFO]`, `[WARN]`, `[CRIT]`, `[----]`), the action and the message |
| no match | "No entries match this filter." |
| more to load | "Load more" ("Loading…" while it runs), and "Could not load the next page." when it fails |
| nothing selected | "Select a log entry to inspect." |
| entry | the severity badge, the action, the message and the stamp; "Entry" (the id), "Actor" (the name with the id in brackets, or the id alone), "Target type" and "Target id", each only when present, and "Metadata" as formatted JSON when present |
| entry gone | "That entry is no longer in this page of the trail." |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `CursorList`), `crate::v2::core::auth`
  (`AuthStore`), `crate::v2::core::ui` (`AdminGate`, `SplitPane`, `SplitPaneEmpty`,
  `search_matches`, `badge_class`, `MaterialIcon`, `cn`) and `crate::v2::core::utils::datefmt`
  (`log_stamp`); over HTTP, the audit log route of the
  [administration](/documentation_v2/glossary.md#administration) domain.
- Used by: the `/admin/audit` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Audit Logs" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; `audit_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins the page's sources for its
  tests; the DOM oracle's `audit` capture in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: the first page asks for no `before` and each further page forwards the cursor
  (`first_page_path_has_no_before`, `continuation_path_forwards_cursor_as_before`); a further page
  appends and a null cursor ends the trail (`merge_appends_and_returns_cursor`,
  `empty_page_with_null_cursor_stops`); "Load more" appends through `merge_audit_page`
  (`on_load_more_appends_via_merge_audit_page`), all in `tests/audit.rs`; the page only reads.

## Related documentation

- [Audit logs page](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md)
  — the page's behaviour, what the trail holds server-side, its design, open work and decisions.
- [Administration domain](/apps/website/api_v2/src/administration/README.md) — the audit log
  routes, the export and the live feed.
