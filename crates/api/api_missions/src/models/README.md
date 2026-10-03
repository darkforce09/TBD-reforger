# Missions models

The rows and wire shapes of the [missions](/documentation/glossary/g_to_m.md#missions) domain: the
[mission](/documentation/glossary/g_to_m.md#mission) library row with its versions and
[armory](/documentation/glossary/a_to_f.md#armory), reviews of
[artifacts](/documentation/glossary/a_to_f.md#artifact),
[mission deployments](/documentation/glossary/g_to_m.md#mission-deployment), the faction library and
the item [registry](/documentation/glossary/n_to_z.md#registry). Keys are snake_case, absent values
are skipped and timestamps are RFC 3339.

## Contents

```text
crates/api/api_missions/src/models/
├── faction.rs             `UserFaction`, one reusable faction of the caller's faction library
├── mission.rs             the library row, its versions and armory lines, enums and override rows
├── mission_deployment.rs  deployment bodies, transitions, and what operators and runtimes read
├── mission_review.rs      a review, the review thread, and the approve, reject and comment bodies
├── mod.rs                 the module tree; re-exports the faction, mission and registry models
└── registry.rs            `RegistryItem` and `RegistryCompatEdge`, one modpack's catalog and its graph
```

## How it works

`MissionStatus` and `WeatherType` map to the Postgres enums of the same names in snake_case
(`mission_status`, `weather_type`), as the `api_mission_vocabulary` crate's `TerrainType` and
`GameMode` (`crates/api/api_mission_vocabulary/src/`) map `terrain_type` and `game_mode`; a mission
moves through `draft`, `pending_approval`, `live`, `rejected` and `archived`. `Mission` names its
current version and, once approved, `approved_artifact_id`, the artifact
[deployments](/documentation/glossary/a_to_f.md#deployment) load. A `MissionVersion` is written once,
unique per mission and semver, and carries its payload as `RawJson`, the bytes Postgres stored,
never re-serialized. A `MissionArmory` line with no quantity is unlimited. No struct carries a
soft-delete column; the queries filter deleted rows. The camelCase documents (the compiled mission
document, the export envelope, the loadout export) live in `services` and `contract`, not here.

Request bodies (`DeploymentRequest`, `RelayedDeploymentRequest`, `ApprovalDecision`,
`RejectionDecision`, `ReviewCommentRequest`) refuse unknown fields. `DeploymentTransition` holds
each transition's wire name and confirmation deadline: `scenario_restart` 600 s, `host_restart`
1200 s.

## Boundaries

- Depends on: `fleet_wire_contract::rfc3339_timestamps` for timestamps and `api_foundation::wire_format::RawJson`; serde and sqlx. The types
  generated from `contracts/definitions/mission-review.schema.json` and
  `contracts/definitions/mission-deployment.schema.json` are in `contract_schema_types::missions`; `faction.rs` and `registry.rs` carry
  the `@contract` tags of `faction-library.schema.json`, `registry-items.schema.json` and
  `registry-compat.schema.json`, and `faction.rs` also cites the faction row of
  `arsenal-envelopes.schema.json` (`UserFaction`, whose `doc` is the faction-library document).
  `mission.rs` cites `mission-review.schema.json` (`MissionRow`, `MissionVersion`),
  `mission-library.schema.json` (`MissionArmory`) and `mission-default-overrides.schema.json`.
- Used by: the domain's handlers and services; `api_operations` (`MissionArmory` in the
  [event](/documentation/glossary/a_to_f.md#event) hub); the [API](/documentation/glossary/a_to_f.md#api) tests
  `apps/api/tests/models_serde.rs`, `apps/api/tests/models_fromrow.rs` and
  `apps/api/tests/mission_review_contract.rs`, which decodes live answers into the
  generated types. The web app's DTOs in `apps/frontend/src/foundation/transport/dto/`
  (`missions.rs`, `mission_reviews.rs`, `mission_deployments.rs`, `registry.rs`) mirror these
  shapes.
- Rules: the generated types are written by `cargo xtask ci schema-codegen` and never edited by
  hand (`cargo xtask ci verify-codegen-fresh` checks them); every `@contract` tag resolves against
  `contracts/definitions/` (`cargo xtask schema citations`); a stored version is never updated,
  and the `mission_versions_are_immutable` trigger of
  `crates/api/api_database/migrations/0052_mission_version_immutability.sql` refuses any update.
