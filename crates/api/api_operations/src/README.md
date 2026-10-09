# Operations domain

The [API](/documentation/glossary/a_to_f.md#api)'s
[operations](/documentation/glossary/n_to_z.md#operations) domain: the calendar of
[events](/documentation/glossary/a_to_f.md#event) and each event's hub, the
[missions](/documentation/glossary/g_to_m.md#mission) attached to an event with their
[ORBAT](/documentation/glossary/n_to_z.md#orbat), event access and reservation pools,
[slot](/documentation/glossary/n_to_z.md#slot) reservations with the waiting list and its promotion, the
member directory leaders seat from, the [game runtime](/documentation/glossary/g_to_m.md#game-runtime)'s
roster and player [deployments](/documentation/glossary/a_to_f.md#deployment), attendance derived from
match results, a member's own [service record](/documentation/glossary/n_to_z.md#service-record) and
leave requests, and the saved mortar fire missions.

## Contents

```text
crates/api/api_operations/src/
├── error.rs    the crate's `Error` (a database failure, converting into `ApiError`) and `Result`
├── handlers/   one HTTP handler module per operations surface
├── lib.rs      the crate root: the module tree; re-exports `routes`, `Error` and `Result`
├── models/     the domain's rows and wire shapes, snake_case on the wire
├── prelude.rs  the event models, the lifecycle sweep and the re-evaluation entry points the workers import
├── routes.rs   the domain's `/api/v1` route table
└── services/   event status, access, reservations, live occupancy and attendance
```

## How it works

A request reaches a handler through `routes.rs`, and the extractor the handler takes sets its
tier: `AuthUser`, `LeaderUser`, `AdminUser`, or `MachineCaller` with the `mod_runtime` executor
kind on `/api/v1/game-runtime/*`, where a runtime acts only for its own server. An event is a
container of missions in sequence; attaching a mission copies its ORBAT into `orbat_slots` rows of
the event mission, which members then reserve.

```text
access      slot, then squad, then event policy ──▶ its grants, then gates no grant overrides
reserve     lock the event scope ──▶ plan (pure) ──▶ seat or place, quota allocation, history
release     free the seat ──▶ promote the earliest eligible waiting member, same transaction
deploy      the game runtime reads the roster ──▶ asks to deploy a player life into a slot
attend      finalized match results ──▶ attended or no-show, derived per reservation
```

Every reservation writer takes one lock order (event, its attachments in UUID order, the sorted
account union, then an authority recheck), and producers outside the domain, such as bans and
membership changes, only queue a re-evaluation that a worker runs event-first. An event's status is
derived inside Postgres from its schedule and the database's clock, so no read waits on the sweep
that stores it. The roster and deployment authorization read the slot bindings a
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) recorded, so the game, the
roster and seat authorization name one [artifact](/documentation/glossary/a_to_f.md#artifact). The
ballistics of the fire missions live in the ballistics crates (`fire_mission_planning` and the
crates under it); this domain re-solves every saved fire mission through them against the stored catalog version it
pins, refuses a client solution that disagrees, and stores the server's solution.

## Public surface

- `routes::routes()`: the table the API's router (`api_server::router`) merges under `/api/v1`, one route
  each:
  - `GET /api/v1/me/deployments`: `AuthUser`; the caller's service record.
  - `GET` and `POST /api/v1/me/leave-requests`: `AuthUser`; the caller's leave requests, file one.
  - `GET /api/v1/admin/leave-requests`: `AdminUser`; every leave request.
  - `PATCH /api/v1/admin/leave-requests/{id}`: `AdminUser`; approve or deny one.
  - `GET` and `POST /api/v1/events`: `AuthUser` to list the events the viewer may see, `AdminUser`
    to create one.
  - `GET`, `PATCH` and `DELETE /api/v1/events/{id}`: `AuthUser` to read the event hub; `AdminUser`
    to update, and to soft-delete with every reservation released.
  - `POST /api/v1/events/{id}/missions`: `AdminUser`; attach a mission and copy its ORBAT.
  - `DELETE /api/v1/events/{id}/missions/{emid}`: `AdminUser`; detach it.
  - `GET /api/v1/events/{id}/access`: `AdminUser`; the event's policies, groups and pools.
  - `GET /api/v1/events/{id}/access/participants`: `AdminUser`; why each participant is or is
    not admitted.
  - `PUT /api/v1/events/{id}/access-policy`: `AdminUser`; replace the event policy.
  - `PUT /api/v1/events/{id}/reservation-quotas`: `AdminUser`; replace the member, guest and open
    pools.
  - `POST /api/v1/events/{id}/groups`: `AdminUser`; create a group.
  - `PATCH` and `DELETE /api/v1/events/{id}/groups/{groupId}`: `AdminUser`; change or delete it.
  - `PUT` and `DELETE /api/v1/events/{id}/groups/{groupId}/members/{discordId}`: `AdminUser`; add
    or remove a roster member.
  - `PUT` and `DELETE /api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy`:
    `AdminUser`; set or remove a squad's policy.
  - `PUT` and `DELETE /api/v1/event-missions/{emid}/slots/{slotId}/access-policy`: `AdminUser`;
    set or remove a slot's policy.
  - `POST /api/v1/event-missions/{emid}/waitlist/promote`: `LeaderUser`; promote the earliest
    eligible waiting members.
  - `GET /api/v1/event-missions/{emid}/orbat`: `AuthUser`; the ORBAT the viewer may see, with
    their own registration.
  - `POST` and `DELETE /api/v1/event-missions/{emid}/register`: `AuthUser`; reserve a seat or a
    place, withdraw.
  - `PUT` and `DELETE /api/v1/event-missions/{emid}/slots/{slotId}/assign`: `LeaderUser`; seat a
    member, clear the seat.
  - `POST /api/v1/event-missions/{emid}/squads/reserve`: `LeaderUser`; hold a squad.
  - `POST /api/v1/event-missions/{emid}/squads/release`: `LeaderUser`; release a held squad.
  - `GET /api/v1/members`: `LeaderUser`; the paged member directory, without banned members.
  - `GET /api/v1/game-runtime/events/{id}/roster`: `mod_runtime` credential; the roster of an event
    bound to its server.
  - `POST /api/v1/game-runtime/sessions/{sessionId}/deployments`: `mod_runtime` credential;
    authorize a player life into a slot (a refusal answers 200 with the decision).
  - `POST /api/v1/game-runtime/sessions/{sessionId}/deployments/{occupancyId}/end`: `mod_runtime`
    credential; end that life.
  - `GET /api/v1/ballistics-catalogs`: public; every stored ballistics catalog version.
  - `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}`: public; one catalog
    document, immutably cached under the ETag of its SHA-256.
  - `POST /api/v1/ballistics-catalogs`: `AdminUser`; upload a catalog with its calibration bundle
    (multipart, own body limit), stored only when the calibration lists no failure.
  - `POST /api/v1/fire-missions`: `AuthUser`; re-solve a save against its pinned ballistics
    catalog, refuse a client solution beyond 1 mil or 0.1 s (422 `solution_mismatch`), and store
    the server's solution with its guns.
  - `GET /api/v1/events/{id}/fire-missions`: `AuthUser` the event admits; an event's fire
    missions, oldest first.
- `services::event_lifecycle_sweep::sweep_once`, run by the event lifecycle sweeper worker.
- `services::event_reservations`: the `eligibility_reevaluation` items the reservation
  re-evaluator worker runs over the queue in `api_member_activity::reevaluation_queue`.
- `models`: `Event`, `EventMission` and `OrbatSlot`, read by the dashboard in `api_command_center`.
- `prelude`: the event models, `sweep_once` and the re-evaluation entry points in one import;
  `Error` and `Result`, a database failure that converts into `ApiError`.

## Boundaries

- Depends on:
  - the API kernel crates: `api_state` for the application state, `api_foundation` for the
    errors, pagination, path parameters and wire formats, `api_http_layer` for the extractors,
    `api_configuration`, `api_database`, `api_failpoints` and `api_identifiers`, with
    `http_url_guard` for the URL guard;
  - the API crates: `api_audit_log` for the audit rows; `api_member_activity` for the user-statistics
    recompute and the reservation re-evaluation queue; `api_caller_identity` for account and identity
    locks, session authorization, cached membership permissions, `UserRole` and
    `MachineCaller`; `api_mission_vocabulary` for `TerrainType` and `GameMode`;
  - `api_identity_and_access` for user lookups and the Discord membership enrollment of partner
    guilds; `api_missions` for mission titles and terrains, `MissionArmory` and the deployment a
    server runs; `api_server_infrastructure` for the open runtime session; `fleet_wire_contract`
    for `ExecutorKind`; `api_match_telemetry::models` for the matches of a service record;
  - `mission_model::orbat` for the ORBAT template and the faction join-key check;
  - the ballistics crates `ballistics_model`, `ballistics_solver`, `fire_mission_planning` and
    `ballistics_calibration` for the ballistics.
- Used by:
  - the API's router (`crates/api/api_server/src/router.rs`), which merges the route table;
  - the `event_lifecycle_sweeper` and `event_reservation_reevaluator` workers in
    `crates/api/api_background_workers/src/`;
  - `api_command_center`, the staging fixtures tool and the API's integration suites,
    through the surface above;
  - over HTTP, the operations pages in `crates/frontend/pages/operations_pages/src/`, the
    [event manager](/documentation/glossary/a_to_f.md#event-manager) in
    `crates/frontend/pages/administration_pages/src/event_manager/`, the mortar calculator in
    `crates/frontend/pages/field_tools_pages/src/mortar/`, and the game runtime's roster loader
    and deployment queues in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`domain_handlers_import_no_foreign_handlers` and
  `every_domain_exports_a_route_table` in `crates/api/api_server/src/tests/architecture_rules.rs`);
  every handler carries its `/// @route` tag (`cargo xtask verify route-tags`);
  the domain's generated contract types (`contract_schema_types::operations`) are written by
  `cargo xtask ci schema-codegen` and never edited by hand (`cargo xtask ci verify-codegen-fresh`
  checks them); every reservation writer takes the one lock
  order of `services/event_reservations/reservation_scope.rs`.

## Related documentation

- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes and the
  layers they share.
- [Event eligibility and allocation](/documentation/crates/api/api_server/verification_evidence/event_eligibility_allocation.md)
  — access, visibility, pools, promotion, re-evaluation and derived attendance.
- [Event administration transactions](/documentation/crates/api/api_server/verification_evidence/event_administration.md)
  — the locks every event change takes.
- [Live slot occupancy](/documentation/crates/api/api_server/verification_evidence/live_occupancy.md)
  — player deployment authorization.
- [Event schedule page](/documentation/crates/frontend/pages/operations_pages/schedule/event_schedule_page.md),
  [Event hub page](/documentation/crates/frontend/pages/operations_pages/event_detail/event_hub_page.md)
  and [Deployments page](/documentation/crates/frontend/pages/operations_pages/deployments/deployments_page.md)
  — the member pages over these routes.
