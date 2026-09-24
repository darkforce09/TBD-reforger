# Reservation transaction design

This is the implementation design for the event requirements in requirements.json. It records
chosen semantics and the integration work still required; it is not acceptance evidence.

## Accounting and policies

An event quota place belongs to a distinct participant. Reservations in multiple missions of
that event reference the same allocation. Member and guest allocations fall back to open capacity
only after the corresponding pool opens. A zero-capacity pool grants no places. The existing
max_slots=0 convention keeps the event's distinct-participant total uncapped; physical mission
slots and configured finite quotas still constrain reservations.

A missing slot policy inherits its squad policy; a missing squad policy inherits the event policy.
An explicitly empty grant list denies. Grants are alternatives; conditions within each grant are
conjoined. A default event policy requires verified TBD membership. Named accounts, mapped roles,
and groups never bypass current session authority, bans, capacity, opening times, or deployment
conditions. Group provenance identifies a managed roster or a verified partner guild with optional
role requirements. A guild observation failure is uncertainty, not confirmed departure.

## Mutation order

Every reservation mutation uses one shared service and the same ordering:

1. Lock the event row with FOR NO KEY UPDATE, including capacity/policy/lifecycle mutations. Catalog attachment takes the catalog-mission lock
   after its event lock and before attachment/account locks; catalog lifecycle never takes event locks.
2. Lock affected event mission rows in UUID order with FOR NO KEY UPDATE.
3. While holding the parent locks, collect the complete account union: actor, assignees,
   affected occupants, and every candidate promotion may inspect. Lock that union once in
   Discord ID order with FOR NO KEY UPDATE, then revalidate sessions, availability, and relevant
   verified membership snapshots. Promotion cannot lazily acquire additional account locks;
   discovery of an account outside the locked union requires restarting the transaction.
4. Lock/update participant allocations, registration rows, and slots in deterministic key order.
5. Append registration history, required audit, and outbox records before committing.

Parent keys remain immutable in these transactions. Parent-row deletion and referenced-key changes
are prohibited: either can upgrade NO KEY UPDATE to FOR UPDATE and block foreign-key KEY SHARE
locks. Mission removal preserves parent rows and history through a soft-removal state. Gameplay
ingestion and linking lock identity rows before account rows; they must not acquire conflicting
event or mission locks while retaining accounts. Reservation transactions must not acquire identity
locks after account locks. Ordinary parent foreign-key checks remain compatible with NO KEY UPDATE.

Attendance and reservation writers use independent columns and match-backed participation records;
see reservation_attendance.md for the implemented cutover and its checks.
The attendance writers retain account-before-attendance ordering and never request event parent
locks after acquiring accounts. Membership synchronization commits its observation and a durable
reservation-recheck request together; its worker starts a new event-first transaction. This prevents
an account-to-event inversion during Discord demotion or departure.

The refactor covers signup, explicit seat selection, administrative assignment, clearing,
withdrawal, promotion, squad holds, policy edits, capacity edits, mission attachment/removal,
rescheduling, deletion, and lifecycle sweeps together. No legacy mutation path retains an opposite
lock order. Authorization performed before acquiring locks is advisory only; the transaction
rechecks relevant state using the database clock after waiting.

## Attendance and compatibility cutover

The reservation_attendance.md contract describes the implemented separate state and history cutover.
Remaining consumers must use explicit allocation state rather than the compatibility projection.

Retain the legacy observation when migrating historical attended/no_show rows. Their original
allocation state cannot be reconstructed from that value; record unknown provenance explicitly
instead of fabricating a withdrawal or reservation history. Attendance from the updated machine
protocol carries source-match, server, finalization and accepted revision provenance. A scheduled
end time alone cannot create a no-show. Document the attendance obligation and denominator before
changing aggregate computation, and test numerator/denominator membership using the same facts.

Keep compatibility state strings as projections while exposing independent reservation and
attendance fields. The frontend must inspect allocation state: a retained withdrawn registration
is not an active reservation merely because my_state is present. Game roster loading derives from
authorized active slots and live occupancy. It does not consume registration state directly.
Upcoming deployment rows and registration receipts have typed frontend DTOs with independent
reservation and attendance fields. Historical service-history rows still need full typed parity.

## Persisted responsibilities

Separate current reservation status from attendance status. A registration retains its stable
identity and authored signup facts; an append-only transition history records withdrawals,
releases, promotions and repeat signups. Queue entry time plus registration UUID determines
promotion order. A participant allocation records quota kind and acquisition/release facts, with
one active allocation per account per event. Each active mission reservation references its
allocation and one actual slot.

Promotion scans waiting participants in queue order, skips ineligible candidates, and chooses an
eligible available slot deterministically by faction, squad, slot index and UUID. Slot and quota
allocation commit together. Administrative assignment uses the same checks and conflicts with an
occupied reservation. Managers express exceptions through access policy edits.

Restrictive policy changes and confirmed membership loss release only affected reservations,
record the reason, release unused quota allocations, and promote replacements atomically.
Temporary Discord errors preserve reservations. Expired cached authority can block further actions
without pretending that membership departure was confirmed.

Live occupancy has its own server/runtime-session/player/life identity. Releasing a reservation
never kicks the occupant. A replacement may hold the reservation, but deployment refuses an
occupied live slot until the matching life/session ends. Delayed occupancy messages cannot clear
a newer occupant. Finalized participation and explicit revisions derive attendance/no-show without
rewriting reservation history.

## Module and acceptance boundaries

Keep policy models and pure grant evaluation separate from PostgreSQL context loading and
visibility filtering. Split reservation transactions into parent locking, quota allocation, seat
assignment, promotion and eligibility release modules. Thin handlers retain /api/v1 routes.
Production files stay below 500 lines and tests below 1000, with sibling test files only.

Required checks include property-generated grant precedence and quota conservation; real HTTP
visibility projections; barrier-controlled last-seat, assignment/withdrawal, capacity/policy and
membership races; transaction failure injection; finalized attendance correction; live occupancy
fencing; and actual frontend/mod consumers. Schema and rollout changes remain forward-only.

## Implementation state

Migration 0036 adds explicit event, squad and slot policy storage and manager-authored group
provenance. Migrations 0042–0045 add quota pools, participant allocations with the one-claim-per-seat
index and consistency triggers, the durable re-evaluation queue, and derived no-show attendance;
0046–0048 add machine credentials, runtime sessions and live occupancy.
event_eligibility_allocation.md, machine_credentials.md and live_occupancy.md describe the
implemented transactions: policy and group administration, authorized visibility, quota
allocation, seatability of seatless holds, eligibility-aware promotion, eligibility release and its
worker, deployment authorization and derived attendance.

The attendance separation and history-preserving withdrawal/removal are implemented with independent
state projections, forward migrations, typed reservation responses and frontend allocation actions. Event-wide capacity updates, schedule
cascades, cancellation/deletion and lifecycle materialization use the transactions in event_administration.md.
The shared authority and capacity guards in reservation_mutation_guards.md protect assignment,
registration, withdrawal, clearing and squad ownership changes.
The frontend access, waiting-list and credential consumers and the mod runtime-session, roster and
deployment consumers are separate slices; a requirement that names them is discharged only by their
own checks (frontend lane, browser gates, mod compilation), not by backend tests.
