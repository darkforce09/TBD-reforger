# Community content services

The two pieces of community content that other code shares: the Discord webhook that announcements
are pushed through, and the modpack read path that returns a pack with its mods.

## Contents

```text
apps/website/api_v2/src/community_content/services/
├── discord_webhook.rs  `WebhookService`: posts an announcement embed to Discord, answers the message id
├── mod.rs              the module tree
├── modpack_lookup.rs   `ModpackDto` and its reads: by id, the current pack, and mod hydration
└── tests/              unit tests for the embed sanitiser and the webhook payload
```

## How it works

`WebhookService` holds the `DISCORD_WEBHOOK_URL` the configuration reads; an empty URL disables
pushing. Before posting, `sanitize_discord_embed_field` strips ASCII control characters and puts a
zero-width space before a leading `=`, `+`, `-` or `@`, so a copied title cannot become a
spreadsheet formula. `modpack_lookup.rs` owns the one pack-plus-mods query: `load_modpack`,
`load_current_modpack` and `with_mods` return a `ModpackDto`, the pack with its ordered mods
flattened into one object, and the `modpack_cols!` projection keeps every modpack read
null-tolerant in the same way.

## Boundaries

- Depends on: the domain's models; `core` for the HTTP retry helper and errors; reqwest.
- Used by: the domain's handlers; `core::application_state`, which holds the one `WebhookService`;
  `command_center`'s dashboard (`load_current_modpack`) and `server_infrastructure`'s server intel
  (`load_modpack`, `ModpackDto`); the integration test
  `apps/website/api_v2/tests/discord_embed_sanitisation.rs`.
- Rules: a domain that needs a modpack calls `modpack_lookup.rs` rather than writing its own query;
  the embed is sanitised here, at the sink, and nowhere else.
