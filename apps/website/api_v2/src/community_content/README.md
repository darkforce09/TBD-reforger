# `community_content/`

Authored, member-facing content: the announcement feed and the CMS that writes it, the Discord push
that mirrors an announcement to the guild, multipart media uploads, the doctrine knowledgebase, the
vehicle / IFF database, and the modpack manifests a server and a player both resolve against.

## Public surface

- **`routes::routes()`** — the domain's `/api/v1` table, merged by `core::http_router::api_v1_routes`
  and nested under `/api/v1`. The literals in `routes.rs` are the public URLs: `/announcements`,
  `/announcements/{id}`, `/cms/announcements`, `/cms/announcements/{id}`,
  `/cms/announcements/{id}/push-discord`, `/cms/uploads`, `/wiki`, `/wiki/{slug}`,
  `/vehicle-database`, `/modpacks`, `/modpacks/current`, `/modpacks/{id}`,
  `/modpacks/{id}/set-current`.
- **`services::modpack_lookup`** — `ModpackDto`, `load_modpack`, and `load_current_modpack`: the
  single pack-plus-nested-mods read path. `server_infrastructure` and the dashboard consume it.
- **`services::discord_webhook::WebhookService`** — the announcement webhook sink. `AppState` holds
  one instance, so `core::application_state.rs` names this type.
- **`models::announcement::Announcement`** and **`models::modpack::Modpack`** — the rows other
  surfaces project.

## Dependency rules

- Handlers here never import another domain's handlers. A domain that needs a modpack calls
  `services::modpack_lookup`, which is the only place the pack-plus-mods query lives.
- This domain imports `core`, plus `administration::{models, services}` for the papertrail its admin
  writes leave. It imports no other domain's code.
- `core::application_state.rs` is the one `core` file allowed to name this domain, because it owns
  the `WebhookService` instance.
- Uploaded media is written under the directory `handlers/media_upload.rs` owns; no other module
  resolves that path.

## Files

```text
mod.rs                                 Domain module tree; re-exports `routes`.
routes.rs                              The `/api/v1` route table for community content.
handlers/
  mod.rs                               One module per content surface.
  announcement_discord_push.rs         Pushing an announcement to the Discord announcements channel.
  announcements_admin.rs               The CMS announcement surface: list, create, partial edit, archive.
  announcements_public.rs              The member-facing feed: the published list and one published row.
  media_upload.rs                      CMS media uploads and the local storage directory they land in.
  modpack_admin.rs                     Modpack authoring: create, full replace, set-current, delete.
  modpack_catalog.rs                   The member-facing catalog: every pack with its mods, and the active manifest.
  vehicle_database.rs                  The vehicle / IFF catalog: the member table and the admin row writer.
  wiki_knowledgebase.rs                The doctrine knowledgebase: SOP navigation, one markdown page, admin writes.
  tests/
    announcement_discord_push.rs       Sibling unit tests for `announcement_discord_push.rs`.
    announcements_admin.rs             Sibling unit tests for `announcements_admin.rs`.
services/
  mod.rs                               The Discord announcement webhook sink and the modpack read path.
  discord_webhook.rs                   Announcement → Discord webhook embed dispatch.
  modpack_lookup.rs                    Modpack manifest loading: the DTO and its three read paths.
  tests/
    discord_webhook.rs                 Sibling unit tests for `discord_webhook.rs`.
models/
  mod.rs                               Community-content wire/database models.
  announcement.rs                      The news-feed / CMS row and its two Postgres ENUM vocabularies.
  modpack.rs                           The downloadable dependency set and its nested mod rows.
  wiki.rs                              The markdown SOP page and the vehicle IFF table row.
```

Unit tests live in the sibling files above, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
