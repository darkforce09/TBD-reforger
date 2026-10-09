# API community content

The `api_community_content` crate: the
[community content](/documentation/glossary/a_to_f.md#community-content) domain of the
[API](/documentation/glossary/a_to_f.md#api). It serves what members read and administrators
author: the announcement feed and the CMS that writes it, the push that mirrors an announcement to
Discord, CMS image uploads, the doctrine wiki, the vehicle database and the modpack manifests, and
in development the equipment data viewer's anonymous debug reads.

## Contents

```text
crates/api/api_community_content/
├── Cargo.toml  the package: the API kernel crates it builds on, axum, sqlx (`postgres`), pulldown-cmark, layout tier 6
└── src/        the route table, the handlers, the content models, the modpack lookups and the wiki markup reader, the error and the prelude
```

## How it works

`routes(dev)` returns the domain's `/api/v1` route table, which the API's router merges. Reads
take the member extractor and writes the administrator one; the equipment data viewer reads are
registered only when `dev` is true. The handlers read and write the content tables through the
pool of the application state (`api_state`), leave their audit lines through `api_audit_log`,
push announcements through the webhook of `api_discord` and read the equipment datasets of
`api_equipment_datasets`. The modpack lookups (`load_modpack`, `load_current_modpack`) are the one
pack-plus-mods read every domain uses. The source README holds the routes one by one.

## Getting started

Run from the repository root:

```bash
cargo test -p api_community_content
cargo clippy -p api_community_content --all-targets -- -D warnings
cargo xtask db test-it --test community_content_reads --test contract_parity_equipment_viewer --test content_storage --test route_acceptance_administration_center_content
```

The unit tests need no database: the wiki markup reader with its block goldens (validated against
`contracts/definitions/wiki-page.schema.json`), the upload format checks and store, the vehicle
body validation and the announcement handler source pins. The integration suites of `crates/api/api_server`
prove the routes against Postgres.

## Configuration

No feature and no variable of its own; the handlers read the upload directory
(`Config::upload_dir`) and the webhook from the application state, and the router passes `dev`.

## Public surface

- `routes` (`routes::routes`): the `/api/v1` route table.
- `handlers`: one module per content surface; `announcement_discord_push::webhook_announcement`
  (the webhook input of an announcement) and `media_upload::MAX_UPLOAD_BYTES` are read by the
  integration suites.
- `models`: `Announcement`, `AnnouncementStatus`, `AnnouncementTag`, `Modpack`, `ModpackMod`,
  `VehicleDatabase`, `WikiPage` and the wiki wire shapes.
- `services`: `modpack_lookup` (`ModpackDto`, `with_mods`, `load_modpack`,
  `load_current_modpack`) and `wiki_markup` (`read_markup`, the block tree and the findings).
- `Error` and `Result` (a read failure a caller keeps apart, converting into `ApiError`), and
  `prelude`.

## Boundaries

- Depends on: `api_state`, `api_http_layer`, `api_foundation`, `api_discord`,
  `api_equipment_datasets`, `api_audit_log`, `api_identifiers`, `contract_schema_types`,
  `fleet_wire_contract`, `http_url_guard`; axum, sqlx, pulldown-cmark, serde, tokio, tracing and
  uuid; the tables of `crates/api/api_database/migrations/`.
- Used by: the API application (`crates/api/api_server`): its router, the server infrastructure, missions and
  command center domains (the modpack and announcement models and lookups) and the integration
  suites.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); a domain crate names no
  domain outside its edges of the domain graph, here none
  (`crates/api/api_server/src/tests/architecture_rules.rs`).

## Related documentation

- [API community content source](/crates/api/api_community_content/src/README.md) — the routes
  and how the feed, the wiki, the vehicles, the modpacks and the uploads work.
- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
