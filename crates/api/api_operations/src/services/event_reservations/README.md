# Event reservations

Every writer of [event](/documentation/glossary/a_to_f.md#event) reservations, interactive and
background alike: one lock order, pure planning, the quota allocation each participant holds, seat
claims, releases, waitlist promotion and the re-evaluation of eligibility.

## Contents

```text
crates/api/api_operations/src/services/event_reservations/
├── claim_refusals.rs            why a seat, place or promotion is refused, with a stable `code`
├── eligibility_reevaluation.rs  release reservations whose eligibility is confirmed lost; promote
├── event_administration.rs      event-wide changes that serialize with reservations
├── mission_restoration.rs       restore a removed attachment without reopening its reservations
├── mod.rs                       the module tree
├── mutation_authority.rs        the authority checks a writer applies after taking the scope's locks
├── participant_allocations.rs   the one active quota allocation per participant and event
├── quota_availability.rs        each pool as a viewer sees it: limit, places left, opening time
├── quota_selection.rs           checked quota accounting and the pool a new place comes from
├── reservation_planning.rs      pure plans for claims, promotion and eligibility releases
├── reservation_release.rs       release a reservation, keeping its history and signup facts
├── reservation_scope.rs         `ReservationScope`: the single lock order every writer takes
├── scope_snapshot.rs            one projection of a locked event scope, for planning
├── seat_claims.rs               the claim decision registration and assignment share
├── seat_matching.rs             whether every seatless place holder can still get a seat
├── tests/                       unit tests for pools, planning and seat matching
└── waitlist_promotion.rs        deterministic promotion of waiting participants into seats
```

## How it works

Every writer takes `ReservationScope::lock` first: the event, its attachments in UUID order, then
the complete sorted union of accounts (registrants in every state, seat occupants, allocation
holders, the actor and any assignee), and then rechecks the actor's authority after the waits.
Promotion and re-evaluation lock no further accounts, so no writer inverts that order.

```text
lock the scope ──▶ scope_snapshot ──▶ reservation_planning (no database) ──▶ apply the plan
               ──▶ history rows, audit rows and promotions in the same transaction
```

- A new claim needs current authority for the seat, a quota place, event and
  [mission](/documentation/glossary/g_to_m.md#mission) capacity, and must leave every seatless place
  holder seatable (`seat_matching.rs`, a bipartite matching in priority order); an existing
  reservation keeps its allocation and is never charged twice.
- A participant holds one allocation per event, shared by their reservations in every mission of
  it: members draw on the member pool, others on the guest pool, and either falls back to the open
  pool once it has opened. A pool not yet open refuses with `QUOTA_NOT_OPEN` and its opening time.
- Promotion takes waiting registrations in `(queue_entered_at, id)` order, skips those that are
  ineligible or have no room or seat, and runs in the transaction of the release that freed the
  place; registration never promotes.
- Only confirmed ineligibility releases a reservation (a ban, a deletion, or a policy the last
  verified membership facts fail); stale Discord data never evicts. Membership changes and bans
  only upsert a row in the member activity crate's queue
  (`crates/api/api_member_activity/src/reevaluation_queue.rs`), and the
  `event_reservation_reevaluator` worker
  leases it and re-evaluates the event in its own event-first transaction.
- A release clears the seat and keeps the registration row, its history and signup facts; it
  never ends a live player life in a running game.

## Boundaries

- Depends on: `api_operations::services::event_access` for policies, membership facts and
  eligibility; `api_operations::services::event_status_rules` for the derived event status and the
  statuses that admit registration; `api_operations::services::event_lifecycle_transition` to store the
  derived status once the event is locked; `api_operations::models`; the API crates for account locks and
  session authorization (`api_caller_identity`), the audit rows (`api_audit_log`), and the
  user-statistics recompute and the re-evaluation queue (`api_member_activity`); `api_configuration`,
  `api_foundation` and `api_http_layer` for configuration, errors and `AuthUser`.
- Used by: the [operations](/documentation/glossary/n_to_z.md#operations) handlers for registration,
  assignment, promotion, events, attachments, access administration and the event hub;
  `api_operations::services::access_administration`; the ban handler in
  `crates/api/api_administration/src/handlers/disciplinary.rs` and the membership cache in
  `crates/api/api_identity_and_access/src/services/discord_membership_cache.rs`, which queue
  re-evaluations; the `event_reservation_reevaluator` worker in
  `crates/api/api_background_workers/src/`; the test
  `crates/api/api_server/tests/events_and_reservations/reservation_quota_allocations.rs`.
- Rules: planned promotions respect quota, capacity, eligibility and seatability
  (`planned_promotions_respect_quota_capacity_eligibility_and_seatability`) whatever the input
  order (`promotion_order_is_independent_of_input_permutation`), both in
  `tests/reservation_planning.rs`; allocations and capacity are conserved over any operation
  sequence (`generated_operation_sequences_conserve_allocations_and_capacity` in
  `tests/quota_selection.rs`); the matching agrees with an exhaustive search
  (`seat_matching_agrees_with_exhaustive_assignment` in `tests/seat_matching.rs`); producers of
  re-evaluation requests never take event locks.

## Related documentation

- [Event eligibility and allocation](/documentation/crates/api/api_server/design_notes/event_eligibility_allocation.md)
  — pools, allocations, promotion, re-evaluation and the lock order.
- [Reservation transaction design](/documentation/crates/api/api_server/design_notes/reservation_transaction_design.md)
  and [Reservation mutation authority and capacity](/documentation/crates/api/api_server/design_notes/reservation_mutation_guards.md)
  — the transaction and authority design of the reservation writers.
