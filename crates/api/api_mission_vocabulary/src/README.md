# API mission vocabulary source

The source of `api_mission_vocabulary`: the mission enums more than one domain names, the terrain
a mission, a match and a server's running mission are on, and the game mode a mission is played
in. The rest of the mission row stays in `crates/api/api_missions/src/models/`.

## Contents

```text
crates/api/api_mission_vocabulary/src/
├── error.rs         `Error`: a string that names no terrain or game mode, and `Result`
├── game_mode.rs     `GameMode`, the `game_mode` enum, and its parse
├── lib.rs           the crate root: module header, `mod` lines and the re-exports
├── prelude.rs       `GameMode` and `TerrainType` for glob import
├── terrain_type.rs  `TerrainType`, the `terrain_type` enum, and its parse
└── tests/           the spelling tests: wire, SQL and parse agree, unknown values are refused
```

## How it works

Each enum derives serde and `sqlx::Type` with snake_case names, so the JSON value and the Postgres
enum value are the same string; `as_str` returns it and `FromStr` reads exactly that string back,
refusing any other spelling with an `Error` that carries the string as given.

## Boundaries

- Depends on: serde, sqlx and thiserror; no other API crate.
- Used by: `api_missions` (the mission row and its validation), `api_match_telemetry` (the match record
  and its ingest parsing), `api_operations` (the event hub) and `api_server_infrastructure` (server
  intel); the integration test `crates/api/api_server/tests/models_serde.rs`.
- Rules: an enum moves here only when a domain other than `api_missions` names it; each enum and its
  Postgres enum in `crates/api/api_database/migrations/` change together.
