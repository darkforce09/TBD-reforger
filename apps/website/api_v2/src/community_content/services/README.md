# Community content services

The pieces of community content that other code shares: the Discord webhook that announcements
are pushed through, the modpack read path that returns a pack with its mods, the wiki markup
reader that turns a doctrine page's markdown into the typed tree the wiki page renders, and the
equipment data viewer's dataset import and read queries.

## Contents

```text
apps/website/api_v2/src/community_content/services/
├── discord_webhook.rs      `WebhookService`: posts an announcement embed to Discord, answers the message id
├── equipment_data_viewer/  verified imports of the equipment datasets, and their read queries
├── mod.rs                  the module tree
├── modpack_lookup.rs       `ModpackDto` and its reads: by id, the current pack, and mod hydration
├── tests/                  unit tests for the embed sanitiser and the webhook payload
└── wiki_markup/            `read_markup`: a wiki page's markdown as safe typed blocks, and the save's refusal findings
```

## How it works

`WebhookService` holds the `DISCORD_WEBHOOK_URL` the configuration reads; an empty URL disables
pushing. Before posting, `sanitize_discord_embed_field` strips ASCII control characters and puts a
zero-width space before a leading `=`, `+`, `-` or `@`, so a copied title cannot become a
spreadsheet formula. `modpack_lookup.rs` owns the one pack-plus-mods query: `load_modpack`,
`load_current_modpack` and `with_mods` return a `ModpackDto`, the pack with its ordered mods
flattened into one object, and the `modpack_cols!` projection keeps every modpack read
null-tolerant in the same way. `wiki_markup/` parses a page's `body_md` with pulldown-cmark into
headings, paragraphs, lists, tables, callouts, quotes, code and rules, replacing each unsafe link
or image, raw HTML and nesting deeper than 16 with a safe form and recording it as a finding with
its line; its README holds the tree and the refusal rules. `equipment_data_viewer/` imports each
published generation of the Workbench equipment export into a local data directory with a SQLite
navigation index, and answers the development-only equipment data viewer routes from one pinned
generation; nothing in it reads Postgres.

## Boundaries

- Depends on: the domain's models; `core` for the HTTP retry helper, errors and the content URL
  policy; reqwest; pulldown-cmark (`wiki_markup/`); `sqlx` with SQLite and
  `contracts/rules/equipment-gameplay/native-matching.json` (`equipment_data_viewer/`).
- Used by: the domain's handlers, the wiki handlers reading and saving through `wiki_markup/`;
  `core::application_state`, which holds the one `WebhookService` and the `EquipmentDatasets`;
  the `equipment_export_watcher` background worker;
  `command_center`'s dashboard (`load_current_modpack`) and `server_infrastructure`'s server intel
  (`load_modpack`, `ModpackDto`); the integration test
  `apps/website/api_v2/tests/discord_embed_sanitisation.rs`, and the equipment viewer's
  `apps/website/api_v2/tests/contract_parity_equipment_viewer.rs`.
- Rules: a domain that needs a modpack calls `modpack_lookup.rs` rather than writing its own query;
  the embed is sanitised here, at the sink, and nowhere else; a wiki page's markdown is parsed
  only by `wiki_markup/`, for the reads and the save alike.
