# API v2 completion and executable verification

## Acceptance contract

Complete all eight API domains, the supplied findings, the API backlog including every
T-940 acceptance condition, and the frontend, mod and deployment integrations needed for
those behaviors. The machine-readable acceptance register is `requirements.json` in this
directory. Every requirement stays required until executable evidence establishes it.

Evidence has three categories: Rust implementation tests, Rust property tests, and measured
operational checks. None establishes universal correctness of external services or hardware.
Requirement coverage means every recorded requirement has current acceptance evidence.

The milestones after E, F and M (telemetry, administration and content, game ballistics,
verification completeness and staging) are listed with their exact checks in
`remaining_milestones.md`.
This program adds no TLA+, TLAPS, TLC, formal specifications, or formal-proof toolchain.

Use `cargo xtask verify api-readiness --execute` to execute registered local checks and
evaluate all required receipts. Without `--execute`, the command evaluates existing receipts.
Missing, stale, empty, skipped, failed or unavailable evidence prevents readiness success.
External staging acceptance requires structured observations, source/configuration identity,
tool versions, complete output and recorded operating conditions.

## Identity and Discord resilience

Authenticated Guest is distinct from anonymous access. Confirmed nonmembers may sign in and
use guest features and explicitly permitted events. Website privileges derive from TBD
Discord roles; partner roles supply event eligibility only. Local bans always apply.

Store the last verified membership state and verification time in PostgreSQL alongside
guild-scoped `user_discord_roles`. Distinguish a verified member with no roles, confirmed
nonmembership, and unavailable/unverified state. A timeout, 429 or transient error must not
erase a verified snapshot or become a departure event.

Use bot-authenticated REST reconciliation through
`GET /guilds/{guild_id}/members/{user_id}`. Coordinate due work through PostgreSQL leases,
with a lease token/revision fencing stale responses. Multiple API instances share work
without duplicate periodic fetches. Never hold a database transaction during network I/O.
Honor Discord Retry-After and rate-limit buckets with bounded backoff and jitter.

OAuth completion, session refresh and supported authenticated webhook notifications enqueue
coalesced priority refreshes. Webhook hints never directly grant roles and only supported,
verified event sources are accepted. The Axum process contains no persistent Discord Gateway
WebSocket client. REST remains the authoritative reconciliation path.

Continue authorizing from the last verified snapshot during a **48-hour grace period**.
Expose verification age and a non-blocking staleness banner; refresh failures do not log
people out or prevent normal cached-permission access. After grace, degrade to authenticated
Guest permissions rather than denying authentication or taking the website offline.
Unknown membership starts as Guest; never fabricate verified member privileges.

Provide an audited administrative grace override with a required reason and expiry, extending
an existing cached snapshot for at most 48 hours per action. Previously verified, non-banned
administrators retain access to the override operation even when their snapshot is stale.
An override does not invent roles, bypass bans, or undo confirmed departure. Record actor,
target, old/new expiry and reason in the same transaction. Display override provenance.

Target propagation within 60 seconds under the recorded healthy-service/rate-budget
conditions. Rate limits or outages preserve cached access rather than enforcing a global
lockout. Confirmed departure becomes Guest. Temporary staleness does not evict reservations.

Validate JWT algorithm, issuer, audience, expiry and persisted session identity. Authorization
uses current account/session state so bans, deletion and logout affect existing access tokens.
Refresh rotation, replay detection and successor issuance serialize in one transaction.
Logout failures are visible. Long-lived streams periodically revalidate authority. Resolve
equal-priority Discord mappings deterministically.

## Events, reservations and identity attribution

Event managers configure alternative grants based on TBD membership, applicable roles,
event groups or named accounts. Each grant's conditions all apply. A group uses either a
manager-maintained roster or verified partner-guild membership/roles, with visible provenance.

An explicit slot policy overrides the squad policy; an explicit squad policy overrides the
event policy. Missing policy inherits; an explicitly empty grant list denies. Mandatory bans,
session validity, capacity, opening times and deployment conditions cannot be overridden.
Users permitted by a child policy can discover the event without receiving unauthorized data.
New events default to TBD-member access; broader participation is configured explicitly.

Member and guest quota pools each fall back to open capacity only after it opens. Persist
the consumed quota and use UTC opening times. Enforce one reservation per user per event
mission, unique slot ownership and the event's distinct-participant capacity, retaining the
documented uncapped setting. Reject capacity reductions below current allocation.

All assignment, signup, withdrawal, promotion, policy and capacity operations use one lock
order. Assignment cannot overwrite another occupant. Promote the earliest eligible waiting
participant deterministically, allocating an actual slot and quota in the transaction.
Confirmed eligibility loss releases the reservation with a reason and re-evaluates waiters.
Preserve historical registrations and attendance.

Do not automatically kick live players. Their current life may finish; further deployment
requires eligibility. Track live occupancy separately so a promoted participant cannot spawn
into an occupied live slot. Derive no-show from finalized participation facts and allow valid
late corrections. Rescheduling moves dependent mission times atomically. Deleted missions
disappear from selection while historical facts remain interpretable.

Keep the existing mod account-link command. Enforce one pending code per account, single
consumption and unique Arma-identity ownership using consistent transaction locks. Confirm
using scoped server credentials and engine-supplied identity. Gameplay history follows the
verified Arma identity; unlink/relink recomputes derived attribution without changing factual
signup, moderation or audit authorship.

## Fleet, missions and telemetry

Use a durable command ledger with separate host-agent and mod-runtime executor types. Both
poll outbound HTTPS with independently revocable per-instance credentials. Host operations
are fixed process-control actions; RCON credentials remain on the host. Custom game-console
input never reaches a host shell. Implement start, stop, restart, players, kick and map change.
Bind kicks to identity and server session so recycled transient IDs cannot target another user.

Accepted commands return receipts. Track queued, claimed, executing, succeeded, failed,
expired, cancelled and indeterminate outcomes. Persist intent before effects, fence stale
executors, serialize incompatible commands and revalidate authority on claim. Lost
acknowledgement never causes blind repetition of a non-idempotent effect or fabricated success.
Test the actual RCON protocol and do not assume chat handlers are RCON handlers.

Compile immutable mission artifacts from exact version, metadata, catalog/modpack,
compiler/schema versions and content digest. Validate exact delivered bytes within 8 MiB.
Review/approval, deployment, loading and roster derivation reference the same artifact.
Reject unsupported authored gameplay data explicitly rather than silently dropping it.
Persist review threads with author/version context and launch a real read-only review workspace.

Validate terrain/resources before changing selection. Same-terrain changes use scenario
restart without requiring a process restart; terrain changes coordinate with the host when
needed. Success requires matching artifact and runtime-session acknowledgement. Exercise
rejected transitions and recovery from partial deployment.

Scope ingestion to the authenticated server. Persist a stable source-match identity before
reporting. Monotonic revisions and payload digests make duplicates inert, conflicting same
revisions errors and older revisions unable to overwrite current results. Higher corrections
may decrease counters while retaining finalization. Validate/store whole batches transactionally
and derive statistics from accepted facts without double counting.

Store combat, medical and vehicle events with stable IDs and deterministic ordering. Invalid
entries reject the entire batch with an indexed error. Mod reporting persists until acknowledged,
with bounded queues and visible failures. Session generations and sequences fence stale
heartbeats. Dashboard and status interfaces explicitly represent the fleet.

## Audit, content and game mathematics

Business changes and required audit/outbox records commit together. A serialized transactional
publisher assigns delivery sequence numbers only to committed pending records. Row allocation
IDs are not commit-order cursors. SSE provides replay IDs, deduplication, reset behavior and
periodic recovery of failed reads even when the notification listener stays healthy.
The audit frontend consumes this feed and deduplicates overlap with paged history.

Complete stable personnel pagination, vehicle mutations, wiki headings/Markdown features,
safe links/images and revision history. Test ownership, validation, storage failures, request
limits and error contracts at actual consumer boundaries. Retain `/api/v1` URLs and update
schemas, generated types, Rust models, frontend DTOs and game protocol consumers together.

Extend the existing headless map-engine game-ballistics module for elevation, drag, wind,
dispersion and battery solutions; share it with the offline frontend. Verify analytic special
cases, symmetry, convergence, bounded failure and native/WASM agreement within one mil.
Keep versioned game-derived calibration fixtures; numerical model validation does not imply
physical-world weapon accuracy.

## Executable invariants and engineering gates

Use proptest with recorded seeds, retained failure traces and explicit case counts for policy precedence,
role/grace boundaries, quota conservation, uniqueness, link consumption, session revocation,
artifact identity, telemetry revisions and command/audit state transitions. Test the production
functions and database constraints, not a disconnected model that merely resembles them.

Every property check enumerates its required property IDs and minimum generated-case counts.
A successful Rust test function does not establish that its generator executed. The shared
recorder counts successful property invocations, verifies that the requested count completed,
and emits the seed, ChaCha RNG identity and a digest of the framed Debug input stream.
Receipts must match these output records exactly. Missing, duplicate, partial or zero-case
runs cannot pass. Domain-wide acceptance includes every required domain invariant; recorder
self-tests do not stand in for application behavior.

Canonical verification rejects any PROPTEST_CASES override before creating a database.
PROPTEST_RNG_SEED accepts a decimal u64 and defaults to 2026092201; the command records and
passes that effective seed to child tests. Configuration fingerprints include all PROPTEST_*
inputs. Case counts belong to suites. Generated regression-file replay is disabled for the
measured stream so local leftover files cannot silently change its identity; retain failing
seed/counterexample output and add explicit regression fixtures alongside the relevant suite.

Use Tokio barriers/channels to control last-seat races, refresh winner/replay ordering,
assignment versus withdrawal, competing linking/approval, duplicate/corrected ingestion and
inverted audit commit order. Check invariants on every observed result and final persisted state.
Inject failures before commit, after commit before response, before/after external effects and
during reconnect. Assert rollback, recovery and explicitly indeterminate effects.

Production files remain strictly below 500 lines and tests below 1000. Decompose expanding
domains into policy evaluation, persistence, transactions and handler modules before growth.
All unit tests use sibling test files through `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
No inline test-module bodies. Preserve domain boundaries and main-only development.

Pin migration checksums and preserve versions 1–21 and 25 onward. Versions **22–24 are retired**
and may never be filled. Migration 27 adds Guest; subsequent migrations append with increasing
versions. The immutable-migration test explicitly rejects gap insertion, duplicates, reordering
and missing historical versions. Enum addition commits before later use of the value.

Database-backed tests require TEST_DATABASE_URL; absence panics rather than passing silently,
including CI=true and TBD_API_VERIFICATION=true. Canonical test-it generates a bounded random
invocation namespace, creates without pre-deleting another database, and cleans only exact
owned names on success, test failure and process-start failure. Redact connection credentials.

Run actual backend, schema/codegen, architecture/file-size, frontend, browser and mod gates.
The readiness register binds each requirement to its real command and named cases. Negative
controls must show that missing tests, stale sources/configuration, failed execution and
insufficient operational measurements cannot produce a passing receipt.

## Operational acceptance and completion

Use the isolated five-server staging fleet and real game clients for identity, control,
same-terrain and cross-terrain deployment, rejection and lost-acknowledgement scenarios.
Exercise live Discord role changes, nonmembership, partner groups, 429s/outages, grace,
warnings, overrides and eligibility re-evaluation without causing a website-wide lockout.

Record hardware, network, fixture and workload identities for a 30-minute workload with
1,000 accounts, 100 concurrent clients and at least 20 requests/second. Target p95 JSON reads
at most 500 ms and writes at most 1 second, with no unexpected errors. Measure asynchronous
game operations separately from HTTP receipt latency.

Implement in dependency order: acceptance/harness foundations; identity, migrations and audit;
event/link/review/telemetry transactions; fleet and artifact integration; remaining consumers
and features; full staging verification. Missing checks remain required and cannot be marked
complete. The completion report identifies executed tests, property coverage, operational
observations, assumptions and any unresolved acceptance failures without claiming formal proof.
