# Operations models

The rows and wire shapes of the [operations](/documentation_v2/glossary.md#operations) domain: the
[event](/documentation_v2/glossary.md#event) with its
[missions](/documentation_v2/glossary.md#mission), [ORBAT](/documentation_v2/glossary.md#orbat)
seats and registrations, access policies and groups, reservation pools and allocations, the
[game runtime](/documentation_v2/glossary.md#game-runtime)'s roster and player
[deployments](/documentation_v2/glossary.md#deployment), leave requests and saved fire missions.
Keys are snake_case, absent values are skipped and timestamps are RFC 3339.

## Contents

```text
apps/website/api_v2/src/operations/models/
├── event.rs                        the event, its missions, seats, squad holds, registrations
├── event_access_administration.rs  the manager's access changes and views, participant evidence
├── event_access_policy.rs          `EventAccessPolicy`: grants, conditions and their bounds
├── event_group.rs                  an event group: a managed roster or a partner guild
├── event_viewer_access.rs          what the viewer may see and reserve in one event
├── fire_mission.rs                 `FireMission`, a saved mortar firing solution
├── game_runtime_roster.rs          the roster wire: seat assignments and compiled slots
├── generated/                      types generated from the event and game-runtime schemas
├── leave_request.rs                `LeaveRequest` and its review states
├── live_occupancy.rs               player deployment requests, decisions and ended lives
├── mod.rs                          the module tree; re-exports the event, fire and leave models
├── participant_allocation.rs       the pool a participant's allocation came from
├── reservation_quota.rs            the member, guest and open pools: limits, opening times
├── reservation_response.rs         the registration answer, reservation apart from attendance
└── tests/                          unit tests for access policy parsing, bounds and defaults
```

## How it works

`EventStatus` (`scheduled`, `open`, `locked`, `live`, `completed`, `cancelled`) and
`RegistrationState` (`registered`, `waitlisted`, `withdrawn`, `attended`, `no_show`,
`legacy_unknown`) map to the Postgres enums `event_status` and `registration_state`. An event is a
container of missions in sequence, each an `EventMission` with its own start time and its
`OrbatSlot` seats. No struct carries a soft-delete column; the queries filter deleted rows.

An `EventAccessPolicy` is a list of grants, each a list of conditions; a missing squad or
[slot](/documentation_v2/glossary.md#slot) policy inherits, and an empty grant list admits nobody.
`validate` bounds a policy to 32 grants of 1 to 16 conditions each, and every Discord or account id
to 1 to 128 unpadded bytes. A pool limit of zero closes the pool and an explicit `null` leaves it
uncapped. A registration answer reports the reservation and the attendance separately, and a refused
player deployment is a `DeploymentDecision` with its `DeploymentDenial`, not an error.

## Boundaries

- Depends on: `core::wire_format` for timestamps; serde and sqlx. `generated/` follows the
  schemas `event-access-administration`, `event-viewer-access`, `event-hub`, `event-orbat`,
  `waitlist-promotion-response`, `game-runtime-roster`, `game-runtime-deployment` and
  `reservation-response` in `contracts_v2/definitions/`.
- Used by: the domain's handlers and services; the dashboard in `command_center` (`Event`,
  `EventMission`, `OrbatSlot`); the [API](/documentation_v2/glossary.md#api) tests
  `apps/website/api_v2/tests/models_serde.rs`, `apps/website/api_v2/tests/event_access_contract.rs`,
  `apps/website/api_v2/tests/game_runtime_contract.rs` and
  `apps/website/api_v2/tests/reservation_attendance_transactions.rs`, which decode live answers into
  the generated types. The web app's DTOs in `apps/website/frontend/src/v2/core/api/dto/`
  (`events.rs`, `event_access_administration.rs`, `event_viewer_access.rs`) mirror these shapes.
- Rules: `generated/` is written by `cargo xtask ci schema-codegen` and never edited by hand
  (`cargo xtask ci verify-codegen-fresh` checks it); a new event's default policy admits TBD
  members only, and a closed policy is stated explicitly
  (`default_policy_requires_tbd_membership_and_closed_policy_is_explicit` in
  `tests/event_access_policy.rs`).
