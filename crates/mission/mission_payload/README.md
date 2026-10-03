# Mission payload

The `mission_payload` crate: the compiler from a
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) document to the editor payload
a [mission](/documentation/glossary/g_to_m.md#mission) version stores, the export envelope and the
version body around it, the terrain bounds it stamps, and the kit alias table the game-document
compiler resolves [registry](/documentation/glossary/n_to_z.md#registry) aliases through.

## Contents

```text
crates/mission/mission_payload/
├── Cargo.toml  the package: `mission_model`, `serde`, `serde_json`, `thiserror`; layout tier 2
└── src/        the payload compiler, the envelope and version body, the bounds, the kit aliases
```

## How it works

`compile_payload` reads the by-id maps and the slots the document store projects to JSON and
builds the payload in the editor payload schema's key order, each list in the document's
`entityOrder`; with `include_orbat` (the Export path) it derives the ORBAT through
`mission_model::orbat`, and it promotes every authored block out of the environment bag onto the
payload root through `mission_model::authored_blocks`. `compile_export` wraps a payload in the
download envelope, `version_body` and `version_body_to_writer` build the body of
`POST /api/v1/missions/{id}/versions` with identical bytes, and `terrain_bounds` answers a
terrain's playable square. `kit_aliases` parses `contracts/rules/kit-aliases.json`, embedded at
build time, once per process.

The [source README](/crates/mission/mission_payload/src/README.md) lists every key the payload
carries.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_payload    # the payload shapes, the version body, the kit aliases, the block round trips
cargo clippy -p mission_payload --all-targets -- -D warnings
```

## Configuration

None: no features and no environment variables. The kit alias table is compiled in.

## Public surface

All also in `prelude`:

- `compile_payload`, `compile_export` (its mission id a `mission_model::ids::MissionId`),
  `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS`, `is_known_editor_payload_top_level`.
- `version_body`, `version_body_to_writer` (fallible: `Error::VersionBodyWrite`).
- `terrain_bounds`.
- `kit_aliases::{KitAliases, load_kit_aliases}`.
- `Error` and `Result`.

## Boundaries

- Depends on: `mission_model` (the ORBAT projection, the authored block registry, `MissionId`),
  `serde`, `serde_json` (`preserve_order`) and `thiserror`; `contracts/rules/kit-aliases.json`
  through `include_str!`.
- Used by:
  - `mission_compiler` and `mission_validation` (`terrain_bounds`, the kit aliases);
  - `mission_operations` and `mission_document`'s tests, and the map engine's `editing::persist`;
  - the API, which reads the kit aliases;
  - the Mission Creator's Save and Export, its validation panel and zone inspector, and the
    mission library's document upload.
- Rules: mission tier 2, one above `mission_model` (`cargo xtask verify crate-tiers`); the
  round trip of a briefing through the CRDT store is the store's test
  (`crates/mission/mission_document/src/tests/payload_round_trips.rs`), since this crate never depends
  on the store.

## Related documentation

- [Mission crates](/crates/mission/README.md) — the category and its dependency rule.
- [Mission editor payload schema](/contracts/definitions/mission-editor-payload.schema.json) —
  the save-time contract of the payload this crate builds.
- [Contract rules](/contracts/rules/README.md) — the kit alias table.
