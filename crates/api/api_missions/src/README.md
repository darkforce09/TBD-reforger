# Missions domain

The [API](/documentation/glossary/a_to_f.md#api)'s [missions](/documentation/glossary/g_to_m.md#missions)
domain: the library of [missions](/documentation/glossary/g_to_m.md#mission) and the versions the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) saves, the
[armory](/documentation/glossary/a_to_f.md#armory), the faction library, the item
[registry](/documentation/glossary/n_to_z.md#registry) with its compatibility graph, the export
document, and the path from a saved version to a running server through immutable
[artifacts](/documentation/glossary/a_to_f.md#artifact), reviews,
[approvals](/documentation/glossary/a_to_f.md#approvals) and
[mission deployments](/documentation/glossary/g_to_m.md#mission-deployment).

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

A request reaches a handler through `routes.rs`, and the extractor the handler takes sets its
tier: `AuthUser`, `MissionMakerUser`, `AdminUser`, or `MachineCaller` with the `mod_runtime`
executor kind on `/api/v1/game-runtime/*`. A mission document crosses three boundaries (the Mission
Creator, this API and the [mod](/documentation/glossary/g_to_m.md#mod)), so this is the only domain
with a `contract/` and a `validation/` folder: authored input passes the `validation/` predicates
and the schemas in `contract/` before a row is written, and the compiled document is checked
against `mission.schema.json` before it is stored.

```text
save      POST /api/v1/missions/{id}/versions ──▶ validated payload ──▶ new current version
submit    POST /api/v1/missions/{id}/submit ──▶ compile the current version into an artifact
            ──▶ open a review of exactly that artifact            (status pending_approval)
decide    POST /api/v1/approvals/{id}/approve or reject ──▶ the artifact under review
            ──▶ live with approved_artifact_id, or rejected with the reason
deploy    POST /api/v1/servers/{id}/deployments ──▶ validate ──▶ one fleet command
            ──▶ confirmed when a runtime session of that server reports the artifact
run       GET /api/v1/game-runtime/deployment, then GET /api/v1/game-runtime/artifacts/{artifactId}
```

Submission writes the artifact, the review, the status and the audit row in one transaction, under
the mission write lock that the metadata patch, the delete and review comments also take. The
compile itself lives in the `mission_compiler` crate; `services/mission_compile.rs` only
adapts a mission row and its payload to it. An approved artifact stays the one
[deployments](/documentation/glossary/a_to_f.md#deployment) load while its author saves later versions.
A deployment runs as a `load_mission` or `restart_with_mission`
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) of `api_server_infrastructure`, and its
[slot](/documentation/glossary/n_to_z.md#slot) bindings pair the
[event](/documentation/glossary/a_to_f.md#event)'s [ORBAT](/documentation/glossary/n_to_z.md#orbat) seats
with the artifact's compiled slots, which the event roster in `api_operations` reads.

## Public surface

- `routes::routes(version_limit)`: the table the API's router (`api_server::router`) merges under `/api/v1`, one route
  each; "author or administrator" means `MissionMakerUser` plus ownership of the mission:
  - `GET /api/v1/registry`: `MissionMakerUser`; one modpack's item catalog, paged on request,
    with a weak `ETag`.
  - `GET /api/v1/registry/compat`: `MissionMakerUser`; that modpack's compatibility edges, or the
    per-character cargo defaults.
  - `GET` and `POST /api/v1/factions`: `MissionMakerUser`; the caller's faction library, a new
    faction.
  - `GET`, `PUT` and `DELETE /api/v1/factions/{id}`: `MissionMakerUser`; one of the caller's
    factions.
  - `GET` and `POST /api/v1/missions`: `AuthUser` to list the library by scope and filters,
    `MissionMakerUser` to create a draft with its first version.
  - `GET`, `PATCH` and `DELETE /api/v1/missions/{id}`: `AuthUser` to read the overview; author or
    administrator to patch the metadata and to soft-delete.
  - `POST /api/v1/missions/{id}/submit`: author or administrator; compile the current version and
    open its review.
  - `GET /api/v1/missions/{id}/reviews`: `AuthUser`, author or administrator; every review with the
    thread.
  - `POST /api/v1/missions/{id}/review-comments`: author or administrator; a comment on the thread.
  - `GET /api/v1/missions/{id}/artifacts/{artifact_id}`: `AuthUser`, author or administrator; an
    artifact's provenance.
  - `GET /api/v1/missions/{id}/artifacts/{artifact_id}/document`: `AuthUser`, author or
    administrator; its exact bytes, with their SHA-256 as the `ETag`.
  - `GET /api/v1/missions/{id}/artifacts/{artifact_id}/workspace`: `AuthUser`, author or
    administrator; the artifact with the version it compiled from, for the read-only workspace.
  - `POST /api/v1/missions/{id}/versions`: author or administrator; save a version, under
    `MISSION_VERSION_MAX_BODY_BYTES` (256 MiB by default) rather than the 1 MiB default.
  - `GET /api/v1/missions/{id}/versions/{vid}`: `AuthUser`; one version.
  - `POST /api/v1/missions/{id}/versions/{vid}/set-current`: author or administrator; make an
    earlier version current.
  - `GET` and `PUT /api/v1/missions/{id}/armory`: `AuthUser` to read, author or administrator to
    replace it whole.
  - `POST` and `DELETE /api/v1/missions/{id}/bookmark`: `AuthUser`; bookmark, remove the bookmark.
  - `GET /api/v1/missions/{id}/export`: `MissionMakerUser`; the export document as `mission.json`.
  - `GET /api/v1/admin/mission-default-overrides`: `AdminUser`; how many missions change each
    zone-rule default of `mission.schema.json`.
  - `GET` and `POST /api/v1/servers/{id}/deployments`: `AdminUser`; the server's deployments, a
    new request (202).
  - `GET /api/v1/servers/{id}/deployments/{deploymentId}`: `AdminUser`; one deployment.
  - `POST /api/v1/servers/{id}/deployments/{deploymentId}/cancel`: `AdminUser`; cancel while its
    command is unclaimed.
  - `GET /api/v1/game-runtime/deployment`: `mod_runtime` credential; the deployment in flight,
    else the latest confirmed one.
  - `POST /api/v1/game-runtime/deployments`: `mod_runtime` credential; an in-game administrator's
    request (202).
  - `GET /api/v1/game-runtime/artifacts/{artifactId}`: `mod_runtime` credential; the bytes of an
    artifact deployed to its server.
  - `GET /api/v1/game-runtime/missions`: `mod_runtime` credential; the live missions its server
    can run.
  - `GET /api/v1/approvals`: `AdminUser`; the missions pending approval, oldest first.
  - `POST /api/v1/approvals/{id}/approve`: `AdminUser`; approve the artifact under review,
    optionally with conditions.
  - `POST /api/v1/approvals/{id}/reject`: `AdminUser`; reject it with a reason.
- `services::mission_lookup`: `mission_title_terrain` and `historical_mission_title_terrain`, the
  mission title and terrain `api_command_center` and `api_operations` read.
- `services::mission_deployments`: `deployment_reads::deployment_in_effect` and
  `deployment_settlement::lock_and_settle` for the event roster in `api_operations`, and
  `deployment_settlement::reconcile_mission_deployments` for the deployment reconciler worker.
- `services::registry_import`: `import_items` and `import_compat`, run by the `import-item-registry`
  binary.
- `models::mission`: `MissionArmory`, read by the event hub in `api_operations`.
- `Error` and `Result` (a database failure, a schema that does not compile or a refused registry
  envelope, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on:
  - the API crates: `api_state` for the application state, `api_foundation` for the handler
    error, pagination and the wire formats, `api_http_layer` for the extractors and the realtime
    hub, `api_configuration`, `api_database`, `api_identifiers`,
    `api_audit_log` for the audit rows, `api_caller_identity` for session authorization,
    account locks, administrator authority and `MachineCaller`, `api_mission_vocabulary` for
    `TerrainType` and `GameMode`;
  - `api_server_infrastructure` for the fleet command ledger, `fleet_wire_contract` for
    `ExecutorKind` and `FleetAction`, and `api_community_content::models` for the modpack a
    registry belongs to;
  - the mission crates `mission_compiler`, `mission_validation`, `mission_model` and
    `mission_wire_safety`, which compile and check mission documents
    (`mission_model::orbat` also holds the ORBAT template a deployment binds);
  - the schemas in `contracts/definitions/`, embedded at compile time.
- Used by:
  - the API's router (`crates/api/api_server/src/router.rs`), which merges the route
    table;
  - the `mission_deployment_reconciler` worker in `crates/api/api_background_workers/src/`
    and the `import-item-registry` binary in `crates/api/api_server/src/bin/`;
  - `api_command_center` and `api_operations`, through the surface above;
  - over HTTP, the Mission Creator in `crates/frontend/workspaces/mission_creator_workspace/src/`, the mission hub
    pages in `crates/frontend/pages/mission_hub_pages/src/`, the
    [event manager](/documentation/glossary/a_to_f.md#event-manager), approvals and
    [server control](/documentation/glossary/n_to_z.md#server-control) pages in
    `crates/frontend/pages/administration_pages/src/`, the
    [game runtime](/documentation/glossary/g_to_m.md#game-runtime) in
    `mod/tbd-framework/Scripts/Game/TBD/`, and the `cargo xtask mod` commands through
    `tools/commands/mod_operations/src/website_api_client/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`domain_handlers_import_no_foreign_handlers` and
  `every_domain_exports_a_route_table` in `crates/api/api_server/src/tests/architecture_rules.rs`);
  every handler carries its `/// @route` tag (`cargo xtask verify route-tags`);
  the domain's generated contract types (`contract_schema_types::missions`) are written by
  `cargo xtask ci schema-codegen` and never edited by hand (`cargo xtask ci verify-codegen-fresh`
  checks them); artifacts and saved
  versions are immutable, which triggers of `crates/api/api_database/migrations/` enforce.

## Related documentation

- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes and the
  layers they share.
- [API decisions](/documentation/crates/api/api_server/decisions.md) — why game servers fetch artifacts over
  HTTPS rather than from staged files.
- [Mission artifacts, reviews and deployment](/documentation/crates/api/api_server/verification_evidence/mission_artifacts.md)
  — the design of artifacts, their reviews and deployments.
- [Mission library page](/documentation/crates/frontend/pages/mission_hub_pages/library/mission_library_page.md),
  [Mission overview page](/documentation/crates/frontend/pages/mission_hub_pages/overview/mission_overview_page.md)
  and [Mission approvals page](/documentation/crates/frontend/pages/administration_pages/approvals/mission_approvals_page.md)
  — the pages over these routes.
