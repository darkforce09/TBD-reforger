**Status:** live

# Audit logs page

The `/admin/audit` page, one of the [administration](/documentation/glossary/a_to_f.md#administration)
pages: administrators read the platform's trail of administrative actions, newest first, with new
entries arriving live over the [SSE](/documentation/glossary/n_to_z.md#sse) stream and older ones a
page at a time, and inspect one entry's actor, target and metadata. The screen only reads.

## Where it lives

- Code: [`crates/frontend/pages/administration_pages/src/audit_logs/`](/crates/frontend/pages/administration_pages/src/audit_logs/):
  `page.rs` holds the route component `AuditLogsPage`, the stream wiring, the history reloads and
  the list paths; `live_merge.rs` the board that merges history pages and live rows by audit id;
  `live_status.rs` the status badge and the history load state; `filter_bar.rs` the search field
  and the text an entry is matched against; `log_table.rs` the trail, its load control and the
  entry inspector. The folder's
  [README](/crates/frontend/pages/administration_pages/src/audit_logs/README.md) describes
  each file. The stream client is
  [`core/api/audit_stream.rs`](/crates/frontend/foundation/frontend_transport/src/audit_stream.rs), which reads
  the bytes with the event-stream parser
  [`core/api/sse_frames.rs`](/crates/frontend/foundation/frontend_transport/src/sse_frames.rs).
- Entry: the route, its tier and its layout are in the README's
  [Routes](/crates/frontend/pages/administration_pages/src/audit_logs/README.md#routes).
- Related: the [API](/documentation/glossary/a_to_f.md#api)'s
  [administration domain](/crates/api/api_administration/src/README.md), which serves the
  trail; every page that changes state writes to it.

## Behaviour

1. The page body sits in `AdminGate` (`crates/frontend/foundation/frontend_session/src/gates.rs`), which
   shows the session and access states of the README's
   [States](/crates/frontend/pages/administration_pages/src/audit_logs/README.md#states) in
   place of the page until a signed-in viewer holds the `admin`
   [role](/documentation/glossary/n_to_z.md#role). The route redirects no one.
2. The page connects to the live audit stream first and loads the newest page of the trail only
   after the stream's `ready`, so no entry committed in between is missed. When the first
   connection fails before any `ready`, the trail loads anyway, and a later `ready` on a fresh
   connection reloads it. A failed history load offers no retry, so the viewer reloads the page.
3. New entries join the trail as the stream delivers them. The trail holds each entry once, by its
   id, so an entry that arrives live and again in a history page shows once, and the trail stays
   in id order, newest first.
4. After a drop the stream reconnects with the last event id it received, waiting 1 s, then
   longer, up to 30 s, and the server replays what was missed. When the server cannot replay
   (`event: reset`), the page empties the trail and reloads the newest page.
5. A status badge next to the search field says `Connecting`, `Live`, `Reconnecting`, `Reloading`
   (a reload after a reset is in flight) or `Offline` (the session ended or the admin role is
   gone, with the reason under it), beside the count of entries the stream delivered, such as
   "3 live rows".
6. The master column lists one line per entry, in the format the README's
   [States](/crates/frontend/pages/administration_pages/src/audit_logs/README.md#states) give:
   the local time, the level token, the action and the message, with a blinking cursor after the
   last line.
7. The search field above the trail filters in the browser only: an entry matches when its local
   stamp, level, action, actor name, message or target type contains the text. Entries not loaded
   yet are never searched, and the text is never sent to the API. A filter that hides every
   loaded entry says so, which reads differently from an empty trail.
8. While the API reports a further page, a "Load more" control follows the trail. It fetches the
   entries below the smallest id a history page delivered and merges them, so the trail only grows;
   an entry that arrived live never moves where the next page starts.
9. Selecting a line opens it in the detail column: the severity, the action, the message and the
   stamp, then the entry's id, its actor, its target type and target id, each shown only when the
   entry carries it, and its metadata, printed as formatted JSON. A filter hides lines but never
   the open entry.

## Data

The page reads the stream and the list, which the README's
[Data](/crates/frontend/pages/administration_pages/src/audit_logs/README.md#data) lists with
the DTOs it reads. Server-side:

- `GET /api/v1/admin/audit-logs/stream` (`stream_audit_logs` in
  `crates/api/api_administration/src/handlers/audit_logs.rs`): an SSE stream that opens with
  `event: ready` (its id is the start cursor), sends every published entry as an unnamed event
  whose id is its publication sequence, and sends `event: reset` when the `Last-Event-ID` it was
  given cannot be replayed, then continues from the tail. It is woken by Postgres notifications,
  falling back to a 2-second poll, and ends with `event: authorization_expired` when the session
  or the role lapses; the page then refreshes the session and reconnects. The design note's
  [audit replay and reset](/documentation/crates/api/api_server/design_notes/administration_and_content.md#audit-replay-and-reset)
  gives the protocol.

- `GET /api/v1/admin/audit-logs`, then `?before=<id>` for each further page (`list_audit_logs` in
  `crates/api/api_administration/src/handlers/audit_logs.rs`): the API returns the entries
  newest first (`ORDER BY id DESC`), keyset-paged: `before` keeps the ids below it, a page holds
  20 entries unless `limit` asks for up to 100, and `next_cursor` carries the last id whenever the
  page came back full. Each entry is an `AuditLog` row
  (`crates/api/api_administration/src/models/audit_log.rs`): `id`, `severity` (`info`,
  `warn` or `crit`), `actor_id` (absent when the entry records no account), `actor_name`, `action`,
  `message`, `target_type`, `target_id`, free-form JSON `metadata` and `created_at`.
- The same handler filters by `?severity=` and by `?q=`, a case-insensitive match on the message;
  the page sends neither.
- The API serves one more route the page does not call: `GET /api/v1/admin/audit-logs/export.csv`
  (`export_audit_logs_csv`), the newest 10 000 entries as the attachment `audit-logs.csv` with
  every cell escaped against spreadsheet formulas.
- Most entries are appended inside the transaction of the change they record, through
  `append_actor_audit` and its siblings in
  `crates/api/api_audit_log/src/required_audit.rs`, so the entry and its change
  commit or fail together; triggers from `crates/api/api_database/migrations/0025_audit_notify.sql`
  write the entries for creating an [event](/documentation/glossary/a_to_f.md#event), soft-deleting a
  [mission](/documentation/glossary/g_to_m.md#mission) and removing a member from a
  [slot](/documentation/glossary/n_to_z.md#slot) in the statement that makes the change. The role
  resync, warnings, modpack and announcement administration, the server
  [registry](/documentation/glossary/n_to_z.md#registry) and mission versions write best-effort instead,
  through `write_audit` in `crates/api/api_audit_log/src/audit_writer.rs`, after
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
  [design tokens](/documentation/design_system/design_tokens.md).
- Design target: the [audit logs blueprint](/documentation/crates/frontend/pages/administration_pages/audit_logs/visual_references/audit_logs_blueprint/README.md),
  a design-phase reference.
  The built page differs from the blueprint:
  - no page heading or subtitle: the breadcrumb names the page;
  - no "Export to CSV" button, although the API serves the export (see Open work);
  - a status badge with five states and a live row count in place of the "Live Feed" badge;
  - no terminal window chrome (the traffic-light dots and the `~/syslog_view` title bar);
  - each line carries the action as well as the message;
  - the inspector shows the entry's fields and its metadata, and has no stack-trace panel.

## Open work

- Audit SSE stream has no client (ticket `audit-sse-stream-has` in `ttm`): the page now consumes the
  stream; the ticket record still has to be closed.
- Add CSV export to the audit logs page (ticket `add-csv-export-audit` in `ttm`): the page gains a
  control that downloads `GET /api/v1/admin/audit-logs/export.csv`, which the API already serves.

## Decisions

- The trail is keyset-paged by the last id seen, not by offset: entries written while an
  administrator reads cannot shift the window and hide a line.
- "Load more" appends: replacing the trail with the next page would drop everything already read.
- The filter runs over what is loaded: fetching per keystroke would send a request per character
  and risk showing a stale answer. The cost is that it cannot find an entry not loaded yet.
- Opening an entry never fetches: each loaded entry carries every field the inspector shows, and
  its free-form metadata is printed whole, as formatted JSON, since its keys differ by action.
- The stream connects before the history loads, and the history loads after `ready`: loading
  first would leave a gap for entries committed between the list answer and the stream's start.
- The trail is keyed by audit id and merges both sources, because an entry committed around
  `ready` arrives both ways; showing it twice would read as two actions.
- "Load more" continues below the smallest history id, not the smallest id on the trail: a live
  entry's id is its allocation order, which can sit far below the loaded history and would skip
  the entries between.
- A reset empties the trail before reloading, and a history page requested before the reset is
  dropped when it lands, so the trail never mixes the history it replaced with the new one.
- The history loads without the stream when the first connection fails, so the trail never hangs
  on a stream that is down.
