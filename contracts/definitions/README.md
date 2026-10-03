# Contract definitions

The authoritative JSON Schemas of every shape that crosses a network, process or language boundary:
the [API](/documentation/glossary/a_to_f.md#api)'s web and game-runtime payloads, the
[mission](/documentation/glossary/g_to_m.md#mission) document and its editor payload, the item
[registry](/documentation/glossary/n_to_z.md#registry), the loadout and faction documents, the terrain
and world-object data, the building geometry, the
[Workbench](/documentation/glossary/n_to_z.md#workbench) equipment export and the voice bridge. Code
generates types from them, embeds them for runtime validation, and gates fixtures and live responses
against them.

## Contents

```text
contracts/definitions/
├── equipment-data-viewer/  the equipment data viewer's debug API pages, one schema per page
├── equipment-gameplay/     the published equipment gameplay dataset and its receipt
└── *.schema.json           one JSON Schema per contract, named `<subject>.schema.json`
```

## How it works

A schema reaches code in one of three ways:

- **Generated types.** `cargo xtask schema codegen` (the `schema-codegen` CI task) runs typify over
  the schemas listed in `TARGETS` in `tools/commands/schema_tooling/src/generate/schema_types.rs` and
  writes one module directory per schema into
  `crates/contracts/contract_schema_types/src/generated/`, under the module of the API domain that
  serves or reads it. `cargo xtask ci verify-codegen-fresh` re-renders them in memory
  and fails on any missing, changed or stray file; it runs in `ci-local-schema` and in the
  `contracts.yml` workflow. `loadout-export.schema.json` stays out of codegen, because its
  versioned root `oneOf` does not survive typify; its model is hand-written in
  `apps/api/src/missions/contract/loadout_projection.rs`.
- **Embedded validators.** Code embeds a schema with `include_str!` and validates at runtime: the
  API checks the editor payload of `POST /api/v1/missions/{id}/versions`, every compiled
  [artifact](/documentation/glossary/a_to_f.md#artifact), faction documents and registry envelopes
  (`apps/api/src/missions/contract/schema_validators.rs`); the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) embeds `mission.schema.json` for
  its zone vocabulary and `loadout-export.schema.json` for its loadout export; the xtask
  equipment-export commands embed `equipment-vehicle-export.schema.json`. An unknown key in a closed
  object (`additionalProperties: false`) is then a validation error, not a dropped field.
- **Gates and tests.** `cargo xtask schema validate` validates the fixtures of
  `contracts/fixtures/`, the catalogues of `contracts/catalogs/` and the committed Everon
  manifests against these schemas, resolving cross-file `$ref`s through each map-object schema's
  `$id`, and holds the committed ballistics catalog and its calibration bundle to one game build,
  export generation and catalog SHA-256; the developer tools' map verifications read the terrain, label and geometry schemas; and
  the API's contract suites validate live responses, request bodies and the frontend's captured
  API goldens (`apps/api/tests/contract_support/mod.rs`).

`cargo xtask schema citations` (the `verify-citations` CI task) resolves every
`@contract <schema>#<pointer>` tag in `.c`, `.rs` and the other code files under `apps/` and
`tools/` against these files, so a renamed schema or definition fails where code still cites it.

| Contract | Schemas | Read by |
|---|---|---|
| Web API responses | `current-profile`, `reservation-response`, `event-hub`, `event-orbat`, `event-viewer-access`, `event-access-administration`, `waitlist-promotion-response` | generated API models; API contract tests |
| Web API responses without generated types | `service-health`, `session-token`, `profile-update`, `arma-link`, `personnel-actions`, `command-center`, `service-record`, `leave-request`, `event-schedule`, `reservation-actions`, `member-directory`, `fire-mission`, `announcement`, `modpack`, `mission-library`, `mission-default-overrides`, `arsenal-envelopes`, `server-intel`, `runtime-heartbeat-receipt` | the `@contract` tags of the hand-written API models and handlers; the API's route-acceptance and golden parity tests |
| Fleet and machine credentials | `machine-credential`, `fleet-command` | generated API models; the fleet host agent's ledger client; API contract tests |
| Game runtime | `game-runtime-session`, `game-runtime-roster`, `game-runtime-deployment` | generated API models; API contract tests; the [mod](/documentation/glossary/g_to_m.md#mod)'s API bridge, which calls these routes |
| Administration | `personnel-roster`, `audit-log` | generated types in `crates/contracts/contract_schema_types/src/generated/administration/`; API contract tests; the web app's DTOs |
| Community content | `vehicle-database`, `wiki-page`, `content-upload` | generated types in `crates/contracts/contract_schema_types/src/generated/community_content/`; API contract tests; the web app's DTOs |
| Match telemetry | `match-telemetry` | generated types in `crates/contracts/contract_schema_types/src/generated/match_telemetry/match_telemetry/`; the API's ingest decoders and the integration suites; the web app's DTOs; the mod's telemetry reports, queue and event wire |
| Missions | `mission`, `mission-editor-payload`, `mission-review`, `mission-deployment` | API validators and generated models; the mod's mission DTOs; the Mission Creator; the map engine's tests |
| Arsenal and factions | `registry-items`, `registry-compat`, `registry`, `loadout-export`, `faction-library` | API validators, generated and hand-written models; the registry export plugin; the mod's loadout equip path; the Mission Creator's [arsenal](/documentation/glossary/a_to_f.md#arsenal) |
| Terrain | `terrain-manifest`, `terrain-anchors`, `terrain-registry`, `locations`, `height-labels` | the schema gate; the developer tools' map verifications |
| Offline map tile index | `map-tile-index` | `cargo xtask map tile-index`, which writes `/map-assets/<terrain>/tiles/map/index.json`; the offline service worker crate's `MapTileIndex` and the web app's offline pack download |
| World objects | `map-object-enums`, `map-object-prefab`, `map-object-instance`, `map-object-region`, `map-object-roads`, `map-object-resolved`, `map-object-catalog`, `map-object-type-inventory` | the schema gates; the developer tools' world export and golden gates |
| Building geometry | `building-blueprint`, `building-instances`, `prefab-descriptor`, `blas-manifest` | the developer tools' blueprint compiler and map verifications |
| Workbench equipment export | `equipment-vehicle-export` | `cargo xtask mod validate-equipment-vehicle-export` and `publish-equipment-vehicle-export` |
| Voice bridge | `bridge-messages` | the schema gate, over `contracts/fixtures/bridge_samples/` |
| Game ballistics | `ballistics-catalog`, `ballistics-calibration` | the `ballistics_model` catalog types and the `ballistics_calibration` evaluator; the API's ballistics catalog routes; the schema gate, over the committed catalog in `contracts/catalogs/ballistics/` and its calibration bundle in `contracts/fixtures/ballistics/` |

`match-telemetry.schema.json` describes the event read page at its root and carries the
machine-authenticated ingest shapes of `/api/v1/ingest/matches`, `/api/v1/ingest/match-results` and
`/api/v1/ingest/match-events` as definitions, with the refusal body and the telemetry queue
reading. The `RuntimeHeartbeat` definition of `game-runtime-session.schema.json` carries the same
queue reading as its optional `telemetry_queue` block: absent keeps the stored reading, and present
requires all four counters with `backlog` at most `capacity`.

Every JSON success answer of the API has a definition here. The web API schemas outside codegen
are self-contained: the API's contract tests compile one file's `definitions` at a time, so a
shape two files need (a modpack, an announcement, a server status, a mission row) is repeated in
each and says which file it matches, as `event-hub.schema.json` does.
`arsenal-envelopes.schema.json` holds the envelopes around the arsenal data. A faction row's `doc`
is an open object that a consumer validates against the draft 2020-12 `faction-library` on its
own. The registry pages' rows are closed `RegistryItemRow` and `RegistryCompatRow` definitions:
the `registry-items#/$defs/item` or `registry-compat#/$defs/edge` fields with their constraints
repeated, plus the storage columns the API adds (`id`, `modpack_id`, `sort_order` or `qty`, and the
timestamps). `profile-update` and
`runtime-heartbeat-receipt` hold answers of routes whose request or sibling shapes live in a
codegen target, which stays unchanged so its generated module does not go stale.

`audit-log.schema.json` carries the audit stream's `ready` and `reset` event payloads beside the
history page, and each stream row is an `AuditLogEntry`. In `wiki-page.schema.json`, `WikiBlock`
and `WikiInline` are `type`-tagged unions that nest (a callout, quote or list item holds blocks; a
link or emphasis holds inlines), which typify renders as internally tagged enums. The generated
`VehiclePatch` of `vehicle-database.schema.json` reads an absent key and `null` alike as `None`,
so a consumer that must tell "unchanged" from "clear" decodes the body itself.

`map-object-enums.schema.json` is the single source of every closed `kind` and `class` enum: the
other map-object schemas, the prefab classification rules and the glyph keys all draw from it, and
`cargo xtask schema map-object-enums` holds them to it.

## Format

- Encoding: UTF-8 JSON Schema, one contract per file, named `<subject>.schema.json` in lowercase
  hyphenated words. The web API, fleet, game-runtime, match-telemetry, mission-review and
  mission-deployment contracts use draft-07 and carry no `$id`; the others use draft 2020-12 with an `$id`, most under
  `https://schema.tbdevent.eu/`, which the gates use to resolve cross-file references.
- Schema: each file is a JSON Schema whose `title` and `description` state the endpoint or file it
  shapes and the rules that hold there. The API's models keep snake_case keys, the game-runtime
  roster uses camelCase, as its description says, and the mission and map contracts use camelCase.
- Adding a file: write the schema here; for generated types, add it to `TARGETS` in
  `tools/commands/schema_tooling/src/generate/schema_types.rs` and run `cargo xtask schema codegen`; cite
  it with `@contract <file>#<pointer>` where code implements it; then run
  `cargo xtask ci ci-local-schema`.

## Producers and consumers

- Producers: people; a schema change ships with its regenerated types and updated fixtures.
- Consumers:
  - the typify codegen in `tools/commands/schema_tooling/src/generate/`, and the generated modules of
    `crates/contracts/contract_schema_types/`;
  - the API's embedded validators in
    `apps/api/src/missions/contract/schema_validators.rs` and
    `apps/api/src/missions/handlers/mission_default_overrides.rs`, and its contract tests
    under `apps/api/tests/`;
  - the Mission Creator's embeds in
    `apps/frontend/src/workspaces/editor/ui/inspector/zones_panel/zone_schema_vocabulary.rs`
    and `apps/frontend/src/workspaces/editor/arsenal/rules/export_schema_contract.rs`;
  - the xtask schema gates in `tools/commands/schema_tooling/src/schema_checks/` and the
    equipment-export validation in
    `tools/commands/mod_operations/src/equipment_vehicle_export/validation.rs`;
  - the developer tools' map verifications, world export and blueprint compiler in
    `tools/developer_tools/src/`, which find the folder through `contract_definitions_dir` and
    `definition_path` (`tools/foundation/repository_layout/src/contracts.rs`);
  - the map engine's native tests, which embed `mission.schema.json` and
    `mission-editor-payload.schema.json`;
  - the mod's scripts, whose `@contract` tags cite `mission.schema.json`,
    `loadout-export.schema.json` and the two registry schemas, and the fleet host agent in
    `apps/fleet_host_agent/`, whose ledger client follows `fleet-command.schema.json`;
  - the API's release image, which copies the whole folder (`deploy/Dockerfile`).

## Boundaries

- Depends on: nothing; the schemas are the source the code follows.
- Used by: the API, the Mission Creator, the map engine's tests, the mod, the fleet host agent,
  the developer tools and the xtask gates, as listed above; the `schema.yml` and `contracts.yml`
  workflows run on every change under `contracts/`.
- Rules: generated types match their schemas byte for byte (`cargo xtask ci verify-codegen-fresh`);
  every `@contract` citation resolves (`cargo xtask schema citations`); every fixture, catalogue
  and committed manifest validates (`cargo xtask schema validate`); every closed enum lives in
  `map-object-enums.schema.json` (`cargo xtask schema map-object-enums`); a published mission
  artifact is validated once, when it is compiled, and its stored bytes are never reinterpreted
  under a newer schema.

## Related documentation

- [TBD Voice game bridge contract](/documentation/contracts/definitions/bridge_messages.md)
  — the transport, envelope and lifecycle of `bridge-messages.schema.json`.
- [Schema evolution policy](/documentation/contracts/schema_evolution_policy.md) — how a
  schema here may change and what a change must carry with it.
