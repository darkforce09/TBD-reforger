# `operations/`

The community's scheduled activity: the event calendar and its hub dossier, the missions attached to
an event, event access (policies, groups, reservation quotas and authorized visibility), ORBAT
slotting with reservation allocations, the waiting list and its promotion, the member directory a
leader seats from, the roster and deployment authorization a running game runtime uses, attendance
derived from finalized match facts, a member's own service record and leave requests, and the
persisted mortar fire missions of the field tools.

The ballistics themselves are not here — the charge tables and `solve_fire_mission` live in
`website-map-engine::data::scenario::ballistics`, and the fire-mission handler calls that solver.
This crate persists the solution.

## Public surface

- **`routes::routes()`** — the domain's `/api/v1` table, merged by
  `core::http_router::api_v1_routes` and nested under `/api/v1`. The literals in `routes.rs` are the
  public URLs: `/events`, `/events/{id}`, `/events/{id}/missions`, `/events/{id}/missions/{emid}`,
  `/events/{id}/access`, `/events/{id}/access/participants`, `/events/{id}/access-policy`,
  `/events/{id}/reservation-quotas`, `/events/{id}/groups`, `/events/{id}/groups/{groupId}`,
  `/events/{id}/groups/{groupId}/members/{discordId}`, `/events/{id}/fire-missions`,
  `/event-missions/{emid}/orbat`, `/event-missions/{emid}/register`,
  `/event-missions/{emid}/slots/{slotId}/assign`, `/event-missions/{emid}/squads/reserve`,
  `/event-missions/{emid}/squads/release`,
  `/event-missions/{emid}/squads/{faction}/{squad}/access-policy`,
  `/event-missions/{emid}/slots/{slotId}/access-policy`, `/event-missions/{emid}/waitlist/promote`,
  `/members`, `/me/deployments`, `/me/leave-requests`, `/admin/leave-requests`,
  `/admin/leave-requests/{id}`, `/fire-missions`, `/fire-missions/solve`,
  `/game-runtime/events/{id}/roster`, `/game-runtime/sessions/{sessionId}/deployments`,
  `/game-runtime/sessions/{sessionId}/deployments/{occupancyId}/end`.
- **`services::event_reservations`** — every reservation writer: the event-scope lock order
  (`reservation_scope`), seat claims, releases, allocations, waitlist promotion, eligibility
  re-evaluation and its durable queue.
- **`services::event_access`** — pure policy evaluation, verified membership facts, effective slot
  policies and viewer visibility.
- **`services::live_slot_occupancy`** — deployment authorization and the end of one player life.
- **`services::participation_attribution`** — match provenance and derived attendance.
- **`services::event_status_rules`**, **`services::event_lookup`**,
  **`services::event_lifecycle_sweep::sweep_once`** — derived status, canonical row reads, and the
  convergence pass the scheduled sweeper calls.
- `services/mod.rs` re-exports `OrbatSlotTemplate`, `OrbatSquadTemplate` and `parse_orbat_template`
  from `website_map_engine::data::scenario::orbat`, so every handler that seats an ORBAT names one
  path for the template shapes.

## Dependency rules

- Handlers here never import another domain's handlers; `src/tests/architecture_rules.rs` enforces
  it across all eight domains.
- This domain imports `core`, `missions::{models, services}` (mission lookup, title/terrain, the
  cargo phys catalog), `identity_and_access::services` (session reauthorization, account and
  identity locks, membership enrollment), `server_infrastructure::{models, services}` (machine
  callers and runtime sessions of the game-runtime routes), `match_telemetry::models` (the match an
  attended deployment refers to) and `administration::{models, services}` (the papertrail).
- Nothing in `core` imports it.

## Files

```text
mod.rs                                 Domain module tree; re-exports `routes`.
routes.rs                              The `/api/v1` route table for operations.
handlers/
  mod.rs                               One module per operations surface.
  event_access_administration.rs       Administrator routes for policies, reservation quotas and eligibility evidence.
  event_create_update.rs               Event writes under current authority and the event-scope lock order.
  event_group_administration.rs        Administrator routes for managed rosters and partner-guild groups.
  event_hub.rs                         The Event Hub dossier for one event, projected for the viewer's access.
  event_listing.rs                     The calendar list, filtered to the events the viewer may see.
  event_mission_attachment.rs          Attaching a mission to an event and detaching it again.
  fire_missions.rs                     The mortar fire-mission calculator and the persisted solutions.
  game_runtime_deployments.rs          Game-runtime deployment authorization and end of one player life.
  game_runtime_roster.rs               The identity → slot map a running game runtime seats players from.
  leave_requests.rs                    Leave of absence: filing, the member's own queue, and admin review.
  member_service_record.rs             The caller's own record: combat figures, upcoming and past deployments.
  orbat_view.rs                        The ORBAT read for one event mission and the member directory behind it.
  slot_assignment.rs                   Leader and admin writes on an ORBAT: seat, clear, and squad holds.
  slot_registration.rs                 Self-service registration: claiming a seat or a place, and withdrawing.
  waitlist_promotion.rs                Leader or administrator request for the deterministic waitlist promotion.
  tests/                               Sibling unit tests for `event_create_update.rs`, `event_mission_attachment.rs`,
                                       `game_runtime_roster.rs` and `orbat_view.rs`.
services/
  mod.rs                               Event status derivation and the lookups the handlers share.
  event_lifecycle_sweep.rs             The convergence pass over the stored `events.status` column.
  event_lifecycle_transition.rs        Materializes observed lifecycle state before scheduling inputs change.
  event_lookup.rs                      The canonical single-row reads for an event and an event mission.
  event_status_rules.rs                Derived event status and the legal transitions between stored states.
  live_slot_occupancy.rs               Deployment authorization and live slot occupancy for game runtimes.
  participation_attribution.rs         Match provenance and attendance derived from finalized facts.
  access_administration/
    mod.rs                             Manager-controlled access: policies, groups, quotas and evidence.
    participant_explanation.rs         Why each participant is, or is not, admitted.
    persistence.rs                     Storage of policies, groups and quotas, and the access view.
  event_access/
    mod.rs                             Pure policy evaluation and the verified-facts projections.
    context.rs                         Event-scoped policy references and verified membership facts.
    evaluation.rs                      Grants, precedence and the mandatory gates no grant overrides.
    slot_eligibility.rs                Effective policy for concrete seats: slot, then squad, then event.
    subject_loading.rs                 Membership facts under current authority and last verified facts.
    visibility.rs                      Which events, attachments and seats a viewer may see.
    tests/evaluation.rs                Sibling unit and property tests for `evaluation.rs`.
  event_reservations/
    mod.rs                             Reservation allocation shared by interactive and background writers.
    claim_refusals.rs                  Why a seat, place or promotion is refused, with stable codes.
    eligibility_reevaluation.rs        Releases on confirmed eligibility loss or restrictive policy edits.
    event_administration.rs            Event-wide changes that serialize with reservations.
    mission_restoration.rs             Restoring a removed attachment without reopening reservations.
    mutation_authority.rs              Authority checks writers apply after taking the scope's locks.
    participant_allocations.rs         One persisted quota allocation per participant and event.
    quota_availability.rs              Pool availability as a viewer sees it.
    quota_selection.rs                 Checked quota accounting and the pool a new place comes from.
    reevaluation_queue.rs              Durable re-evaluation requests with leases and revisions.
    reservation_planning.rs            Pure planning for claims, promotion and eligibility releases.
    reservation_release.rs             Releasing a reservation while keeping its history and tombstone.
    reservation_scope.rs               The single lock order every reservation writer uses.
    scope_snapshot.rs                  One projection of a locked event scope for planning.
    seat_claims.rs                     The claim decision shared by registration and assignment.
    seat_matching.rs                   Seatability of seatless place holders (bipartite matching).
    waitlist_promotion.rs              Deterministic earliest-eligible promotion into actual seats.
    tests/                             Sibling unit and property tests for `quota_availability.rs`,
                                       `quota_selection.rs`, `reservation_planning.rs` and `seat_matching.rs`.
models/
  mod.rs                               Operations-domain database and wire models.
  event.rs                             The event container, its missions, and the ORBAT seats and squads.
  event_access_administration.rs       The manager's access view, change requests and eligibility evidence.
  event_access_policy.rs               Access grants and their conditions.
  event_group.rs                       Managed-roster and partner-guild group sources.
  event_viewer_access.rs               The viewer's visibility, pool class, pool availability and seat access.
  fire_mission.rs                      The saved mortar firing solution model.
  game_runtime_roster.rs               The roster wire the game runtime reads (version 2).
  leave_request.rs                     Leave-of-absence requests and the review state they move through.
  live_occupancy.rs                    Deployment requests, decisions and ended lives.
  participant_allocation.rs            The pool a participant's allocation came from.
  reservation_quota.rs                 Member, guest and open pools with limits and opening times.
  reservation_response.rs              The registration response: reservation and attendance apart.
  generated/                           Types generated from `contracts_v2/definitions` for contract tests.
  tests/event_access_policy.rs         Sibling unit tests for `event_access_policy.rs`.
```

Unit tests live in the sibling files above, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
