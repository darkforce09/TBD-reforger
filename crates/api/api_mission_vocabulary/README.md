# API mission vocabulary

The `api_mission_vocabulary` crate: the terrain and game mode enums that more than one domain of
the [API](/documentation/glossary/a_to_f.md#api) names, so a mission, a match record, a server's
status and an event hub speak one vocabulary without depending on the missions domain.

## Contents

```text
crates/api/api_mission_vocabulary/
├── Cargo.toml  the package: serde, sqlx (`postgres`), thiserror, layout tier 0
└── src/        `TerrainType`, `GameMode`, their parse error, the prelude and the spelling tests
```

## How it works

`TerrainType` (`everon`, `arland`, `custom`) and `GameMode` (`pve_coop`, `pvp`, `zeus`) each hold
the values of the Postgres enum of the same name, spelled snake_case on the wire and in SQL.
`as_str` and `FromStr` are inverse over exactly those values: the mission validation answers a
refused parse as a 400, and the match ingest reads an unknown terrain as absent rather than
failing the report.

## Getting started

Run from the repository root:

```bash
cargo test -p api_mission_vocabulary   # the wire, SQL and parse spellings agree; no database
cargo xtask db test-it --test models_serde
```

## Configuration

No feature and no variable.

## Public surface

- `TerrainType` and `GameMode`, each with `as_str` and `FromStr`.
- `Error` (`UnknownTerrain`, `UnknownGameMode`) and `Result`, and `prelude` (both enums).

## Boundaries

- Depends on: serde, sqlx and thiserror; the `terrain_type` and `game_mode` enums of
  `crates/api/api_database/migrations/`.
- Used by: the API application (`crates/api/api_server`): missions, match telemetry, operations, server
  infrastructure and the integration suites.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); an enum moves here only when a
  domain other than `api_missions` names it.

## Related documentation

- [API mission vocabulary source](/crates/api/api_mission_vocabulary/src/README.md) — the files
  and the spelling contract.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
