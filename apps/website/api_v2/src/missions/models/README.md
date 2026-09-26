# Missions models

The rows and wire shapes of the [missions](/documentation_v2/glossary/g_to_m.md#missions) domain: the
[mission](/documentation_v2/glossary/g_to_m.md#mission) library row with its versions and
[armory](/documentation_v2/glossary/a_to_f.md#armory), reviews of
[artifacts](/documentation_v2/glossary/a_to_f.md#artifact),
[mission deployments](/documentation_v2/glossary/g_to_m.md#mission-deployment), the faction library and
the item [registry](/documentation_v2/glossary/n_to_z.md#registry). Keys are snake_case, absent values
are skipped and timestamps are RFC 3339.

## Contents

```text
apps/website/api_v2/src/missions/models/
├── faction.rs             `UserFaction`, one reusable faction of the caller's faction library
├── generated/             types generated from the mission review and mission deployment schemas
├── mission.rs             the library row, its versions and armory lines, enums and override rows
├── mission_deployment.rs  deployment bodies, transitions, and what operators and runtimes read
├── mission_review.rs      a review, the review thread, and the approve, reject and comment bodies
├── mod.rs                 the module tree; re-exports the faction, mission and registry models
└── registry.rs            `RegistryItem` and `RegistryCompatEdge`, one modpack's catalog and its graph
```

## How it works

`MissionStatus`, `TerrainType`, `GameMode` and `WeatherType` map to the Postgres enums of the same
names in snake_case (`mission_status`, `terrain_type`, `game_mode`, `weather_type`), and a mission
moves through `draft`, `pending_approval`, `live`, `rejected` and `archived`. `Mission` names its
current version and, once approved, `approved_artifact_id`, the artifact
[deployments](/documentation_v2/glossary/a_to_f.md#deployment) load. A `MissionVersion` is written once,
unique per mission and semver, and carries its payload as `RawJson`, the bytes Postgres stored,
never re-serialized. A `MissionArmory` line with no quantity is unlimited. No struct carries a
soft-delete column; the queries filter deleted rows. The camelCase documents (the compiled mission
document, the export envelope, the loadout export) live in `services` and `contract`, not here.

Request bodies (`DeploymentRequest`, `RelayedDeploymentRequest`, `ApprovalDecision`,
`RejectionDecision`, `ReviewCommentRequest`) refuse unknown fields. `DeploymentTransition` holds
each transition's wire name and confirmation deadline: `scenario_restart` 600 s, `host_restart`
1200 s.

## Boundaries

- Depends on: `core::wire_format` for timestamps and `RawJson`; serde and sqlx. `generated/`
  follows `contracts_v2/definitions/mission-review.schema.json` and
  `contracts_v2/definitions/mission-deployment.schema.json`; `faction.rs` and `registry.rs` carry
  the `@contract` tags of `faction-library.schema.json`, `registry-items.schema.json` and
  `registry-compat.schema.json`.
- Used by: the domain's handlers and services; `operations` (`MissionArmory`, `TerrainType`,
  `GameMode` in the [event](/documentation_v2/glossary/a_to_f.md#event) hub), `match_telemetry` and
  `server_infrastructure` (`TerrainType`); the [API](/documentation_v2/glossary/a_to_f.md#api) tests
  `apps/website/api_v2/tests/models_serde.rs`, `apps/website/api_v2/tests/models_fromrow.rs` and
  `apps/website/api_v2/tests/mission_review_contract.rs`, which decodes live answers into the
  generated types. The web app's DTOs in `apps/website/frontend/src/v2/core/api/dto/`
  (`missions.rs`, `mission_reviews.rs`, `mission_deployments.rs`, `registry.rs`) mirror these
  shapes.
- Rules: `generated/` is written by `cargo xtask ci schema-codegen` and never edited by hand
  (`cargo xtask ci verify-codegen-fresh` checks it); every `@contract` tag resolves against
  `contracts_v2/definitions/` (`cargo xtask schema citations`); a stored version is never updated,
  and the `mission_versions_are_immutable` trigger of
  `apps/website/api_v2/migrations/0052_mission_version_immutability.sql` refuses any update.
