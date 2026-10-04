# Community content handlers

The HTTP handlers of what members read and administrators author: announcements and their Discord
push, the doctrine wiki, the vehicle database, modpack manifests and CMS image uploads. Members read
with `AuthUser`; every write takes `AdminUser`. The equipment data viewer's debug reads are
anonymous and exist only in development.

## Contents

```text
crates/api/api_community_content/src/handlers/
├── announcement_discord_push.rs  pushes a published announcement to Discord and records the result
├── announcements_admin.rs        the CMS announcement list, create, partial edit and archive
├── announcements_public.rs       the member feed: published announcements, and one of them
├── equipment_data_viewer/        development-only debug reads of the exported equipment datasets
├── media_upload/                 the CMS image upload: format checks and the upload directory writer
├── mod.rs                        the module tree
├── modpack_admin.rs              modpack create, full replace, set-current and delete
├── modpack_catalog.rs            every modpack with its mods, and the current one
├── tests/                        unit tests for the Discord push and the CMS announcement writers
├── vehicle_database/             the vehicle database: member reads, admin writes, their validator
└── wiki_knowledgebase/           the wiki: summaries, the article, the admin save and the revision history
```

## How it works

- **Announcements.** The public routes read published rows only. The CMS writers create and edit
  announcements, check thumbnail URLs with the HTTP URL guard and cap the snippet, and push a row
  to Discord when it is published; `DELETE` archives a row, which stays readable to the writers.
  The manual push route refuses anything not published. A push records `pushed_to_discord` and the
  Discord message id; a failed push writes a `crit` audit line instead. Both lists page by `limit`
  and `offset`, and a value that is not an integer answers 400 in the error envelope through
  `ApiError::from_query_rejection`. The modpack create and replace bodies, like the announcement
  bodies, decode through `ApiError::from_json_rejection` (413 `request_too_large`, 415, or 400
  naming the failing field).
- **Uploads.** `POST /api/v1/cms/uploads` takes one multipart `file` field of at most 5 MiB whose
  extension (`jpg`, `jpeg`, `png`, `webp`) and leading bytes agree, stores it under a random name in
  `Config::upload_dir` (`UPLOAD_DIR`) through a staging file and an atomic rename, and answers 201
  with its URL under `/uploads/`, which the API's router serves. Over the limit answers 413, a
  missing field 400, a wrong extension or content 415, and a storage failure 503
  `storage_unavailable`. The route's body limit is `api_http_layer::middleware::MAX_MULTIPART_BODY`.
- **Equipment data viewer.** `equipment_data_viewer/` holds the anonymous, generation-pinned
  reads under `GET /api/v1/debug/equipment-data/*`: dataset status and overview, resource,
  relationship and field listings, source inspection and whole-document downloads. The domain's
  `routes.rs` registers them only when the configuration reports a development environment, so a
  production router answers 404. A failure a handler raises answers 400 in the `{error}` envelope,
  and a JSON answer larger than the viewer's page limit is refused.
- **Modpacks.** A write replaces a pack's whole mod list, so a pack and its `game.mods[]` entries
  never disagree; at most one pack is current, and set-current moves that mark.
- **Vehicles.** `vehicle_database/` holds the list and single-row reads and the create, replace,
  partial edit and soft delete of the identification table, which the domain's `routes.rs`
  registers. One validator checks the three write bodies, and each write locks the row and appends
  its audit line in one transaction.
- **Wiki.** `wiki_knowledgebase/` holds the summaries, the article with its parsed blocks, the
  revision history and the administrator's save, which the domain's `routes.rs` registers.
  `PUT /api/v1/wiki/{slug}` names the revision it edits (`base_revision`, `null` to create),
  refuses unsafe markup with its findings, and writes the page, its revision row and its audit
  line in one transaction.

## Boundaries

- Depends on: the domain's models and services (`modpack_lookup`, `wiki_markup`);
  `api_discord` (the announcement webhook); `api_equipment_datasets` (the equipment data viewer
  reads); `api_audit_log` (`write_audit`, `actor_display_name`, `AuditSeverity`, and
  `required_audit::append_actor_audit` for the vehicle writes and the wiki save); `api_state` for the
  application state; `api_http_layer` for the extractors; `api_foundation` for `ApiError`,
  pagination, the text caps and the content URL policy; `http_url_guard` for the URL guard.
- Used by: the domain's `routes.rs`, which registers the flat handlers and the `vehicle_database/`,
  `wiki_knowledgebase/` and `equipment_data_viewer/` handlers; over HTTP, the announcement feed
  and dashboard intel under
  `crates/frontend/pages/command_center_pages/src/`, the wiki, vehicle and modpack pages under
  `crates/frontend/pages/doctrine_pages/src/`, and the content manager under
  `crates/frontend/pages/administration_pages/src/content_manager/`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/api/src/tests/architecture_rules.rs`); the
  Discord embed is sanitised at the webhook sink, never at the CMS write, so the site shows the
  title as authored.
