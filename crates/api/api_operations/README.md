# API operations

The `api_operations` crate: the [API](/documentation/glossary/a_to_f.md#api)'s
[operations](/documentation/glossary/n_to_z.md#operations) domain. It holds the calendar of
[events](/documentation/glossary/a_to_f.md#event) and each event's hub, the
[missions](/documentation/glossary/g_to_m.md#mission) attached to an event with their
[ORBAT](/documentation/glossary/n_to_z.md#orbat), the event access control and reservation pools,
[slot](/documentation/glossary/n_to_z.md#slot) reservations with the waiting list, the member
directory, the [game runtime](/documentation/glossary/g_to_m.md#game-runtime)'s roster and player
[deployments](/documentation/glossary/a_to_f.md#deployment), the attendance derived from match
results, a member's [service record](/documentation/glossary/n_to_z.md#service-record) and leave
requests, the saved mortar fire missions and the ballistics catalog uploads, with the `/api/v1`
route table the API's router merges.

## Contents

```text
crates/api/api_operations/
├── Cargo.toml  the package: `api_state`, `api_caller_identity`, `api_identity_and_access`, `api_match_telemetry`, `api_missions`, `api_server_infrastructure`, the ballistics crates, sqlx (`postgres`), axum (`multipart`), layout tier 9
└── src/        the route table, the handlers, the access, reservation, authoring, lifecycle, occupancy, fire-mission and catalog services, the models, the error and the prelude
```

## How it works

An event is a container of missions in sequence; attaching a mission copies its ORBAT into the
event mission's seats, which members then reserve. Every reservation writer locks the event, its
attachments in UUID order and the sorted account union, rechecks the caller's authority, plans the
change as a pure function of that locked snapshot and writes it, releasing seats and promoting the
earliest eligible waiting member in the same transaction. Access policies resolve slot, then
squad, then event. A saved fire mission is re-solved through the ballistics crates against the
catalog version it pins, and only the server's solution is stored. The source tree README has the
routes and the detail.

## Getting started

Run from the repository root:

```bash
cargo test -p api_operations
cargo clippy -p api_operations --all-targets -- -D warnings
cargo xtask db test-it --test event_administration_transactions --test event_access_contract --test attendance_no_show_derivation
```

The unit tests cover the access evaluation and its properties, the quota selection, the
reservation planner and its seat matching, the ORBAT and event payload checks, the ballistics
catalog upload decoding; the routes, the locks and the reservation
transactions are proved against Postgres by the API's integration suites.

## Configuration

No feature and no variable of its own. The reservation writers reread the caller's authority
against the guild of the API's `Config`.

## Public surface

- `routes()`: the domain's `/api/v1` route table.
- `handlers`: the event, access administration, group, attachment, ORBAT, registration,
  assignment, waiting list, member search, service record, leave request, game-runtime roster
  and deployment, fire mission and ballistics catalog handlers, each with its `/// @route` tag.
- `services`: the event access evaluation and visibility, the access administration, the event
  authoring, the lifecycle sweep and status rules, the reservations (scope, snapshot, planner,
  claims, releases, promotion, re-evaluation, allocations), the live slot occupancy, the
  fire-mission resolve and store, and the ballistics catalog upload and store.
- `models`: `Event`, `EventMission`, `OrbatSlot`, the access policy, group, quota, allocation,
  viewer access, live occupancy, roster, fire mission, catalog and leave request shapes.
- `Error` and `Result` (a database failure, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_state`, `api_caller_identity`, `api_http_layer`, `api_member_activity`, the
  domains `api_identity_and_access` (user lookups, partner guild enrollment),
  `api_match_telemetry` (the matches of a service record), `api_missions` (mission titles, the
  armory, the deployment in effect) and `api_server_infrastructure` (the open runtime session),
  `api_audit_log`, `api_configuration`, `api_database`, `api_foundation`,
  `api_identifiers`, `api_mission_vocabulary`, `mission_model`, the ballistics crates
  `ballistics_model`, `ballistics_solver`, `fire_mission_planning` and `ballistics_calibration`,
  `content_digest`, `fleet_wire_contract`, `http_url_guard`, sqlx, axum, serde, chrono,
  thiserror, tokio, tracing and uuid. It names no other domain.
- Used by: the API application (`crates/api/api_server`): its router merges `routes`, its background workers
  run the lifecycle sweep and the reservation re-evaluation, the command center domain reads the
  event models, the staging fixtures tool seeds events through the event authoring services, and
  the integration suites reach the services directly. Over HTTP: the operations, event manager
  and mortar calculator pages and the game runtime's roster and deployment queues.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); its route table, its handlers
  and its imports follow the domain graph.

## Related documentation

- [API operations source](/crates/api/api_operations/src/README.md) — the files, the routes, the
  reservation lock order and the fire-mission re-solve.
- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [Event eligibility and allocation](/documentation/crates/api/api_server/design_notes/event_eligibility_allocation.md)
  — access, visibility, pools, promotion, re-evaluation and derived attendance.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
