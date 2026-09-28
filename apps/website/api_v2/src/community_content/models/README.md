# Community content models

The rows of community content as the database stores them and the wire carries them: snake_case
keys, absent values skipped, RFC 3339 timestamps, and each enum mapped to a Postgres enum.

## Contents

```text
apps/website/api_v2/src/community_content/models/
├── announcement.rs      `Announcement`, with the `AnnouncementStatus` and `AnnouncementTag` enums
├── generated/           typify types of the equipment data viewer, vehicle, wiki and upload schemas
├── mod.rs               the module tree; re-exports every model
├── modpack.rs           `Modpack`, a downloadable dependency set, and `ModpackMod`, one of its mods
├── vehicle_database.rs  `VehicleDatabase`, one vehicle row, and `VehicleDatabaseList`, the list
└── wiki.rs              the wiki page and revision rows, summaries, article, save body and refusal, history
```

## How it works

Each struct is one row of `announcements`, `modpacks`, `modpack_mods`, `wiki_pages` or
`vehicle_databases`. A `ModpackMod`'s `workshop_id`, `mod_guid` and `version` fill one Reforger
`game.mods[]` entry, and empty strings stay off the wire. Soft-delete columns are absent from the
structs, because the queries filter them. `wiki.rs` holds the `wiki_pages` and
`wiki_page_revisions` rows and the wiki's wire shapes beside them: the summaries, the article and
the revision, which embed the `WikiBlock` tree of `services::wiki_markup`, the save body
`WikiSaveRequest` (every field required, unknown fields refused) and the `WikiSaveRefusal`
details, which carry `WikiMarkupFinding`s.

## Boundaries

- Depends on: `core::wire_format` for timestamps, serde and sqlx; `services::wiki_markup` for the
  wiki blocks and findings. `generated/` follows the
  schemas under `contracts_v2/definitions/equipment-data-viewer/`, and
  `vehicle-database.schema.json` (`Vehicle`, `VehicleList`, `VehicleWrite`, `VehiclePatch`),
  `wiki-page.schema.json` (summaries, the article with its typed `WikiBlock` and `WikiInline`
  tree, the save and its refusal, the revisions) and `content-upload.schema.json`
  (`UploadResponse`, `ContentError`, `ContentRefusal`) in `contracts_v2/definitions/`.
  `announcement.rs` cites `announcement.schema.json` (`Announcement`, `AnnouncementTag`,
  `AnnouncementStatus`) and `modpack.rs` cites `modpack.schema.json` (`Modpack`, `ModpackMod`)
  with `@contract` tags, which `cargo xtask schema citations` resolves.
- Used by: the domain's handlers and services; `command_center`'s dashboard (`Announcement`);
  `server_infrastructure`'s server intel (`Modpack`, `ModpackMod`); `missions`'
  [registry](/documentation_v2/glossary/n_to_z.md#registry) items (`Modpack`); the web app's
  `apps/website/frontend/src/v2/core/api/dto/content.rs` mirrors the modpack wire shape.
- Rules: an enum here and its Postgres enum in `apps/website/api_v2/migrations/` change together;
  `generated/` is written by `cargo xtask ci schema-codegen` and never edited by hand
  (`cargo xtask ci verify-codegen-fresh` checks it).
