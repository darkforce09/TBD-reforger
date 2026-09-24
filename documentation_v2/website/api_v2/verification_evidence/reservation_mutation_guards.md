# Reservation mutation authority and capacity

The six reservation mutation routes reauthorize their actor inside the business transaction:
self registration, withdrawal, assignment, clearing, squad reservation and squad release.
The lock order is event, attachment, then the complete sorted account union. Missing accounts
are rejected during that locking pass. Session revocation, bans and role changes committed
while a request waits are observed before allocation changes. Squad ownership is also read
after the parent lock. Role extractors alone do not authorize the later transaction.

Assignment rejects a different current occupant, unavailable targets, terminal operations,
and unavailable event or mission capacity. The transaction preserves the target's previous
seat on every rejection. Leaders need the current squad hold; only administrators bypass the
registration lock. An identical assignment preserves timestamps and creates no duplicate audit.

Self registration and assignment share one claim decision (seat_claims.rs). Event capacity
counts active participant allocations, one per participant across all active attachments, and a
seat occupant without an allocation counts as an unclassified place; mission capacity counts the
union of active reservations and current occupants, so an orphaned occupied slot still consumes
capacity. Event max_slots=0 remains uncapped. Existing allocations move without consuming an
additional place. A seatless request with room keeps a place without a seat; an explicit claim
that would leave an existing seatless holder without an eligible free seat is refused
(SEAT_NEEDED_BY_HOLDER), and a seatless request without room waits.

Clearing a seat retains the registered participant's place until withdrawal. Withdrawal and
every release that frees a place promote waiting participants in queue-time/UUID order in the
same transaction: each promoted participant receives an actual eligible seat and its quota
allocation together, and ineligible candidates keep their queue position (event_eligibility_allocation.md).

Reservation mutations append required actor audit records and publication entries in the
business transaction. Automatic capacity promotion appends a system audit in that transaction.
Audit insertion failure rolls back seats, registration changes, transition history and outbox.
Repairs of inconsistent occupancy are audited. Repeated withdrawal preserves the original
release reason and transition history, including manager cancellation or removal reasons.

Verification is in tests/reservation_guard_transactions.rs, with fixtures in
 tests/reservation_guard_support/mod.rs. It exercises actual HTTP routes and PostgreSQL locks,
including actor invalidation and squad ownership changes while a request waits, assignment versus
withdrawal in both serialized orders, occupied-seat rejection, capacity and rollback behavior.
These tests establish the listed guards; eligibility, quota and promotion acceptance is covered by
the suites listed in event_eligibility_allocation.md.
