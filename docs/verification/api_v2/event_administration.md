# Event administration transactions

Event changes lock the event, every attachment in UUID order, and the complete sorted account
union before rechecking session authority. Attachment additionally locks its catalog mission
between event and attachment locks; catalog archive/delete locks that mission and never acquires
an event lock. The catalog guard and attachment validation cannot pass each other concurrently.
Parent locks use NO KEY UPDATE to admit telemetry's foreign-key KEY SHARE checks. Catalog
lifecycle takes its mission lock then the actor account lock and rechecks current session and role.
Removed attachments and attachments under deleted parent events do not block archive/delete;
active attachments keep the applicable guards. Historical service records retain mission names.

Capacity counts distinct allocated participants across active attachments, including unresolved
legacy reservations and assigned occupants. Zero remains uncapped. A lower positive maximum
conflicts until the manager releases enough reservations; the entire PATCH rolls back on conflict.
Quotas and eligibility-aware allocation remain separate required integration work.

Rescheduling shifts active child times by the exact difference between the old and new event time.
Removed attachments retain their historical schedule. Inputs use four-digit UTC years, reject leap
seconds, and normalize sub-microsecond fractions once to PostgreSQL precision before calculating
the interval. The response returns the normalized time. Dependent range errors reject the whole
transaction. Affected attendance caches recompute before commit.

The current effective lifecycle state is materialized through legal automatic edges before changing
schedule inputs. Completed events therefore remain completed whether or not the background sweep
has already run. Postponing a live event into an open/locked state requires an explicit legal status
transition. Schedule completion uses the existing six-hour horizon; it does not finalize game results
or create no-show observations.

Create, attach, restore, update, cancellation, deletion and automatic transitions append required
audit records and publication entries in their business transactions. Cancellation and deletion
release current reservations with explicit reasons while retaining signup authors, timestamps,
attendance, mission/slot definitions and transition history. No action here kicks a connected player.

The executable checks include HTTP capacity boundaries, cross-mission distinct counting, timestamp
range rollback, generated UTC offset sequences, parent-lock barriers for committed allocations,
ban/session revocation and terminal states, injected audit/outbox failures, retry, real foreign-key
lock compatibility, and paired swept/unswept terminal-state edits. Acceptance requires those checks
to pass in the canonical isolated PostgreSQL harness. These bounded checks are not universal proofs.
