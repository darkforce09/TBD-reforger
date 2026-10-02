# Community content domain

The [API](/documentation/glossary/a_to_f.md#api)'s
[community content](/documentation/glossary/a_to_f.md#community-content) domain: what members read and
administrators author. It holds the announcement feed and the CMS that writes it, the push that
mirrors an announcement to Discord, CMS image uploads, the doctrine wiki, the vehicle database, and
the modpack manifests that game servers and players resolve against. In development it also
serves the equipment data viewer's anonymous debug reads.

## Contents

```text
apps/api/src/community_content/
├── handlers/  one handler module per content surface: feed, CMS, uploads, wiki, vehicles, modpacks
├── mod.rs     the module tree; re-exports `routes`
├── models/    the announcement, modpack, wiki page and vehicle rows
├── routes.rs  the domain's `/api/v1` route table
└── services/  the Discord announcement webhook, the modpack read path and the wiki markup reader
```

## How it works

Reads take `AuthUser`, so any signed-in member sees the published feed, the wiki, the vehicle
table and the modpacks; every write takes `AdminUser`. The CMS routes under `/api/v1/cms/*` serve
the [content manager](/documentation/glossary/a_to_f.md#content-manager) page: an announcement is pushed
to Discord through `services::discord_webhook::WebhookService` when it is published, and archiving
it keeps the row. An uploaded image lands in the directory `UPLOAD_DIR` names
(`Config::upload_dir`), which `core::http_router` serves at `/uploads`. A modpack is always written
with its whole mod list, and at most one pack at a time is marked current. The vehicle database
writes and the wiki save append their audit line in the write's own transaction through
`administration::services::required_audit`; the other administrator writes leave best-effort
audit lines through `administration::services::audit_writer`. A wiki page's markdown is read by
`services::wiki_markup` into typed blocks that render safely; a save names the revision it
edits, is refused with every finding when its markup holds an unsafe link or image, raw HTML or
nesting deeper than 16, and records each accepted save as a numbered revision.

## Public surface

- `routes::routes(dev)`: the table `core::http_router` merges under `/api/v1`. It registers the
  flat handlers and the `handlers::vehicle_database`, `handlers::wiki_knowledgebase` and
  `handlers::equipment_data_viewer` handlers itself; one route each:
  - `GET /api/v1/announcements` and `GET /api/v1/announcements/{id}`: `AuthUser`; published rows.
  - `GET` and `POST /api/v1/cms/announcements`: `AdminUser`; every row, and create.
  - `PATCH` and `DELETE /api/v1/cms/announcements/{id}`: `AdminUser`; partial edit, archive.
  - `POST /api/v1/cms/announcements/{id}/push-discord`: `AdminUser`; push a published row again.
  - `POST /api/v1/cms/uploads`: `AdminUser`; one multipart JPEG, PNG or WebP image of at most
    5 MiB, answered 201 with its URL.
  - `GET /api/v1/wiki`: `AuthUser`; the page summaries in navigation order.
  - `GET` and `PUT /api/v1/wiki/{slug}`: `AuthUser` to read the article with its parsed blocks,
    `AdminUser` to create the page (`base_revision: null`, 201) or save its next revision (200).
  - `GET /api/v1/wiki/{slug}/revisions`: `AuthUser`; one page of the revision history, newest
    first.
  - `GET /api/v1/wiki/{slug}/revisions/{revision}`: `AuthUser`; the page as one revision saved it.
  - `GET` and `POST /api/v1/vehicle-database`: `AuthUser` to read the live rows, `AdminUser` to
    add a row (201).
  - `GET`, `PUT`, `PATCH` and `DELETE /api/v1/vehicle-database/{id}`: `AuthUser` to read one row,
    `AdminUser` to replace it, change some of its fields or soft-delete it.
  - `GET` and `POST /api/v1/modpacks`: `AuthUser` to list, `AdminUser` to create.
  - `GET /api/v1/modpacks/current`: `AuthUser`; the current pack.
  - `PUT` and `DELETE /api/v1/modpacks/{id}`: `AdminUser`; full replace, delete.
  - `POST /api/v1/modpacks/{id}/set-current`: `AdminUser`; mark the current pack.
  - `GET /api/v1/debug/equipment-data/{status, overview, resources, relationships, fields,
    resource-cards, selection, containers, properties, values, documents, download}`: anonymous,
    registered only when `dev` is true (`Config::is_development`), so a production router answers
    404; generation-pinned reads of the exported equipment datasets.
- `services::modpack_lookup`: `ModpackDto`, `load_modpack` and `load_current_modpack`, the one
  pack-plus-mods read, used by the dashboard in `command_center` and the server intel in
  `server_infrastructure`.
- `services::discord_webhook::WebhookService`: the webhook sink `core::application_state` holds.
- `services::wiki_markup::read_markup`: a wiki page's markdown as safe blocks and refusal
  findings, used by the wiki handlers.
- `models`: `Announcement`, read by the dashboard, and `Modpack` with `ModpackMod`, read by
  `server_infrastructure` and by the [registry](/documentation/glossary/n_to_z.md#registry) items in
  `missions`.

## Boundaries

- Depends on: `core` (the application state, configuration, errors, extractors, pagination, the
  HTML sanitizer, the URL guard, the content URL policy, the HTTP retry helper, wire formats) and
  `administration::{models, services}` for the audit lines its writes leave.
- Used by:
  - `core::http_router`, which merges the route table, and `core::application_state`;
  - `command_center`, `server_infrastructure` and `missions`, through the services and models
    above;
  - over HTTP, the command center, doctrine and content manager pages in
    `apps/frontend/src/v2/pages/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`apps/api/src/tests/architecture_rules.rs` checks both); a handler
  folder with its own `routes()` holds every registration of its routes, and `routes.rs` only
  merges it; every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); the
  pack-plus-mods query lives only in `services/modpack_lookup.rs`, and `handlers/media_upload/`
  is the only writer of the upload directory.

## Related documentation

- [API overview](/documentation/apps/api/api_overview.md) — every domain's routes.
- [API environment variables](/documentation/apps/api/environment_variables.md)
  — `DISCORD_WEBHOOK_URL` and
  `UPLOAD_DIR`, which the announcement push and the uploads read.
- [Content manager page](/documentation/apps/frontend/pages/administration/content_manager/content_manager_page.md)
  — the CMS that writes announcements and uploads.
