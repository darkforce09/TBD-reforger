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
├── media_upload.rs               the CMS image upload into the configured upload directory
├── mod.rs                        the module tree
├── modpack_admin.rs              modpack create, full replace, set-current and delete
├── modpack_catalog.rs            every modpack with its mods, and the current one
├── tests/                        unit tests for the Discord push and the CMS announcement writers
├── vehicle_database.rs           the vehicle database table and its admin row writer
└── wiki_knowledgebase.rs         the wiki navigation list, one markdown page, the admin page writer
```

## How it works

- **Announcements.** The public routes read published rows only. The CMS writers create and edit
  announcements, check thumbnail URLs with the HTTP URL guard and cap the snippet, and push a row
  to Discord when it is published; `DELETE` archives a row, which stays readable to the writers.
  The manual push route refuses anything not published. A push records `pushed_to_discord` and the
  Discord message id; a failed push writes a `crit` audit line instead.
- **Uploads.** `POST /api/v1/cms/uploads` takes one multipart `file` field of at most 5 MB, a JPG,
  PNG or WEBP, stores it under a random name in `Config::upload_dir` (`UPLOAD_DIR`) and answers its
  URL under `/uploads/`, which `core::http_router` serves. The route's body limit is
  `core::middleware::MAX_MULTIPART_BODY`.
- **Modpacks.** A write replaces a pack's whole mod list, so a pack and its `game.mods[]` entries
  never disagree; at most one pack is current, and set-current moves that mark.
- **Wiki and vehicles.** `PUT /api/v1/wiki/{slug}` creates or replaces a markdown page, and
  `POST /api/v1/vehicle-database` adds a row to the vehicle table.

## Boundaries

- Depends on: the domain's models and services (`discord_webhook`, `modpack_lookup`);
  `administration` (`write_audit`, `actor_display_name`, `AuditSeverity`); `core` for the
  extractors, pagination, the HTML sanitizer, the URL guard and the configuration.
- Used by: the domain's `routes.rs`; over HTTP, the announcement feed and dashboard intel under
  `apps/website/frontend/src/v2/pages/command_center/`, the wiki, vehicle and modpack pages under
  `apps/website/frontend/src/v2/pages/doctrine_and_info/`, and the content manager under
  `apps/website/frontend/src/v2/pages/administration/content_manager/`.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/website/api_v2/src/tests/architecture_rules.rs`); the
  Discord embed is sanitised at the webhook sink, never at the CMS write, so the site shows the
  title as authored.
