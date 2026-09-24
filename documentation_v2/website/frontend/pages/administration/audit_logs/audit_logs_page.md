**Status:** live

# Audit logs page

The `/admin/audit` page, one of the [administration](/documentation_v2/glossary.md#administration)
pages: administrators read the platform's trail of administrative actions, newest first and a page
at a time, and inspect one entry's actor, target and metadata. The screen only reads.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/administration/audit_logs/`](/apps/website/frontend/src/v2/pages/administration/audit_logs/):
  `page.rs` holds the route component `AuditLogsPage`, the first fetch and the keyset paging
  helpers; `filter_bar.rs` the search field and the text an entry is matched against;
  `log_table.rs` the trail, its load control and the entry inspector. The folder's
  [README](/apps/website/frontend/src/v2/pages/administration/audit_logs/README.md) describes
  each file.
- Entry: the `/admin/audit` route renders `AuditLogsPage`
  (`apps/website/frontend/src/app_routes.rs`); `apps/website/frontend/src/router.rs` declares it
  for the `admin` tier, full-bleed, with the breadcrumb "Administration" › "Audit Logs", and the
  sidebar's Administration section lists it as "Audit Logs"
  (`apps/website/frontend/src/v2/pages/navigation/nav_config.rs`).
- Related: the [API](/documentation_v2/glossary.md#api)'s
  [administration domain](/apps/website/api_v2/src/administration/README.md), which serves the
  trail; every page that changes state writes to it.

## Behaviour

1. The page body sits in `AdminGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`): "Loading
   session…" while the session restores; "Sign in to load live data from the platform." with a
   "Sign in with Discord" link to `/login` for a signed-out viewer; "Admin access required." for
   a signed-in viewer below the `admin` [role](/documentation_v2/glossary.md#role). The route
   redirects no one.
2. The page fetches the newest page of the trail, showing "Loading…" meanwhile and "Failed to load
   data." when the fetch fails; the failure offers no retry, so the viewer reloads the page.
3. The master column lists one line per entry: the local time as `[YYYY-MM-DD HH:MM:SS]`, the
   level token (`[INFO]`, `[WARN]`, `[CRIT]`; `[----]` when the severity is empty, and an unknown
   severity upper-cased), the action and the message, with a blinking cursor after the last line.
   An empty trail reads "No audit logs.".
4. The search field above the trail, "Filter by admin, action, or keyword...", filters in the
   browser only: an entry matches when its local stamp, level, action, actor name, message or
   target type contains the text. Entries not loaded yet are never searched, and the text is
   never sent to the API. When the filter hides every loaded entry the column reads "No entries
   match this filter.".
5. While the API reports a further page, a "Load more" control follows the trail. It fetches the
   entries older than the last one loaded and appends them, so the trail only grows; it reads
   "Loading…" while the request runs and "Could not load the next page." when it fails.
6. Selecting a line opens it in the detail column: the severity badge, the action, the message and
   the stamp, then the fields Entry (the id), Actor (the name with the id in brackets), Target
   type and Target id, each shown only when the entry carries it, and Metadata, printed as
   formatted JSON. Before any selection the column reads "Select a log entry to inspect."; a
   selection missing from the loaded trail reads "That entry is no longer in this page of the
   trail.". A filter hides lines but never the open entry.

## Data

The page makes one kind of call. Server-side:

- `GET /api/v1/admin/audit-logs`, then `?before=<id>` for each further page (`list_audit_logs` in
  `apps/website/api_v2/src/administration/handlers/audit_logs.rs`): the page reads it as
  `CursorList<Value>` (`data`, `next_cursor`). The API returns the entries newest first
  (`ORDER BY id DESC`), keyset-paged: `before` keeps the ids below it, a page holds 20 entries
  unless `limit` asks for up to 100, and `next_cursor` carries the last id whenever the page came
  back full. Each entry is an `AuditLog` row
  (`apps/website/api_v2/src/administration/models/audit_log.rs`): `id`, `severity` (`info`,
  `warn` or `crit`), `actor_id` (absent when the entry records no account), `actor_name`, `action`,
  `message`, `target_type`, `target_id`, free-form JSON `metadata` and `created_at`.
- The same handler filters by `?severity=` and by `?q=`, a case-insensitive match on the message;
  the page sends neither.
- The API serves two more routes the page does not call: `GET /api/v1/admin/audit-logs/export.csv`
  (`export_audit_logs_csv`), the newest 10 000 entries as the attachment `audit-logs.csv` with
  every cell escaped against spreadsheet formulas, and `GET /api/v1/admin/audit-logs/stream`
  (`stream_audit_logs`), an [SSE](/documentation_v2/glossary.md#sse) feed that replays from the
  `Last-Event-ID` it is given and is woken by Postgres notifications, falling back to a 2-second
  poll.
- Most entries are appended inside the transaction of the change they record, through
  `append_actor_audit` and its siblings in
  `apps/website/api_v2/src/administration/services/required_audit.rs`, so the entry and its change
  commit or fail together; triggers from `apps/website/api_v2/migrations/0025_audit_notify.sql`
  write the entries for creating an [event](/documentation_v2/glossary.md#event), soft-deleting a
  [mission](/documentation_v2/glossary.md#mission) and removing a member from a
  [slot](/documentation_v2/glossary.md#slot) in the statement that makes the change. The role
  resync, warnings, modpack and announcement administration, the server
  [registry](/documentation_v2/glossary.md#registry) and mission versions write best-effort instead,
  through `write_audit` in `apps/website/api_v2/src/administration/services/audit_writer.rs`, after
  their change has committed, as do the system warnings for a failed Discord push, a failed
  statistics or leaderboard refresh and a low server FPS: a failed write is only logged, and the
  change stands without its entry. The other administration pages' docs name the actions they
  record, such as `user.ban`, `mission.approve` and `event.deleted`.

## Design

- A full-bleed `SplitPane`: the trail takes 60% of the width in a monospace column under the
  search field, and the inspector fills the rest; the empty inspector shows the `terminal` icon.
- Level tokens are coloured by severity: `CRIT` bold in the alert colour, `WARN` in the tactical
  yellow, everything else in the primary colour, and the inspector repeats the severity as a
  badge. Colours and type come from the platform's
  [design tokens](/documentation_v2/design_system/design_tokens.md).
- Design target: the [audit logs blueprint](/documentation_v2/website/frontend/pages/administration/audit_logs/visual_references/audit_logs_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [Audit Logs section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#12-audit-logs).
  The built page differs from the blueprint:
  - no page heading or subtitle: the breadcrumb names the page;
  - no "Export to CSV" button, although the API serves the export (see Open work);
  - no "Live Feed" badge and no live updates: the trail changes only through "Load more";
  - no terminal window chrome (the traffic-light dots and the `~/syslog_view` title bar);
  - each line carries the action as well as the message;
  - the inspector shows the entry's fields and its metadata, and has no stack-trace panel.

## Open work

- [T-950 — Audit SSE stream has no client](/.ai/tickets/T-950.toml) (idea, no plan): either the page
  consumes the audit stream, so new entries appear without a reload, or the stream is recorded as
  an API-only route.
- [T-944 — Audit stream: id-order race and half-open socket](/documentation_v2/tickets/specs/t944_audit_stream_race.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-944_plan.md)): the stream delivers every
  committed entry exactly once, even when a lower id commits late or the database connection
  silently drops; it matters to this page once the page consumes the stream.
- [T-1029 — Add CSV export to the audit logs page](/.ai/tickets/T-1029.toml) (idea, no plan): the
  page gains a control that downloads `GET /api/v1/admin/audit-logs/export.csv`, which the API
  already serves.

## Decisions

- The trail is keyset-paged by the last id seen, not by offset: entries written while an
  administrator reads cannot shift the window and hide a line.
- "Load more" appends: replacing the trail with the next page would drop everything already read.
- The filter runs over what is loaded: fetching per keystroke would send a request per character
  and risk showing a stale answer. The cost is that it cannot find an entry not loaded yet.
- Opening an entry never fetches: each loaded entry carries every field the inspector shows, and
  its free-form metadata is printed whole, as formatted JSON, since its keys differ by action.
