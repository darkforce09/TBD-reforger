# Community content models

The rows of community content as the database stores them and the wire carries them: snake_case
keys, absent values skipped, RFC 3339 timestamps, and each enum mapped to a Postgres enum.

## Contents

```text
apps/website/api_v2/src/community_content/models/
├── announcement.rs  `Announcement`, with the `AnnouncementStatus` and `AnnouncementTag` enums
├── mod.rs           the module tree; re-exports every model
├── modpack.rs       `Modpack`, a downloadable dependency set, and `ModpackMod`, one mod inside it
└── wiki.rs          `WikiPage`, a markdown doctrine page, and `VehicleDatabase`, one vehicle row
```

## How it works

Each struct is one row of `announcements`, `modpacks`, `modpack_mods`, `wiki_pages` or
`vehicle_databases`. A `ModpackMod`'s `workshop_id`, `mod_guid` and `version` fill one Reforger
`game.mods[]` entry, and empty strings stay off the wire. Soft-delete columns are absent from the
structs, because the queries filter them.

## Boundaries

- Depends on: `core::wire_format` for timestamps, serde and sqlx.
- Used by: the domain's handlers and services; `command_center`'s dashboard (`Announcement`);
  `server_infrastructure`'s server intel (`Modpack`, `ModpackMod`); `missions`'
  [registry](/documentation_v2/glossary.md#registry) items (`Modpack`); the web app's
  `apps/website/frontend/src/v2/core/api/dto/content.rs` mirrors the modpack wire shape.
- Rules: an enum here and its Postgres enum in `apps/website/api_v2/migrations/` change together.
