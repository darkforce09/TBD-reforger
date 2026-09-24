# Reservation and attendance separation

## Persisted meaning

A registration retains its UUID, author, first signup time and append-only reservation transition
history. Reservation state is registered, waitlisted, withdrawn or legacy_unknown. Attendance is
nullable attended/no_show and does not allocate a place. The compatibility state column projects
attendance when available, otherwise reservation state; clients use reservation_state for actions.
Withdrawal records participant_withdrew and preserves attendance. Mission removal records
mission_removed, preserves the attachment and slot definitions, and hides it from operational views.
Restoring the identical ORBAT preserves those identities and does not reopen released registrations.
Changing the restored schedule locks affected accounts and recomputes their cached aggregates before commit.
A different restore template receives an actionable conflict instead of silently losing fields.

Historical attended/no_show observations cannot identify their previous reservation state. They
retain legacy_state and use legacy_unknown for allocation. Unknown allocations count conservatively
against capacity until explicitly released. The migration records match provenance when exact
finalized event, mission and Arma facts exist; unmatched observations retain explicit legacy fallback.
No past withdrawal time is invented.

## Attendance and correction transaction

The participation key is (registration_id, match_id, arma_id). Current account attribution of the
Arma identity can change without rewriting factual signup authorship. A finalized matching result
supports attended. An accepted correction can remove that support by moving the result to another
event/mission; another supporting match or explicit legacy observation can still support attendance.
The attendance writer never writes reservation_state or slot_id. Pending matches supply no attendance.
Finalized matches reject return to pending. finalized_at records when PostgreSQL first observes a
terminal report, including during historical migration; it does not claim the gameplay finish time.

Attendance rate is 100 times attended observations divided by decided observations, restricted to
past scheduled missions. Undecided signups, including withdrawals without participation, are excluded
from both counts and cannot become no-shows merely because time passes. Empty populations read zero.
Account reads evaluate the same canonical calculation in their PostgreSQL snapshot, so schedule
boundaries do not require a write to display the current rate. The persisted rate is a transaction cache.
Legacy no-show observations remain explicit. New no-shows derive from finalized facts only
(migration 0045, `derived_attendance_state`): a reservation the history records as active when a
non-aborted match for its exact event mission was first finalized, and without participation in
it, is a no-show; see "Derived attendance" in event_eligibility_allocation.md.

Telemetry holds the source-match guard, share locks on the old and new exact attachment of the match
in UUID order, sorted identity locks, then the complete sorted account union, including accounts
referenced by prior participation after unlinking and every registrant of those attachments. It reconciles provenance,
refreshes attendance and aggregates, and commits together. Linking adds provenance using the same exact
event/mission/finalization predicate without transferring historical signup authorship.

Interactive registration, assignment, clearing, withdrawal and mission removal acquire event then
mission NO KEY UPDATE locks, then all relevant account locks before mutating registrations or slots.
The account union includes current occupants and every waiting participant. Removal is rechecked under
parent locks. These locks are compatible with the ordinary foreign-key KEY SHARE checks of telemetry.
Event container capacity, scheduling, deletion and lifecycle writes use compatible ordered locks;
see event_administration.md. Policy changes, quota allocation and promotion use the same order;
see event_eligibility_allocation.md.

Deferred PostgreSQL constraints validate provenance against finalized exact-match facts and validate
cached attendance against participation or legacy fallback. They examine both sides when a result
moves between matches. Database failure rolls back provenance, attendance, facts and aggregate changes.

## Executable evidence and limits

reservation_attendance_migration.rs exercises schema36 to current-head upgrade, the separately
committed enum addition, preserved historical facts, conservative allocation counting, cached aggregate
recalculation and invalid-provenance negative controls. reservation_attendance_transactions.rs exercises
real HTTP finalization, corrections, withdrawal/re-registration, identity transfer, an account-lock
barrier, injected late aggregate failure, and deterministic generated operation sequences. Current
registration responses are checked against the schema, generated types and a frontend fixture.

Frontend action tests distinguish active reservations from retained withdrawal/unknown history;
attendance appears independently. Quota allocation, eligibility promotion, live-slot occupancy and
derived no-show have their own suites (event_eligibility_allocation.md, live_occupancy.md); no test
here establishes revisioned server-scoped telemetry or staging acceptance, which remain required. PostgreSQL durability, trustworthy authenticated game facts, and eventual
service availability are external assumptions; bounded executable tests are not universal proofs.
