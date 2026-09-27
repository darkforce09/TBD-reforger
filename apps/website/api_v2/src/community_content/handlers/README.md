# Community content handlers

The HTTP handlers of what members read and administrators author: announcements and their Discord
push, the doctrine wiki, the vehicle database, modpack manifests and CMS image uploads. Members read
with `AuthUser`; every write takes `AdminUser`.

## Contents

```text
apps/website/api_v2/src/community_content/handlers/
├── announcement_discord_push.rs  pushes a published announcement to Discord and records the result
├── announcements_admin.rs        the CMS announcement list, create, partial edit and archive
├── announcements_public.rs       the member feed: published announcements, and one of them
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
  `ApiError::from_query_rejection`.
- **Uploads.** `POST /api/v1/cms/uploads` takes one multipart `file` field of at most 5 MiB whose
  extension (`jpg`, `jpeg`, `png`, `webp`) and leading bytes agree, stores it under a random name in
  `Config::upload_dir` (`UPLOAD_DIR`) through a staging file and an atomic rename, and answers 201
  with its URL under `/uploads/`, which `core::http_router` serves. Over the limit answers 413, a
  missing field 400, a wrong extension or content 415, and a storage failure 503
  `storage_unavailable`. The route's body limit is `core::middleware::MAX_MULTIPART_BODY`.
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

- Depends on: the domain's models and services (`discord_webhook`, `modpack_lookup`,
  `wiki_markup`); `administration` (`write_audit`, `actor_display_name`, `AuditSeverity`, and
  `required_audit::append_actor_audit` for the vehicle writes and the wiki save); `core` for the
  extractors, pagination, the HTML sanitizer, the URL guard, the content URL policy and the
  configuration.
- Used by: the domain's `routes.rs`, which registers the flat handlers and the `vehicle_database/`
  and `wiki_knowledgebase/` handlers; over HTTP, the announcement feed
  and dashboard intel under
  `apps/website/frontend/src/v2/pages/command_center/`, the wiki, vehicle and modpack pages under
  `apps/website/frontend/src/v2/pages/doctrine_and_info/`, and the content manager under
  `apps/website/frontend/src/v2/pages/administration/content_manager/`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/website/api_v2/src/tests/architecture_rules.rs`); the
  Discord embed is sanitised at the webhook sink, never at the CMS write, so the site shows the
  title as authored.
