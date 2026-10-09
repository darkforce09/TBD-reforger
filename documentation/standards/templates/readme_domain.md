**Status:** live

# README template: domain or subsystem

**When to use:** a folder with child folders of its own that is none of the more specific kinds: an
API domain, an engine subsystem, a group of pages, a crate's `src/` root. The
[README standard](/documentation/standards/readme_standard.md) defines every rule this template
follows; the domain kind adds Public surface.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there.

````markdown
# <Name of the domain or subsystem, in plain words: no path, no backticks>

<One to three sentences: what the domain or subsystem is responsible for.>

## Contents

```text
<repository path of the folder>/
├── <child folder>/  <what it is for: a lowercase phrase, no closing period>
├── <file>           <what it is for; entries run in name order>
└── mod.rs           <the module tree and what it re-exports>
```

## How it works

<How a request, a frame or a call moves through the children, the main types, and the invariants
that span them. Name each child's part in one clause; the child's own README holds the detail.>

## Public surface

- <module::item>: <what it is, and who outside the folder uses it>

## Boundaries

- Depends on: <the modules, crates, schemas and services the folder uses, read from its imports>
- Used by: <every user outside the folder, found with git grep, and callers over HTTP>
- Rules: <the invariants a change here must keep, and the test or gate that checks each>

## Related documentation

- [<document title>](/documentation/<path to the document>) — <what it covers>
````

## Worked sample

Written from `crates/api/api_missions/src/`. The sample sits in a fenced block, so no gate
reads it as a README; the folder's own README.md is written from the same code and may differ.

````markdown
# Missions domain

The API's [mission](/documentation/glossary/g_to_m.md#mission) domain: the mission library and the
versions the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) saves, the armory,
the faction library and the Virtual Arsenal registries, and the path from a saved version to a
running server through immutable artifacts, reviews, approvals and deployments.

## Contents

```text
crates/api/api_missions/src/
├── contract/    JSON Schema validation of every mission document, and the hand-written loadout projection
├── error.rs     the crate's error type and `Result` alias, converting into `ApiError`
├── handlers/    one HTTP handler module per mission surface
├── lib.rs       the crate root: the module tree; re-exports `routes`, `Error` and `Result`
├── models/      the domain's rows and wire shapes, snake_case on the wire
├── prelude.rs   the deployment, lookup, armory and registry import names other crates import
├── routes.rs    the domain's `/api/v1` route table
├── services/    row reads, the write lock, compile, artifacts, reviews, deployments, registry import
└── validation/  write-boundary predicates: access, scalar fields, semver, version payloads
```

## How it works

A request reaches a handler through `routes.rs`, and the extractor the handler takes sets its tier:
`AuthUser`, `MissionMakerUser`, `AdminUser`, or the `mod_runtime` machine credential on
`/game-runtime/*`. Authored input passes the `validation/` predicates and the JSON Schemas in
`contract/` before a row is written, and the metadata patch, the delete, the submission and review
comments take the mission write lock first.

The Mission Creator saves a version with `POST /api/v1/missions/{id}/versions`, whose body limit is
set for that route alone. Submitting a mission compiles its current version into an immutable
artifact (`services/mission_compile.rs` adapts the compile in the `mission_compiler` crate)
and opens a review of exactly that artifact, in one transaction with the status change and its
audit record. An administrator approves or rejects that artifact from the approval queue. A
deployment selects an approved artifact for a server, is carried out by one fleet command, and
counts as confirmed only when that server's runtime session reports the artifact it loaded; the
game server reads the bytes from `/api/v1/game-runtime/artifacts/{artifactId}`.

## Public surface

- `routes::routes(version_limit)`: the route table the API application's router
  (`crates/api/api_server/src/router.rs`) merges under `/api/v1`:
  `/missions` and `/missions/{id}` with its `armory`, `bookmark`, `export`, `submit`, `reviews`,
  `review-comments`, `artifacts/{artifact_id}` (and its `document` and `workspace`) and `versions`
  children; `/approvals`; `/factions`; `/registry` and `/registry/compat`;
  `/servers/{id}/deployments`; the four `/game-runtime/*` routes; and
  `/admin/mission-default-overrides`.
- `services::mission_lookup`: `mission_title_terrain` and `historical_mission_title_terrain`, the
  mission title and terrain `api_command_center` and `api_operations` read instead of writing
  their own queries.
- `services::mission_deployments`: `deployment_reads::deployment_in_effect` and
  `deployment_settlement::lock_and_settle` for the game-runtime roster in `api_operations`, and
  `deployment_settlement::reconcile_mission_deployments` for the deployment reconciler of
  `api_background_workers`.
- `services::registry_import`: `import_items` and `import_compat`, run by the `import-item-registry`
  binary.
- `models::mission`: `TerrainType`, `GameMode` and `MissionArmory`, which `api_operations` and
  `api_command_center` read.

## Boundaries

- Depends on:
  - the kernel crates: `api_foundation` (error handling, wire formats, HTTP and text helpers),
    `api_http_layer` (the middleware extractors), `api_state` (the application state),
    `api_configuration`, `api_database`, `api_identifiers` and `api_mission_vocabulary`;
  - `api_audit_log` for the audit trail, `api_caller_identity` for session authorization,
    `api_server_infrastructure` for machine credentials and the fleet command ledger, and
    `api_community_content` for the modpack a registry belongs to;
  - the mission crates (`mission_compiler`, `mission_validation`, `mission_wire_safety`,
    `mission_model`), which compile and validate mission documents, and `contract_schema_types`;
  - the schemas in `contracts/definitions/`, embedded at compile time.
- Used by:
  - the API application's router (`crates/api/api_server/src/router.rs`), which merges the route table;
  - the deployment reconciler in `crates/api/api_background_workers/src/` and the
    `import-item-registry` binary in `crates/api/api_server/src/bin/`;
  - `api_command_center` and `api_operations`, through the lookups, deployment services and
    models above;
  - over HTTP, the Mission Creator and the mission hub and approval pages in
    `crates/frontend/shell/frontend_application/`, and the mission loaders of the game server in
    `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`.
- Rules: no other crate imports this crate's handlers, and `routes.rs` exports the one `routes`
  table that `crates/api/api_server/src/router.rs` merges; the domain's generated contract types in `contract_schema_types` are written by
  `cargo xtask ci schema-codegen` and never edited by hand (`cargo xtask ci verify-codegen-fresh`
  checks them), with `contract/loadout_projection.rs` as the one hand-maintained contract model.

## Related documentation

- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes and the
  layers they share.
- [Mission artifacts](/documentation/crates/api/api_server/verification_evidence/mission_artifacts.md)
  — the design of artifacts, their reviews and deployments.
````
