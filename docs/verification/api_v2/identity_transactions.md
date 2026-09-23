# Identity transactions and rollout

## Authorization

Access tokens identify persisted sessions with an HS256 signature, issuer, audience, expiry,
account ID and session UUID. PostgreSQL account/session state supplies current authority.
Development sessions carry explicit provenance and are rejected under production configuration,
including before refresh-replay revocation. Logout and refresh rotation lock the account before
session/token state. Replaying a consumed token in an active session revokes current account sessions;
credentials from an already retired family cannot revoke a subsequent recovery login. Ban and
deletion updates revoke stored credentials in the account transaction.

Discord REST observations use a revision and lease token. Only the current, unexpired lease can
replace membership and guild-scoped roles. A failed request preserves verified facts. Periodic
workers acquire a request budget after claiming the account; a newer shared backoff prevents
an unstarted request. Retry deadlines use the PostgreSQL clock. Fair scheduling and Discord
availability remain assumptions for the measured 60-second propagation target.

Administrative recovery accepts a duration of 1–48 hours and derives expiry from database time.
Each transaction records prior expiry, resulting expiry, guild, actor, target and reason. Membership
facts and persisted session lifetimes use the database clock; JWT expiry uses the issuer clock.
Browser profile requests carry session generations and request order. An immutable session UUID
correlates tabs; decoding it in the browser is not authentication. A different session invalidates
old profile work even when the account is unchanged. Credential storage writers share a browser
Web Lock; profile updates preserve the latest stored refresh token and expiry across rotations.
Logout removes the current session's rotated successor while preserving another login.

The browser removes a refresh credential from shared storage before sending it. A failed removal
prevents transmission. Lost responses and failed successor writes leave the spent credential absent;
no retry restores it. A 30-second fetch/body deadline aborts a stalled refresh and releases the Web
Lock. This may require sign-in again after an ambiguous network failure; it prevents replay of an
already-spent credential. Startup stays unresolved until a locked storage read settles. Web Locks,
a secure context, and writable browser storage are prerequisites for persistent sign-in. Old frontend
tabs must reload during coordinated cutover because they do not obey the consume-before-send rule.

## Linking and gameplay attribution

The existing in-game linking command supplies the engine identity. API transactions enforce
one pending code per account, a single consumption identity, and unique current ownership.
A duplicate confirmation for the same still-linked identity acknowledges the existing result;
it does not spend the code or emit another link audit. An incompatible replay conflicts.
An account must explicitly unlink before changing to a different Arma identity. Unlink also
cancels pending codes. Consumed and cancelled code history is retained.

A verified pending code can reclaim an identity whose former account is soft-deleted. The
transaction includes that current owner in its sorted account lock set even when no gameplay
rows remain attributed to it. Active owners still cause a conflict. Reclamation clears the
deleted account's current identity, cancels pending codes, keeps its sessions revoked, and
records the released owner in a required audit. Gameplay reassignment and recomputation for
both accounts commit with code consumption; signup, attendance and audit authorship remain
with their factual accounts. Invalid or expired codes cannot release the former ownership.

The lock order for gameplay attribution is:

1. Source-match advisory guard for an ingestion request, if it supplies a source identity.
2. All affected Arma identity rows, sorted lexicographically.
3. All affected account rows, sorted lexicographically, with FOR NO KEY UPDATE.
4. Code consumption, match facts and attendance updates.
5. Serialized leaderboard refresh, followed by transaction commit.

A correction includes previously recorded and submitted participants before acquiring identity
locks. A linking operation holds the relevant identity lock while deriving attendance, so a
concurrent correction cannot change the underlying match association midway through that claim.
The two-integer advisory namespace 1 is reserved for source-match serialization; the leaderboard
uses PostgreSQL's separate bigint advisory namespace. Hash collisions serialize unrelated work.

The partial unique Arma ownership index preserves uniqueness for non-NULL identities while
keeping updates compatible with foreign-key KEY SHARE checks on account IDs. Barrier tests
exercise the event-lock/account-FK cycle and actual linking under a retained KEY SHARE lock.
This behavior follows PostgreSQL's documented [row-lock rules](https://www.postgresql.org/docs/18/explicit-locking.html#LOCKING-ROWS)
and is checked against the running database version.

Statistics and required audit/outbox rows commit with linking, unlinking and accepted telemetry.
A statistics or audit failure rolls back code consumption and ownership. Attendance numerator
and denominator share the same past-registration population, and zero denominators produce zero.
Signup, moderation and audit authorship are not transferred by linking. Confirmation retries
return the persisted character name even when a retry supplies a different name.

The six-digit code namespace is finite. Codes are not reused while their consumption history is
retained; issuance makes at most 32 collision attempts and reports an error without invalidating
the previous code if no unused value can be allocated. Extending this protocol requires a
coordinated consumer change and acceptance evidence; this limit is not an unbounded liveness claim.

## Coordinated rollout

Migrations 0027–0035 are forward-only, with individual checksum pins. 0027 introduces Guest
before subsequent migrations can use it. The historical numbering gap remains unfilled.
0030 creates persisted sessions; 0032 revokes credentials whose development/production provenance
cannot be established. Existing users must sign in again. Account and historical facts remain.
0033 preserves code history while cancelling all but the latest pending code per account.
0034 retains unique current ownership with foreign-key-compatible index semantics.
0035 reconciles gameplay attribution against current nondeleted Arma ownership and recomputes
aggregates without rewriting signup, moderation or audit authorship. Its required repair audit
is part of the migration transaction; an audit failure rolls back the repair. Quiesce old API
and background writers for this migration and the coordinated binary cutover.

Use a coordinated API/schema/frontend cutover in staging. Old API code cannot continue issuing
codes against the new one-pending-code constraint. Verify Discord bot access, authoritative cache
reconciliation and administrator recovery before production rollout; old independent website roles
must never be imported as verified membership. Reverting only the binary is not a supported rollback.

Server-scoped machine credentials, game-wire versioning, complete finalized-attendance semantics,
and live mod/fleet acceptance remain required by the completion register. The current linking
transaction tests do not establish those separate requirements or operational readiness.
