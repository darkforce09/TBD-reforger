# Match telemetry models

The stored record of a finished match: the match row, its outcome enum (`success`, `failure`,
`aborted` or `pending`) and the per-player lines the game server reports. Keys are snake_case,
absent values are skipped and timestamps are RFC 3339.

## Contents

```text
apps/website/api_v2/src/match_telemetry/models/
├── match_record.rs  `Match`, its `MissionOutcome` and the per-player `MatchPlayerStat` lines
└── mod.rs           the module tree; re-exports the three types
```

## Boundaries

- Depends on: `core::wire_format` for timestamps; `missions::models::mission::TerrainType`; serde
  and sqlx.
- Used by: the domain's handlers (`MissionOutcome`); the member's
  [service record](/documentation_v2/glossary.md#service-record) in
  `apps/website/api_v2/src/operations/handlers/member_service_record.rs` (`Match`,
  `MatchPlayerStat`); the null-tolerance integration tests in `apps/website/api_v2/tests/`.
- Rules: `Match.winning_faction`, `aar_replay_url` and `created_at` are plain fields over nullable
  columns, so every query that reads them coalesces them; `MissionOutcome` and the
  `mission_outcome` enum in the migrations change together.
