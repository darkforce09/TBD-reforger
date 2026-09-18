# `operations/`

The community's scheduled activity: the event calendar and its dossier, the missions attached to an
event, ORBAT slotting with squad reservation and leader assignment, the member directory a leader
seats from, the roster a running game server reads, a member's own service record and leave
requests, and the persisted mortar fire missions of the field tools.

The ballistics themselves are not here — the charge tables and `solve_fire_mission` live in
`website-map-engine::data::scenario::ballistics`, and the fire-mission handler calls that solver.
This crate persists the solution.

## Public surface

- **`routes::routes()`** — the domain's `/api/v1` table, merged by
  `core::http_router::api_v1_routes` and nested under `/api/v1`. The literals in `routes.rs` are the
  public URLs: `/events`, `/events/{id}`, `/events/{id}/missions`, `/events/{id}/missions/{emid}`,
  `/events/{id}/fire-missions`, `/event-missions/{emid}/orbat`, `/event-missions/{emid}/register`,
  `/event-missions/{emid}/slots/{slotId}/assign`, `/event-missions/{emid}/squads/reserve`,
  `/event-missions/{emid}/squads/release`, `/members`, `/me/deployments`, `/me/leave-requests`,
  `/admin/leave-requests`, `/admin/leave-requests/{id}`, `/fire-missions`, `/fire-missions/solve`,
  `/ingest/events/{id}/roster`.
- **`services::event_status_rules`** — the effective-status SQL, the shared column list, and the
  legal transition table. Every read, guard and sweep in the domain reasons about the same
  derivation.
- **`services::event_lookup`** — the canonical single-row reads (`load_event`, `load_em`).
- **`services::event_lifecycle_sweep::sweep_once`** — the convergence pass the scheduled sweeper
  calls, reachable from a test without a timer.
- **`models::{event, fire_mission, leave_request}`** — the event container and its missions, the
  ORBAT seats and squad reservations, the saved firing solution, and the leave-of-absence record.
- `services/mod.rs` re-exports `OrbatSlotTemplate`, `OrbatSquadTemplate` and `parse_orbat_template`
  from `website_map_engine::data::scenario::orbat`, so every handler that seats an ORBAT names one
  path for the template shapes.

## Dependency rules

- Handlers here never import another domain's handlers; `src/tests/architecture_rules.rs` enforces
  it across all eight domains.
- This domain imports `core`, `missions::{models, services}` (mission lookup, title/terrain, the
  cargo phys catalog), `identity_and_access::services` (the caller's account row),
  `match_telemetry::models` (the match an attended deployment refers to) and
  `administration::{models, services}` (the papertrail).
- Nothing in `core` imports it.

## Files

```text
mod.rs                                 Domain module tree; re-exports `routes`.
routes.rs                              The `/api/v1` route table for operations.
handlers/
  mod.rs                               One module per operations surface.
  event_create_update.rs               Admin writes on the event container: create, patch, soft delete.
  event_listing.rs                     The calendar list with per-event fill decoration, and the dossier read.
  event_mission_attachment.rs          Attaching a mission to an event and detaching it again.
  fire_missions.rs                     The mortar fire-mission calculator and the persisted solutions.
  leave_requests.rs                    Leave of absence: filing, the member's own queue, and admin review.
  member_service_record.rs             The caller's own record: combat figures, upcoming and past deployments.
  orbat_view.rs                        The ORBAT read for one event mission and the member directory behind it.
  roster_ingest.rs                     The identity → slot map a running game server seats players from.
  slot_assignment.rs                   Leader and admin writes on an ORBAT: seat, clear, and squad moves.
  slot_registration.rs                 Self-service registration: claiming a seat and taking the bench.
  tests/
    event_create_update.rs             Sibling unit tests for `event_create_update.rs`.
    event_mission_attachment.rs        Sibling unit tests for `event_mission_attachment.rs`.
    orbat_view.rs                      Sibling unit tests for `orbat_view.rs`.
    roster_ingest.rs                   Sibling unit tests for `roster_ingest.rs`.
services/
  mod.rs                               Event status derivation and the lookups the handlers share.
  event_lifecycle_sweep.rs             The convergence pass over the stored `events.status` column.
  event_lookup.rs                      The canonical single-row reads for an event and an event mission.
  event_status_rules.rs                Derived event status and the legal transitions between stored states.
models/
  mod.rs                               Operations-domain database and wire models.
  event.rs                             The event container, its missions, and the ORBAT seats and squads.
  fire_mission.rs                      The saved mortar firing solution model.
  leave_request.rs                     Leave-of-absence requests and the review state they move through.
```

Unit tests live in the sibling files above, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
