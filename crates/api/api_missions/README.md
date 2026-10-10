# API missions

The `api_missions` crate: the [API](/documentation/glossary/a_to_f.md#api)'s
[missions](/documentation/glossary/g_to_m.md#missions) domain. It holds the library of
[missions](/documentation/glossary/g_to_m.md#mission) and the versions the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) saves, the
[armory](/documentation/glossary/a_to_f.md#armory), the faction library, the item
[registry](/documentation/glossary/n_to_z.md#registry) with its compatibility graph, the export
document, and the path from a saved version to a running server through immutable
[artifacts](/documentation/glossary/a_to_f.md#artifact), reviews,
[approvals](/documentation/glossary/a_to_f.md#approvals) and
[mission deployments](/documentation/glossary/g_to_m.md#mission-deployment), with the `/api/v1`
route table the API's router merges.

## Contents

```text
crates/api/api_missions/
├── Cargo.toml  the package: `api_state`, `api_caller_identity`, `api_server_infrastructure`, `api_community_content`, the mission crates, jsonschema, sqlx (`postgres`), axum, layout tier 8
└── src/        the route table, the handlers, the schema contract, the write-boundary validation, the compile, artifact, review, deployment and registry services, the models, the error and the prelude
```

## How it works

A mission document crosses three boundaries: the Mission Creator, this API and the
[mod](/documentation/glossary/g_to_m.md#mod). Authored input passes the write-boundary predicates
and the embedded JSON Schemas of `contracts/definitions/` before a row is written. Submitting a
mission compiles its current version through the `mission_compiler` crate into an immutable
artifact and opens a review of exactly that artifact; an administrator's decision approves or
rejects that artifact and no other. A deployment of an approved artifact runs as a
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) of the server infrastructure
domain, binds the [event](/documentation/glossary/a_to_f.md#event)'s
[ORBAT](/documentation/glossary/n_to_z.md#orbat) seats to the artifact's compiled slots, and is
confirmed when a runtime session of that server reports the artifact. The source tree README has
the routes and the detail.

## Getting started

Run from the repository root:

```bash
cargo test -p api_missions
cargo clippy -p api_missions --all-targets -- -D warnings
cargo xtask db test-it --test mission_deployment_transitions --test mission_review_binding --test mission_review_contract
```

The unit tests cover the schema validators and the zone quantisation against the compiler, the
loadout projection, the version payload, field and semver validation, the compile diagnostics, the
artifact store and the slot bindings; the routes, the reviews and the
deployments are proved against Postgres by the API's integration suites.

## Configuration

No feature and no variable of its own. The mission-version body cap is the `version_limit`
argument of `routes()`, which the API reads from `MISSION_VERSION_MAX_BODY_BYTES`
([API environment variables](/documentation/crates/api/api_server/environment_variables.md)); the write lock and
the deployment requests reread the caller's authority against the guild of the API's `Config`.

## Public surface

- `routes(version_limit)`: the domain's `/api/v1` route table.
- `handlers`: the registry, faction, mission library and lifecycle, version, review, armory,
  export, approvals, deployment and game-runtime handlers, each with its `/// @route` tag.
- `contract`: the schema validators of the editor payload, the compiled document, the registry
  envelopes and the faction library, the zone quantisation scan and the loadout projection.
- `validation`: the access, field, semver and version payload predicates.
- `services`: the mission lookups, the write lock, the compile, the artifact store, the reviews,
  the deployments (requests, selection, slot bindings, settlement, reads), the cargo catalog and
  the registry import.
- `models`: `Mission`, `MissionVersion`, `MissionArmory`, the review, deployment, faction and
  registry shapes.
- `Error` and `Result` (a database failure, a schema that does not compile or a refused registry
  envelope, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_state`, `api_caller_identity`, `api_http_layer`, `api_server_infrastructure`
  (the fleet command ledger), `api_community_content` (the modpack a registry belongs to),
  `api_audit_log`, `api_configuration`, `api_database`, `api_foundation`,
  `api_identifiers`, `api_mission_vocabulary`, the mission crates `mission_compiler`,
  `mission_model`, `mission_validation` and `mission_wire_safety`,
  `contract_schema_types`, `content_digest`, `fleet_wire_contract`, `http_url_guard`,
  jsonschema, sqlx, axum, serde, chrono, thiserror, tracing and uuid. It names no other domain.
- Used by: the API application (`crates/api/api_server`): its router merges `routes`, its background workers
  run the deployment reconciliation, the operations and command center domains read the mission
  lookups, the armory and the deployment in effect, the `import-item-registry` tool runs the registry
  import, and the integration suites reach the services directly. Over HTTP: the Mission
  Creator, the mission hub, approvals and server control pages, the game runtime and the
  `cargo xtask mod` commands.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); its route table, its handlers
  and its imports follow the domain graph.

## Related documentation

- [API missions source](/crates/api/api_missions/src/README.md) — the files, the routes and the
  path from a saved version to a running server.
- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [Mission artifacts, reviews and deployment](/documentation/crates/api/api_server/design_notes/mission_artifacts.md)
  — the design of artifacts, their reviews and deployments.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
