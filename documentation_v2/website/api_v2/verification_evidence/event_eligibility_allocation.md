# Event eligibility and allocation

This note records the implemented semantics of event access, reservation allocation, waitlist
promotion, eligibility re-evaluation and derived attendance. It describes the code; acceptance
evidence is the command output recorded in progress_checkpoint.md.

## Access policies and groups

- Every event carries an explicit access policy; new events receive `tbd_member` access. A squad
  policy (`event_squad_access_policies`) or slot policy (`orbat_slots.access_policy`) replaces the
  broader one for its seats: slot, then squad, then event. A missing child policy inherits; an
  empty grant list admits nobody. Grants are alternatives; the conditions of one grant are
  conjoined (`operations/services/event_access/evaluation.rs`).
- Conditions: `authenticated`, `tbd_member`, `discord_role`, `event_group`, `named_account`.
  Groups are either a managed roster (every entry records who added it, or the system transition
  that did) or a partner guild whose membership comes only from bot-authenticated Discord
  observations. Creating a partner group enrolls the registrants for observation.
- Mandatory gates are evaluated apart from policy and no grant overrides them: account
  availability (ban or deletion), session, registration status and lock, pool opening time,
  capacity, and deployment conditions (`MandatoryAccessConstraints`).
- Membership evidence has two standards (`event_access/subject_loading.rs`). Current authority
  uses snapshots that are fresh, inside the 48-hour grace period, or under an audited override;
  it decides new actions. Last verified facts use the last `member` observation regardless of age;
  they decide releases, so stale or failed Discord data never evicts. Pending evidence relabels a
  policy denial as `MEMBERSHIP_VERIFICATION_REQUIRED`.
- Administrators manage policies, groups, roster entries and quotas through
  `handlers/event_access_administration.rs` and `handlers/event_group_administration.rs`. Every
  change names `expected_access_revision` (409 `ACCESS_REVISION_CONFLICT` when stale), advances
  the revision, re-evaluates the event's reservations inline with reason `access_policy_changed`,
  and audits in the same transaction. A group a policy still references cannot be deleted (409).

## Visibility

`event_access/visibility.rs` decides per viewer: `Full` when the event policy admits the viewer,
`Partial` when only squad or slot policies do, otherwise `Hidden`. A hidden event or mission
answers exactly like a missing one (404). A partial viewer sees only the admitted missions and
seats, no event briefing, and no occupant outside admitted seats. The event list filters before
paging, so totals count visible events only. Each ORBAT seat carries `viewer_access`
(`eligible` or `restricted`) and its `policy_source`.

## Quotas and allocations

- Each event has member, guest and open pools (`event_reservation_quota_pools`, migration 0042):
  a seat limit (null uncapped, 0 closed) and a UTC opening time. New events get member uncapped
  and open from creation, guest 0 and open 0.
- One active allocation per participant and event (`event_participant_allocations`, migration
  0043) is shared by that participant's reservations in all missions of the event. Members draw
  from the member pool and others from the guest pool; either overflows to the open pool only
  once it has opened. Allocations that existed before the migration are `legacy_unclassified`:
  they count toward the event total and toward no pool. A seat occupant without an allocation
  counts the same way.
- A deferred constraint trigger keeps registrations, seats and allocations consistent; a partial
  unique index allows at most one active reservation per seat.
- A seatless request with room keeps a place without a seat (seatless place hold). A seat claim is
  refused (`SEAT_NEEDED_BY_HOLDER`) when it would leave an existing holder with no eligible free
  seat; seatability is a bipartite matching of holders to free seats in priority order
  (`event_reservations/seat_matching.rs`).
- A pool that has not opened refuses the request with its opening time (`QUOTA_NOT_OPEN`); an
  exhausted event or pool places a seatless request on the waiting list and refuses an explicit
  seat claim (`EVENT_FULL`, `MISSION_FULL`).

## Promotion and re-evaluation

- Promotion takes waiters in `(queue_entered_at, id)` order, skips candidates that are
  ineligible, have no pool room or have no eligible seatability-preserving seat, and gives each
  promoted participant the first free seat in allocation order together with its allocation, in
  the same transaction as the release that freed it. Registration never triggers promotion; a
  release, capacity or quota change, access change, pool opening or the manual leader route does
  (`POST /event-missions/{emid}/waitlist/promote`, 409 `EVENT_FULL` when nothing can be given).
- Confirmed eligibility loss (a bot-verified departure or role change, a ban, a deletion) and
  restrictive policy edits release active reservations with a reason (`eligibility_lost`,
  `account_unavailable`, `access_policy_changed`), clear the seat, keep the registration row, its
  history and signup facts, release unused allocations and promote replacements. Waiting entries
  keep their position and are skipped while ineligible. Membership observations and bans only
  enqueue a durable request (`event_reservation_reevaluations`, migration 0044); the worker runs
  an event-first transaction and decides on the facts it reads after the lock wait.

## Derived attendance

`derived_attendance_state` (migration 0045) is the single rule: participation in a finalized
exact-match result, or a preserved legacy `attended`, is `attended`; otherwise a reservation that
was active, according to `event_registration_history`, when a non-aborted match for its exact
event mission was first finalized is `no_show`; otherwise the preserved legacy value applies.
Scheduled time alone decides nothing. Match results lock the old and new attachment of the match
with `FOR SHARE` before identities and accounts, so every obligated registrant is re-derived in the
same transaction; a late correction re-derives attendance in both directions. An unlinked result
player keeps its result row, which the identity link claims and reconciles. A withdrawal keeps the
registration as a tombstone with `withdrawn_at` and its reason, and frees the seat.

## Lock order

Event → affected attachments in UUID order → the complete sorted account union (registrants in
every state, seat occupants, active allocation holders, the actor and any assignee) →
allocation, registration and slot rows → audit and outbox. Authority is revalidated on the
transaction after the lock waits (`event_reservations/reservation_scope.rs`). Membership and ban
transactions insert queue rows only. Telemetry takes attachment share locks before identities and
accounts and never takes event locks.

## Tests

`tests/event_eligibility_policies.rs`, `tests/event_visibility_projection.rs`,
`tests/reservation_quota_allocations.rs`, `tests/seatless_hold_seatability.rs`,
`tests/waitlist_promotion_transactions.rs`, `tests/eligibility_release_transactions.rs`,
`tests/attendance_no_show_derivation.rs` and `tests/reservation_allocation_migration.rs`, over the
shared fixture `tests/event_eligibility_support/mod.rs`; the pure planners carry property tests in
`services/event_reservations/tests/` and `services/event_access/tests/evaluation.rs`.
