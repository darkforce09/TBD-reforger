# Missions domain

The [API](/documentation_v2/glossary.md#api)'s [missions](/documentation_v2/glossary.md#missions)
domain: the library of [missions](/documentation_v2/glossary.md#mission) and the versions the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) saves, the
[armory](/documentation_v2/glossary.md#armory), the faction library, the item
[registry](/documentation_v2/glossary.md#registry) with its compatibility graph, the export
document, and the path from a saved version to a running server through immutable
[artifacts](/documentation_v2/glossary.md#artifact), reviews,
[approvals](/documentation_v2/glossary.md#approvals) and
[mission deployments](/documentation_v2/glossary.md#mission-deployment).

## Contents

```text
apps/website/api_v2/src/missions/
├── contract/    JSON Schema validation of every mission document, and the generated contract types
├── handlers/    one HTTP handler module per mission surface
├── mod.rs       the module tree; re-exports `routes`
├── models/      the domain's rows and wire shapes, snake_case on the wire
├── routes.rs    the domain's `/api/v1` route table
├── services/    row reads, the write lock, compile, artifacts, reviews, deployments, registry import
└── validation/  write-boundary predicates: access, scalar fields, semver, version payloads
```

## How it works

A request reaches a handler through `routes.rs`, and the extractor the handler takes sets its
tier: `AuthUser`, `MissionMakerUser`, `AdminUser`, or `MachineCaller` with the `mod_runtime`
executor kind on `/api/v1/game-runtime/*`. A mission document crosses three boundaries (the Mission
Creator, this API and the [mod](/documentation_v2/glossary.md#mod)), so this is the only domain
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
compile itself lives in `website_map_engine::data::scenario`; `services/mission_compile.rs` only
adapts a mission row and its payload to it. An approved artifact stays the one
[deployments](/documentation_v2/glossary.md#deployment) load while its author saves later versions.
A deployment runs as a `load_mission` or `restart_with_mission`
[fleet command](/documentation_v2/glossary.md#fleet-command) of `server_infrastructure`, and its
[slot](/documentation_v2/glossary.md#slot) bindings pair the
[event](/documentation_v2/glossary.md#event)'s [ORBAT](/documentation_v2/glossary.md#orbat) seats
with the artifact's compiled slots, which the event roster in `operations` reads.

## Public surface

- `routes::routes(version_limit)`: the table `core::http_router` merges under `/api/v1`, one route
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
  mission title and terrain `command_center` and `operations` read.
- `services::mission_deployments`: `deployment_reads::deployment_in_effect` and
  `deployment_settlement::lock_and_settle` for the event roster in `operations`, and
  `deployment_settlement::reconcile_mission_deployments` for the deployment reconciler worker.
- `services::registry_import`: `import_items` and `import_compat`, run by the `import-registry`
  binary.
- `models::mission`: `TerrainType`, read by `match_telemetry`, `operations` and
  `server_infrastructure`; `GameMode` and `MissionArmory`, read by the event hub in `operations`.

## Boundaries

- Depends on:
  - `core`: the application state, errors, extractors, `role_rank`, pagination, configuration and
    the wire formats;
  - `administration` for the audit rows, `identity_and_access` for session authorization, account
    locks and administrator authority, `server_infrastructure` for `MachineCaller`, `ExecutorKind`,
    `FleetAction` and the fleet command ledger, `operations::services` for the ORBAT template a
    deployment binds, and `community_content::models` for the modpack a registry belongs to;
  - `website_map_engine::data::scenario`, which compiles and checks mission documents;
  - the schemas in `contracts_v2/definitions/`, embedded at compile time.
- Used by:
  - `core::http_router`, which merges the route table;
  - the `mission_deployment_reconciler` worker in `apps/website/api_v2/src/background_workers/`
    and the `import-registry` binary in `apps/website/api_v2/src/bin/`;
  - `command_center`, `operations`, `match_telemetry` and `server_infrastructure`, through the
    surface above;
  - over HTTP, the Mission Creator in `apps/website/frontend/src/v2/apps/editor/`, the mission hub
    pages in `apps/website/frontend/src/v2/pages/mission_hub/`, the
    [event manager](/documentation_v2/glossary.md#event-manager), approvals and
    [server control](/documentation_v2/glossary.md#server-control) pages in
    `apps/website/frontend/src/v2/pages/administration/`, the
    [game runtime](/documentation_v2/glossary.md#game-runtime) in
    `apps/mod/tbd-framework/Scripts/Game/TBD/`, and the `cargo xtask mod` commands through
    `tools_v2/xtask/src/commands/mod_ops/website_api_client/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`domain_handlers_import_no_foreign_handlers` and
  `every_domain_exports_a_route_table` in `apps/website/api_v2/src/tests/architecture_rules.rs`);
  every handler carries its `/// @route` tag (`cargo xtask verify route-tags`);
  `contract/generated/` and `models/generated/` are written by `cargo xtask ci schema-codegen` and
  never edited by hand (`cargo xtask ci verify-codegen-fresh` checks them); artifacts and saved
  versions are immutable, which triggers of `apps/website/api_v2/migrations/` enforce.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes and the
  layers they share.
- [Mission artifacts, reviews and deployment](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
  — the design of artifacts, their reviews and deployments.
- [Mission library page](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md),
  [Mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md)
  and [Mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md)
  — the pages over these routes.
