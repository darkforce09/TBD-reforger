# Operations services

The logic the [operations](/documentation_v2/glossary.md#operations) handlers share and other
domains call: the derived status of an [event](/documentation_v2/glossary.md#event) and the sweep
that stores it, access evaluation and administration, every reservation writer, the live
occupancy of [slots](/documentation_v2/glossary.md#slot) by player lives in a running game, and the
attendance derived from match results.

## Contents

```text
apps/website/api_v2/src/operations/services/
├── access_administration/         stored access settings and the evidence explaining them
├── event_access/                  policy evaluation, membership facts and visibility
├── event_lifecycle_sweep.rs       stores the derived event status and audits each move
├── event_lifecycle_transition.rs  stores an event's status before its schedule changes
├── event_lookup.rs                the event and event-mission reads a handler starts from
├── event_reservations/            every reservation writer: locks, planning, claims, promotion
├── event_status_rules.rs          the derived event status and the operator's transitions
├── live_slot_occupancy.rs         authorize a player life into a slot, and end one
├── mod.rs                         the module tree; re-exports the map engine's ORBAT template
└── participation_attribution.rs   match provenance and the attendance derived from results
```

## How it works

An event's status is derived, not stored: `EFFECTIVE_STATUS_SQL` computes it inside Postgres from
the stored status, the start times of the event and its
[missions](/documentation_v2/glossary.md#mission), and `statement_timestamp()`, so every read and
the registration guard see the current answer whether or not a background task has run. A pre-start
event becomes `live` at its start time and `completed` six hours after the latest start among the
event and its missions; `cancelled`, `open` and `locked` are an operator's to set, and the
derivation only moves forward.
The `event_lifecycle_sweeper` worker runs `sweep_once` to store the derived status and audit each
automatic move, and `event_lifecycle_transition.rs` stores it before a schedule change could alter
the derivation.

`live_slot_occupancy.rs` decides a [game runtime](/documentation_v2/glossary.md#game-runtime)'s
request to deploy a player into a slot: allowed when the player's own active reservation is for that
slot, or when the slot is unreserved and its effective policy admits the player; the identity must
be linked, the account available and the slot free of any other open life. An allowed decision is
recorded under the runtime's life id, so a retry gets it unchanged, and releasing a reservation
never ends a life. Its lock order is event and attachment (share), identity, account, slot, runtime
session (share), the order the reservation writers in `event_reservations/` also follow.

`participation_attribution.rs` derives attendance from finalized match results: a reservation
active when its exact event mission's match was finalized is a no-show unless its player took
part. Match ingest share-locks the attachments before any identity or account lock.

## Public surface

- `event_lifecycle_sweep::sweep_once` for the `event_lifecycle_sweeper` worker.
- `event_reservations`: `reevaluation_queue::request_reevaluation_for_account` for the bans in
  `administration` and the membership cache in `identity_and_access`; the queue lease and
  `eligibility_reevaluation::reevaluate_event_reservations` for the `event_reservation_reevaluator`
  worker.
- `participation_attribution`: `prior_match_accounts`, `reconcile_match` and
  `lock_obligated_registrants` for match results in `match_telemetry`, and `refresh_attendance`
  for identity linking in `identity_and_access`.
- `parse_orbat_template`, `OrbatSquadTemplate` and `OrbatSlotTemplate`, re-exported from
  `website_map_engine::data::scenario::orbat`, for the
  [deployment](/documentation_v2/glossary.md#deployment) slot bindings in `missions`.

## Boundaries

- Depends on: `operations::models`; `core` for configuration, errors and `AuthUser`;
  `administration` for the audit rows; `command_center` for the user-statistics recompute
  (`services::user_stats::recompute_user_stats_on_connection`); `identity_and_access` for account
  locks, session authorization and cached membership permissions; `server_infrastructure` for the
  machine caller and the shared runtime session; `website_map_engine::data::scenario::orbat`.
- Used by:
  - the domain's handlers;
  - `administration`, `identity_and_access`, `match_telemetry` and `missions`, through the surface
    above;
  - the `event_lifecycle_sweeper` and `event_reservation_reevaluator` workers in
    `apps/website/api_v2/src/background_workers/`;
  - the [API](/documentation_v2/glossary.md#api) tests
    `apps/website/api_v2/tests/attendance_no_show_derivation.rs`,
    `apps/website/api_v2/tests/event_access_context.rs`,
    `apps/website/api_v2/tests/event_administration_transactions.rs`,
    `apps/website/api_v2/tests/event_lifecycle_transactions.rs` and
    `apps/website/api_v2/tests/reservation_quota_allocations.rs`.
- Rules: reads and guards use the derived status, never the stored column, and every time
  comparison uses the database's clock; one lock order holds for every writer of reservations and
  live occupancy; producers outside the domain only queue re-evaluation requests and never take
  event locks.

## Related documentation

- [Event eligibility and allocation](/documentation_v2/website/api_v2/verification_evidence/event_eligibility_allocation.md)
  — access, pools, promotion, re-evaluation and derived attendance.
- [Live slot occupancy](/documentation_v2/website/api_v2/verification_evidence/live_occupancy.md)
  — deployment authorization and ended lives.
- [Reservation and attendance separation](/documentation_v2/website/api_v2/verification_evidence/reservation_attendance.md)
  — how a reservation and its attendance stay apart.
