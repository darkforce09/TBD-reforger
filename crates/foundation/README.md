# Foundation crates

The lowest tiers of the library crates: small building blocks that depend on no workspace crate
outside this folder, so every other crate may link them. All are tier 0 leaves except
`orbat_slot_ids`, tier 1 on `newtype_ids`.

## Contents

```text
crates/foundation/
├── browser_platform/  `browser_platform`: browser console macros and same-origin GETs for wasm32 code
├── content_digest/  `content_digest`: lowercase hex SHA-256 and SHA-384, an incremental framed SHA-256
├── deterministic_random/  `deterministic_random`: SplitMix64, the seeded generator of every reproducible draw
├── http_url_guard/  `http_url_guard`: whether a string is an `http` or `https` URL a browser follows
├── newtype_ids/  `newtype_ids`: the `string_id!`, `integer_id!` and `uuid_id!` identifier macros
├── orbat_slot_ids/  `orbat_slot_ids`: `SlotUid`, a slot's durable editor id, and `SlotId`, its derived wire id
├── repository_root/  `repository_root`: the one checkout-root finder, the walk up to the `.ai/tickets/ROOT` marker
└── time_source/  `time_source`: the wall-clock trait and clocks, monotonic time, RFC 3339 UTC stamps
```

## Boundaries

- Depends on: external crates, and `orbat_slot_ids` on `newtype_ids`.
- Used by: the API and the single-page app; `content_digest` by `xtask`; `browser_platform` by
  the map engine's `render` tier; `orbat_slot_ids` by the mission crates, `unit_symbology` and
  `mission_editing_commands`; `newtype_ids` by the typed ids of the mission, ballistics, world
  format, world-object and map overlay crates, by `orbat_slot_ids`, and by `api_identifiers`, the
  API's typed ids; `deterministic_random` by `formation_geometry` and the ballistics solver and
  agreement cases; `time_source` by `mission_crdt`, `mission_document` and `map_editing_tools`;
  `repository_root` by the tool crates, the API's tests and `frontend_test_support`.
- Rules: a foundation crate declares `category = "crates/foundation"` and depends on foundation
  crates only (`cargo xtask verify crate-tiers`).
