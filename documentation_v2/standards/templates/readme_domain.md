**Status:** live

# README template: domain or subsystem

**When to use:** a folder with child folders of its own that is none of the more specific kinds: an
API domain, an engine subsystem, a group of pages, a crate's `src/` root. The
[README standard](/documentation_v2/standards/readme_standard.md) defines every rule this template
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

- [<document title>](/documentation_v2/<path to the document>) — <what it covers>
````

## Worked sample

Written from `apps/website/api_v2/src/missions/`. The sample sits in a fenced block, so no gate
reads it as a README; the folder's own README.md is written from the same code and may differ.

````markdown
# Missions domain

The API's [mission](/documentation_v2/glossary.md#mission) domain: the mission library and the
versions the [Mission Creator](/documentation_v2/glossary.md#mission-creator) saves, the armory,
the faction library and the Virtual Arsenal registries, and the path from a saved version to a
running server through immutable artifacts, reviews, approvals and deployments.

## Contents

```text
apps/website/api_v2/src/missions/
├── contract/     JSON Schema validation of every mission document, and the generated contract types
├── handlers/     one HTTP handler module per mission surface
├── mod.rs        the module tree; re-exports `routes`
├── models/       the domain's database and wire models, snake_case on the wire
├── routes.rs     the domain's `/api/v1` route table
├── services/     logic other surfaces share: lookups, compile, artifacts, reviews, deployments
└── validation/   write-boundary predicates: access, scalar fields, semver, version payloads
```

## How it works

A request reaches a handler through `routes.rs`, and the extractor the handler takes sets its tier:
`AuthUser`, `MissionMakerUser`, `AdminUser`, or the `mod_runtime` machine credential on
`/game-runtime/*`. Authored input passes the `validation/` predicates and the JSON Schemas in
`contract/` before a row is written, and the metadata patch, the delete, the submission and review
comments take the mission write lock first.

The Mission Creator saves a version with `POST /api/v1/missions/{id}/versions`, whose body limit is
set for that route alone. Submitting a mission compiles its current version into an immutable
artifact (`services/mission_compile.rs` adapts the compile in `website_map_engine::data::scenario`)
and opens a review of exactly that artifact, in one transaction with the status change and its
audit record. An administrator approves or rejects that artifact from the approval queue. A
deployment selects an approved artifact for a server, is carried out by one fleet command, and
counts as confirmed only when that server's runtime session reports the artifact it loaded; the
game server reads the bytes from `/api/v1/game-runtime/artifacts/{artifactId}`.

## Public surface

- `routes::routes(version_limit)`: the route table `core::http_router` merges under `/api/v1`:
  `/missions` and `/missions/{id}` with its `armory`, `bookmark`, `export`, `submit`, `reviews`,
  `review-comments`, `artifacts/{artifact_id}` (and its `document` and `workspace`) and `versions`
  children; `/approvals`; `/factions`; `/registry` and `/registry/compat`;
  `/servers/{id}/deployments`; the four `/game-runtime/*` routes; and
  `/admin/mission-default-overrides`.
- `services::mission_lookup`: `mission_title_terrain` and `historical_mission_title_terrain`, the
  mission title and terrain `command_center` and `operations` read instead of writing their own
  queries.
- `services::mission_deployments`: `deployment_reads::deployment_in_effect` and
  `deployment_settlement::lock_and_settle` for the game-runtime roster in `operations`, and
  `deployment_settlement::reconcile_mission_deployments` for the deployment reconciler worker.
- `services::registry_import`: `import_items` and `import_compat`, run by the `import-registry`
  binary.
- `models::mission`: `TerrainType`, `GameMode` and `MissionArmory`, which `operations`,
  `match_telemetry` and `server_infrastructure` read.

## Boundaries

- Depends on:
  - `core`: error handling, the middleware extractors, the application state, the wire formats,
    and the HTTP, text, configuration and database helpers;
  - `administration` services for the audit trail, `identity_and_access` services for session
    authorization, account authority and account locks, `server_infrastructure` for machine
    credentials and the fleet command ledger, `operations` services for the ORBAT templates slot
    bindings read, and `community_content` models for the modpack a registry belongs to;
  - `website_map_engine::data::scenario`, which compiles and validates mission documents;
  - the schemas in `contracts_v2/definitions/`, embedded at compile time.
- Used by:
  - `core::http_router`, which merges the route table;
  - the deployment reconciler in `apps/website/api_v2/src/background_workers/` and the
    `import-registry` binary in `apps/website/api_v2/src/bin/`;
  - `command_center`, `operations`, `match_telemetry` and `server_infrastructure`, through the
    lookups, deployment services and models above;
  - over HTTP, the Mission Creator and the mission hub and approval pages in
    `apps/website/frontend/`, and the mission loaders of the game server in
    `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the `routes`
  table that `core::http_router` merges (`apps/website/api_v2/src/tests/architecture_rules.rs`
  checks both); `contract/generated/` and `models/generated/` are written by
  `cargo xtask ci schema-codegen` and never edited by hand (`cargo xtask ci verify-codegen-fresh`
  checks them), with `contract/loadout_projection.rs` as the one hand-maintained contract model.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes and the
  layers they share.
- [Mission artifacts](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
  — the design of artifacts, their reviews and deployments.
````
