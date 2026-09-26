# Missions services

The logic the [missions](/documentation_v2/glossary/g_to_m.md#missions) handlers share and other domains
call: [mission](/documentation_v2/glossary/g_to_m.md#mission) row reads, the author-or-admin write lock,
the compile of a mission version into the mission document, immutable
[artifacts](/documentation_v2/glossary/a_to_f.md#artifact) with their reviews and
[mission deployments](/documentation_v2/glossary/g_to_m.md#mission-deployment), the export document, and
the import of the item [registry](/documentation_v2/glossary/n_to_z.md#registry).

## Contents

```text
apps/website/api_v2/src/missions/services/
├── cargo_catalog.rs       the current modpack's item weights and capacities, for the save check
├── mission_artifacts/     compile a version into an immutable artifact, and read artifacts back
├── mission_compile.rs     the map engine's compile of a mission row and payload; the findings headers
├── mission_deployments/   request, validate, bind, settle and read deployments of approved artifacts
├── mission_document.rs    the camelCase export document of a mission and its current version
├── mission_lookup.rs      mission row reads shared by the mission routes, dashboard and service record
├── mission_reviews.rs     open a review on submission, decide exactly its artifact, the review thread
├── mission_write_lock.rs  the row and account lock the patch, delete, submit and review comment take
├── mod.rs                 the module tree
├── registry_import.rs     idempotent upsert of one modpack's registry items and compatibility edges
└── tests/                 unit tests for the compile adapter: flatten, environment and diagnostics
```

## How it works

A mission moves through these services from save to server:

```text
save        cargo_catalog ──▶ the save-time cargo check in missions::handlers::mission_versions
submit      mission_write_lock ──▶ mission_reviews::open_review
              ──▶ mission_artifacts::compile_artifact ──▶ mission_compile ──▶ website_map_engine
approve     mission_reviews::decide_review: the review, its comment, the mission status, the audit row
deploy      mission_deployments: validate, bind seats, issue the fleet command, settle
```

`mission_compile.rs` builds the map engine's `MissionMeta` from the mission row and takes the
time of day and weather the payload authors before the row's. It re-exports the compile's output
and finding types, so callers name one path, and names the two headers that carry an artifact's
findings beside its bytes. `mission_write_lock.rs` serialises a mission write with changes to the
mission and to the actor's authority: it locks the live mission row, then the actor's account, then
rereads the actor's authority, so a demotion or a change of author during the wait answers 403. The
metadata patch, the delete, submission and review comments take it; saving a version and setting
the current version check authorship without it.
`mission_reviews.rs` supersedes an earlier pending review when it opens one, and a decision that
names another artifact than the one under review answers 409 `REVIEWED_ARTIFACT_CHANGED`.
`registry_import.rs` validates an envelope against its schema before any SQL runs, then upserts
in chunks of 10,000 rows; a re-run of the same envelope changes no row.

## Public surface

- `mission_lookup`: `mission_title_terrain` for the dashboard in `command_center` and the
  [service record](/documentation_v2/glossary/n_to_z.md#service-record) in `operations`, and
  `historical_mission_title_terrain`, which still reads a deleted mission, for the service record.
- `mission_deployments`: `deployment_reads::deployment_in_effect` and
  `deployment_settlement::lock_and_settle` for the [event](/documentation_v2/glossary/a_to_f.md#event)
  roster in `operations`; `deployment_settlement::reconcile_mission_deployments` for the
  [deployment](/documentation_v2/glossary/a_to_f.md#deployment) reconciler worker.
- `registry_import`: `import_items`, `import_compat` and `ImportCounts` for the `import-registry`
  binary.

## Boundaries

- Depends on: the domain's `models` and `contract`; `core` for configuration, errors and wire
  formats; `administration` for the audit rows; `identity_and_access` for account locks, session
  authorization and administrator authority; `server_infrastructure` for the
  [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) ledger; `operations::services` for
  the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) template a deployment binds;
  `website_map_engine::data::scenario` for the compile, the cargo catalog and the wire-safety scans.
- Used by:
  - the domain's handlers, for everything above;
  - `command_center` and `operations`, through the public surface;
  - the `mission_deployment_reconciler` worker in `apps/website/api_v2/src/background_workers/`
    and the `import-registry` binary in `apps/website/api_v2/src/bin/`;
  - the [API](/documentation_v2/glossary/a_to_f.md#api) tests
    `apps/website/api_v2/tests/registry_compat.rs`, `apps/website/api_v2/tests/models_fromrow.rs`
    and `apps/website/api_v2/tests/mission_deployment_transitions.rs`.
- Rules: the compile refuses over-capacity cargo with the same check and wording as the save, so
  a version the save would refuse never becomes an artifact
  (`compile_with_catalog_refuses_over_capacity_like_save` in
  `tests/mission_compile_diagnostics.rs`); the payload's environment wins over the row's
  (`authored_environment_beats_a_stale_mission_row` in `tests/mission_compile_flatten.rs`); an
  import never marks its modpack current, so importing cannot change the modpack the platform
  serves.

## Related documentation

- [Mission artifacts, reviews and deployment](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
  — the artifact, review and deployment design.
- [Contract catalogs](/contracts_v2/catalogs/README.md) — the registry exports the import reads.
