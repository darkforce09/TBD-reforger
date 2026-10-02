**Status:** live

# Administration and community content

Design for the [administration](/documentation/glossary/a_to_f.md#administration) requirements
(`administration_personnel_pagination`, `administration_audit_replay`,
`administration_audit_query_recovery`, `administration_audit_frontend`) and the
[community content](/documentation/glossary/a_to_f.md#community-content) requirements
(`content_vehicle_mutations`, `content_wiki_features`, `content_content_storage`). The same work
closes the tickets T-940.7 (personnel pagination), T-940.8 (vehicle mutations) and T-940.9 (wiki
markup and revisions). It records the chosen semantics before implementation; acceptance evidence
is the command output recorded in progress_checkpoint.md. The wire shapes are
`personnel-roster.schema.json`, `audit-log.schema.json`, `vehicle-database.schema.json`,
`wiki-page.schema.json` and `content-upload.schema.json` in `contracts/definitions/`.

Source paths below are relative to `apps/website/api_v2/src/`, test paths to `apps/website/api_v2/`.
Every error answers the envelope `{error, details?}`.

## Personnel pagination

`GET /api/v1/admin/users?q&page&per_page` takes the `AdminUser` extractor and serves the
[personnel](/documentation/glossary/n_to_z.md#personnel) page, which reads the roster a page at
a time.

- The answer is `{items, page, per_page, total}`; each item keeps the roster row fields the route
  already serves.
- `page` defaults to 1 and `per_page` to 20; a `per_page` above 100 clamps to 100.
- A `page` below 1, a `per_page` below 1 or a non-numeric value answers 400.
- Rows order by `lower(username) ASC, discord_id ASC`.
- A page past the end answers empty `items` with the real `total`.
- The query text keeps the pinned substring `AS warnings, total_deployments FROM users`.
- Contract: `personnel-roster.schema.json` (`PersonnelRow`, `PersonnelPage`).

## Audit replay and reset

`GET /api/v1/admin/audit-logs/stream` takes `AdminUser` and delivers the audit log as an
[SSE](/documentation/glossary/n_to_z.md#sse) stream. The list route
`GET /api/v1/admin/audit-logs` keeps its `{data, next_cursor}` page.

### Retention floor

Migration 0057 adds `audit_publication_state.retained_after_sequence bigint NOT NULL DEFAULT 0`.
Every delivery sequence at or below it may be missing.

- Backfill: the highest missing sequence at or below `last_sequence`, or 0 when none is missing.
- A statement-level `AFTER DELETE` trigger on `audit_publications` raises it to the highest
  deleted sequence.
- An `AFTER TRUNCATE` trigger sets it to `last_sequence`.

### Stream

`audit_delivery_stream` yields `AuditStreamItem::Ready`, `AuditStreamItem::Delivery` and
`AuditStreamItem::Reset`.

- Open: the stream sends `event: ready` with `id` = the start cursor and data
  `{resume_after, retained_after}`. A request without `Last-Event-ID` starts at the tail.
- A `Last-Event-ID` that does not parse answers 400.
- A cursor above the tail resets with reason `cursor_ahead`; a cursor below `retained_after`
  resets with reason `history_unavailable`.
- A reset is `event: reset` with `id` = the tail and data `{reason, resume_after, retained_after}`;
  the stream then continues from the tail.
- Every wake re-reads the floor; a floor above the live cursor resets.
- Rows: one unnamed SSE message per audit row, `id` = its delivery sequence, data = the
  `AuditLogEntry` JSON, the same shape as a list row.
- Contract: `audit-log.schema.json` (`AuditLogEntry`, `AuditLogPage`, `AuditStreamReady`,
  `AuditStreamReset`).

## Audit query recovery

- A publish failure is logged, and the read still runs.
- A read failure is logged, keeps the cursor, and retries on the next timer tick whether or not the
  `LISTEN` connection is healthy.

## Audit frontend protocol

The [audit logs](/documentation/glossary/a_to_f.md#audit-logs) page combines the stream with the
paged history:

1. Connect to the stream.
2. Wait for `ready`.
3. Load the history through the list route.
4. Merge live rows and history rows, deduplicated by audit id, so a row that arrives both ways
   shows once.
5. On `reset`, reload the history.

## Vehicles

| Route | Method | Caller |
|---|---|---|
| `/api/v1/vehicle-database` | GET | `AuthUser` |
| `/api/v1/vehicle-database` | POST | `AdminUser` |
| `/api/v1/vehicle-database/{id}` | GET | `AuthUser` |
| `/api/v1/vehicle-database/{id}` | PUT, PATCH, DELETE | `AdminUser` |

Migration 0058 adds the nullable columns `created_at`, `updated_at`, `created_by`, `updated_by`,
`deleted_at` and `deleted_by`. Rows that exist before it keep null in all six; `created_at` and
`updated_at` default to `now()` for new rows only.

### Validation

One validator serves POST, PUT and PATCH:

| Field | Rule |
|---|---|
| `name` | required, trimmed, at most 120 |
| `faction` | required, trimmed, at most 60 |
| `armor_type` | required, trimmed, at most 60 |
| `amphibious` | optional, at most 60 |
| `primary_threat` | optional, at most 120 |
| `profile_image_url` | optional; empty or a safe image URL |

- All three bodies reject unknown fields (`deny_unknown_fields`).
- PATCH: an absent field stays unchanged; `null` clears an optional field; `null` or blank on a
  required field answers 400.

### Answers

- POST answers 201 with the row; PUT, PATCH and DELETE answer 200 with the stored row.
- DELETE is soft: it sets `deleted_at` and `deleted_by`. A deleted row answers 404 on GET, PUT,
  PATCH and DELETE and leaves the list.
- The list orders by `name ASC, id ASC`.
- Every mutation appends a transactional audit row (administration `required_audit`) in the same
  transaction.
- The response row shape stays as served: empty optional strings are omitted.
- Contract: `vehicle-database.schema.json`.

## Wiki markup and revisions

- `GET /api/v1/wiki`: the page summaries.
- `GET /api/v1/wiki/{slug}` (`AuthUser`): one article; `PUT /api/v1/wiki/{slug}` (`AdminUser`):
  create or save it.
- `GET /api/v1/wiki/{slug}/revisions` (`AuthUser`): `{items, page, per_page, total}`, newest
  first.
- `GET /api/v1/wiki/{slug}/revisions/{revision}`: one revision.

### Revision storage

Migration 0059:

- `wiki_pages.revision integer NOT NULL DEFAULT 1`.
- Table `wiki_page_revisions`: `page_id uuid` referencing `wiki_pages` `ON DELETE CASCADE`,
  `revision integer` above 0, `slug`, `category`, `title`, `icon`, `nav_order`, `body_md`,
  `author_id text`, `created_at timestamptz DEFAULT now()`; primary key `(page_id, revision)`.
- Backfill: revision 1 of every page, with `author_id` = `updated_by` and `created_at` =
  `updated_at`.

### Shapes

- Summary: `{slug, category, title, icon, nav_order, revision, updated_at}`, ordered by
  `nav_order, title, slug`.
- Article: the summary plus `id`, `body_md`, `updated_by` and `blocks`.
- Contract: `wiki-page.schema.json`.

### Saving

- The PUT body is `{category, title, icon, nav_order, body_md, base_revision}`
  (`deny_unknown_fields`).
- `base_revision: null` creates the page; when the page exists it answers 409.
- A numeric `base_revision` must equal the current revision; otherwise the save answers 409 with
  `details.code = wiki_revision_conflict` and `details.current_revision`.
- The slug matches `[a-z0-9-]{1,64}`.
- `body_md` holds at most 262 144 bytes; a larger body answers 400 with
  `details.code = wiki_body_too_large`.
- An accepted save updates the page, inserts the revision row and appends the audit row in one
  transaction, and answers the article with 200 for an update or 201 for a create.

### Markup service

`community_content/services/wiki_markup/` parses `body_md` with pulldown-cmark 0.13.4
(`default-features = false`) with tables, task lists, strikethrough and GFM blockquote callouts
enabled, plus the bracket callout form `> [!CRITICAL|CAUTION|WARNING|TIP|NOTE|INFO]`. It builds a
typed AST, serialized with a serde `type` tag in snake_case.

| Block | Fields |
|---|---|
| `heading` | `level` 1 to 6, `anchor`, `inlines` |
| `paragraph` | `inlines` |
| `list` | `ordered`, optional `start`, `items`, each `{checked?, blocks}` |
| `table` | `alignments` (each none, left, center or right), `header` (cells), `rows` (rows of cells); a cell is inlines |
| `callout` | `kind`, `blocks` |
| `quote` | `blocks` |
| `code` | optional `language`, `text` |
| `rule` | none |

| Inline | Fields |
|---|---|
| `text` | `text` |
| `strong`, `emphasis`, `strikethrough` | `children` |
| `code` | `text` |
| `link` | `href`, `external`, `children` |
| `image` | `src`, `alt`, optional `title` |
| `line_break` | none |

Anchors are the slugified heading text; a repeated anchor takes `-2`, `-3` and onward.

Rendering-time safety, applied whenever the service builds `blocks`:

- an unsafe link becomes its children as plain inlines;
- an unsafe image becomes `text{alt}`;
- raw HTML becomes literal `text`;
- nesting deeper than 16 is flattened.

Save-time refusal: an unsafe link or image URL, raw HTML or nesting deeper than 16 answers 422 with
`details.code = wiki_markup_refused` and `details.findings`, each `{line, code, detail}`.

### URL policy

`core/text/content_url_policy.rs` decides which URLs are safe:

- Images: `https://host…`, or a site-relative `/path` that does not start with `//`.
- Links: the image forms, plus `http://`, `mailto:` and `#fragment`.
- Everything else is refused: `javascript:`, `data:`, `vbscript:`, `file:`, protocol-relative
  URLs, control characters and whitespace tricks.
- External means an absolute `http(s)` or `mailto` URL.

## Content storage and request limits

- `ApiError::from_json_rejection` (`core/error_handling`) maps a JSON body rejection: a 413 answers
  413 with `details.code = request_too_large`, a missing or invalid content type answers 415, and
  every other rejection answers 400 with the rejection text. The vehicle, wiki and announcement
  handlers use it.
- Announcement public and CMS pages order with an `id DESC` tiebreak.
- Ownership: author and editor ids come from the authenticated caller, never from the body. A
  caller who is not an administrator gets 403, an anonymous caller 401.

### Uploads

`POST /api/v1/cms/uploads` takes `AdminUser`:

| Case | Answer |
|---|---|
| multipart body over the limit | 413, never 400 |
| no file field | 400 |
| extension other than `jpg`, `jpeg`, `png`, `webp` | 415 |
| magic bytes that do not match the extension | 415 |
| storage failure | 503 with `details.code = storage_unavailable`; the io error is logged |
| stored | 201 `{url}` |

The file is written through `tokio::fs` to a temporary name and then renamed, so no partial file is
ever served. Contract: `content-upload.schema.json`.

## Lock order

Each mutation runs in one transaction:

- Vehicle mutations: the vehicle row `FOR UPDATE`, then the audit row.
- Wiki saves: the wiki page row `FOR UPDATE`, then the revision insert, then the audit row.

## Path map

The frozen plans (`documentation/tickets/plans/t-940_7_plan.md`,
`documentation/tickets/plans/t-940_8_plan.md`, `documentation/tickets/plans/t-940_9_plan.md`)
name files of an older layout. Their work lands at these paths:

| Ticket | Plan names | Current path |
|---|---|---|
| T-940.7 | `admin.rs` `list_users` | `apps/website/api_v2/src/administration/handlers/personnel_roster.rs` |
| T-940.7 | `pages/admin/personnel.rs` | `apps/website/frontend/src/v2/pages/administration/personnel/` |
| T-940.8 | `handlers/content/vehicles.rs`; the vehicle handlers in `content/wiki.rs` | `community_content/handlers/vehicle_database/` |
| T-940.8 | `app.rs` routes | `community_content/routes.rs` |
| T-940.8 | `core/dto.rs` (the R-api golden mirror) | `apps/website/frontend/src/v2/core/api/dto/` |
| T-940.8 | the vehicle page | `apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/` |
| T-940.9 | `content/wiki.rs` | `community_content/handlers/wiki_knowledgebase/` |
| T-940.9 | `services/wiki_markup.rs` | `community_content/services/wiki_markup/` |
| T-940.9 | `migrations/0026_wiki_revisions.sql` | migrations 0057 (audit retention floor), 0058 (vehicle audit columns) and 0059 (wiki revisions) |
| T-940.9 | `pages/public/wiki.rs` | `apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/` |

## Tests

Each requirement's check runs `cargo xtask db test-it` and counts the passing cases its prefix
names.

| Requirement | Suite | Cases |
|---|---|---|
| `administration_personnel_pagination` | `tests/personnel_pagination.rs` | `personnel_pagination_*` |
| `administration_audit_replay` | `tests/audit_replay.rs` | `audit_replay_*` |
| `administration_audit_query_recovery` | `tests/audit_query_recovery.rs` | `audit_query_recovery_*` |
| `administration_audit_frontend` | `tests/audit_frontend.rs` | `audit_frontend_*` |
| `content_vehicle_mutations` | `tests/vehicle_mutations.rs` | `vehicle_mutations_*` |
| `content_wiki_features` | `tests/wiki_features.rs` | `wiki_features_*` |
| `content_content_storage` | `tests/content_storage.rs` | `content_storage_*` |

`administration_transactional_audit` and `administration_audit_publication` keep their cases,
`transactional_audit*` and `audit_publication*`, in `tests/audit_publication.rs`.
