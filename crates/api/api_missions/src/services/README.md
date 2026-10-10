# Missions services

The logic the [missions](/documentation/glossary/g_to_m.md#missions) handlers share and other domains
call: [mission](/documentation/glossary/g_to_m.md#mission) row reads, the author-or-admin write lock,
the compile of a mission version into the mission document, immutable
[artifacts](/documentation/glossary/a_to_f.md#artifact) with their reviews and
[mission deployments](/documentation/glossary/g_to_m.md#mission-deployment), the export document, and
the import of the item [registry](/documentation/glossary/n_to_z.md#registry).

## Contents

```text
crates/api/api_missions/src/services/
├── cargo_catalog.rs       the current modpack's item weights and capacities, for the save check
├── mission_artifacts/     compile a version into an immutable artifact, and read artifacts back
├── mission_compile.rs     the `mission_compiler` compile of a mission row and payload; the findings headers
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
save        cargo_catalog ──▶ the save-time cargo check in handlers::mission_versions
submit      mission_write_lock ──▶ mission_reviews::open_review
              ──▶ mission_artifacts::compile_artifact ──▶ mission_compile ──▶ mission_compiler
approve     mission_reviews::decide_review: the review, its comment, the mission status, the audit row
deploy      mission_deployments: validate, bind seats, issue the fleet command, settle
```

`mission_compile.rs` builds the `mission_compiler` crate's `MissionMeta` from the mission row and
takes the time of day and weather the payload authors before the row's. It returns the compiler's
own document and error types, which callers import from `mission_compiler`, and names the two
headers that carry an artifact's findings beside its bytes. `mission_write_lock.rs` serialises a mission write with changes to the
mission and to the actor's authority: it locks the live mission row, then the actor's account, then
rereads the actor's authority, so a demotion or a change of author during the wait answers 403. The
metadata patch, the delete, submission and review comments take it; saving a version and setting
the current version check authorship without it.
`mission_reviews.rs` supersedes an earlier pending review when it opens one, and a decision that
names another artifact than the one under review answers 409 `REVIEWED_ARTIFACT_CHANGED`.
`registry_import.rs` validates an envelope against its schema before any SQL runs, then upserts
in chunks of 10,000 rows; a re-run of the same envelope changes no row.

## Public surface

- `mission_lookup`: `mission_title_terrain` for the dashboard in `api_command_center` and the
  [service record](/documentation/glossary/n_to_z.md#service-record) in `api_operations`, and
  `historical_mission_title_terrain`, which still reads a deleted mission, for the service record.
- `mission_deployments`: `deployment_reads::deployment_in_effect` and
  `deployment_settlement::lock_and_settle` for the [event](/documentation/glossary/a_to_f.md#event)
  roster in `api_operations`; `deployment_settlement::reconcile_mission_deployments` for the
  [deployment](/documentation/glossary/a_to_f.md#deployment) reconciler worker.
- `registry_import`: `import_items`, `import_compat` and `ImportCounts` for the `import-item-registry`
  binary.

## Boundaries

- Depends on: the domain's `models` and `contract`; `api_configuration` for
  configuration, `api_foundation` for errors and wire formats; the API crates for the audit rows (`api_audit_log`), account locks, session authorization and
  administrator authority (`api_caller_identity`) and `TerrainType` (`api_mission_vocabulary`);
  `api_server_infrastructure` for the [fleet command](/documentation/glossary/a_to_f.md#fleet-command)
  ledger; `mission_compiler`, `mission_validation` and `mission_model` for the compile and its
  findings and the [ORBAT](/documentation/glossary/n_to_z.md#orbat) template a deployment binds,
  `mission_wire_safety` for the cargo catalog and the wire-safety scans.
- Used by:
  - the domain's handlers, for everything above;
  - `api_command_center` and `api_operations`, through the public surface;
  - the `mission_deployment_reconciler` worker in `crates/api/api_background_workers/src/`
    and the `import-item-registry` binary in `crates/api/api_server/src/bin/`;
  - the [API](/documentation/glossary/a_to_f.md#api) tests
    `crates/api/api_server/tests/smoke/registry_compat.rs`, `crates/api/api_server/tests/smoke/models_fromrow.rs`
    and `crates/api/api_server/tests/missions/mission_deployment_transitions.rs`.
- Rules: the compile refuses over-capacity cargo with the same check and wording as the save, so
  a version the save would refuse never becomes an artifact
  (`compile_with_catalog_refuses_over_capacity_like_save` in
  `tests/mission_compile_diagnostics.rs`); the payload's environment wins over the row's
  (`authored_environment_beats_a_stale_mission_row` in `tests/mission_compile_flatten.rs`); an
  import never marks its modpack current, so importing cannot change the modpack the platform
  serves.

## Related documentation

- [Mission artifacts, reviews and deployment](/documentation/crates/api/api_server/design_notes/mission_artifacts.md)
  — the artifact, review and deployment design.
- [Contract catalogs](/contracts/catalogs/README.md) — the registry exports the import reads.
