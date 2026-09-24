# Community content domain

The [API](/documentation_v2/glossary.md#api)'s
[community content](/documentation_v2/glossary.md#community-content) domain: what members read and
administrators author. It holds the announcement feed and the CMS that writes it, the push that
mirrors an announcement to Discord, CMS image uploads, the doctrine wiki, the vehicle database, and
the modpack manifests that game servers and players resolve against.

## Contents

```text
apps/website/api_v2/src/community_content/
├── handlers/  one handler module per content surface: feed, CMS, uploads, wiki, vehicles, modpacks
├── mod.rs     the module tree; re-exports `routes`
├── models/    the announcement, modpack, wiki page and vehicle rows
├── routes.rs  the domain's `/api/v1` route table
└── services/  the Discord announcement webhook and the modpack read path
```

## How it works

Reads take `AuthUser`, so any signed-in member sees the published feed, the wiki, the vehicle
table and the modpacks; every write takes `AdminUser`. The CMS routes under `/api/v1/cms/*` serve
the [content manager](/documentation_v2/glossary.md#content-manager) page: an announcement is pushed
to Discord through `services::discord_webhook::WebhookService` when it is published, and archiving
it keeps the row. An uploaded image lands in the directory `UPLOAD_DIR` names
(`Config::upload_dir`), which `core::http_router` serves at `/uploads`. A modpack is always written
with its whole mod list, and at most one pack at a time is marked current. Administrator writes
leave best-effort audit lines through `administration::services::audit_writer`.

## Public surface

- `routes::routes()`: the table `core::http_router` merges under `/api/v1`, one route each:
  - `GET /api/v1/announcements` and `GET /api/v1/announcements/{id}`: `AuthUser`; published rows.
  - `GET` and `POST /api/v1/cms/announcements`: `AdminUser`; every row, and create.
  - `PATCH` and `DELETE /api/v1/cms/announcements/{id}`: `AdminUser`; partial edit, archive.
  - `POST /api/v1/cms/announcements/{id}/push-discord`: `AdminUser`; push a published row again.
  - `POST /api/v1/cms/uploads`: `AdminUser`; one multipart image of at most 5 MB.
  - `GET /api/v1/wiki`: `AuthUser`; the navigation list.
  - `GET` and `PUT /api/v1/wiki/{slug}`: `AuthUser` to read, `AdminUser` to create or replace.
  - `GET` and `POST /api/v1/vehicle-database`: `AuthUser` to read, `AdminUser` to add a row.
  - `GET` and `POST /api/v1/modpacks`: `AuthUser` to list, `AdminUser` to create.
  - `GET /api/v1/modpacks/current`: `AuthUser`; the current pack.
  - `PUT` and `DELETE /api/v1/modpacks/{id}`: `AdminUser`; full replace, delete.
  - `POST /api/v1/modpacks/{id}/set-current`: `AdminUser`; mark the current pack.
- `services::modpack_lookup`: `ModpackDto`, `load_modpack` and `load_current_modpack`, the one
  pack-plus-mods read, used by the dashboard in `command_center` and the server intel in
  `server_infrastructure`.
- `services::discord_webhook::WebhookService`: the webhook sink `core::application_state` holds.
- `models`: `Announcement`, read by the dashboard, and `Modpack` with `ModpackMod`, read by
  `server_infrastructure` and by the [registry](/documentation_v2/glossary.md#registry) items in
  `missions`.

## Boundaries

- Depends on: `core` (the application state, configuration, errors, extractors, pagination, the
  HTML sanitizer, the URL guard, the HTTP retry helper, wire formats) and
  `administration::{models, services}` for the audit lines its writes leave.
- Used by:
  - `core::http_router`, which merges the route table, and `core::application_state`;
  - `command_center`, `server_infrastructure` and `missions`, through the services and models
    above;
  - over HTTP, the command center, doctrine and content manager pages in
    `apps/website/frontend/src/v2/pages/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`apps/website/api_v2/src/tests/architecture_rules.rs` checks both); every handler
  carries its `/// @route` tag (`cargo xtask verify route-tags`); the pack-plus-mods query lives
  only in `services/modpack_lookup.rs`, and `handlers/media_upload.rs` is the only writer of the
  upload directory.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes.
- [Content manager page](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md)
  — the CMS that writes announcements and uploads.
