# Identity and access services

The logic behind sign-in and account authority: sessions and their tokens, the Discord client and
the membership snapshot that decides every account's permissions, the Arma identity link, and the
account lookup other domains share.

## Contents

```text
apps/website/api_v2/src/identity_and_access/services/
├── account_authority.rs              an account's current permissions from its verified Discord snapshot
├── cached_membership_permissions.rs  pure decisions over a cached snapshot: grace period, staleness, overrides
├── discord_client.rs                 the Discord OAuth2 and guild-member HTTP client
├── discord_membership_cache.rs       fenced membership observations: lease, accept, record a failure
├── discord_membership_enrollment.rs  enrolls the guilds an event's eligibility needs verified
├── discord_rest_reconciliation.rs    the bot-authenticated reconciler: leases, rate budget, backoff
├── discord_role_sync.rs              re-applies the guild role mapping to every account's display role
├── discord_user_profile.rs           the Discord user profile and the names and avatar URL taken from it
├── identity_linking.rs               spends a link code or unlinks, with attribution, statistics and audit
├── identity_ownership.rs             the identity and account locks, in their fixed order
├── link_code_issuance.rs             issues the single pending six-digit link code of an account
├── membership_grace_overrides.rs     audited, expiring extensions of cached permissions during outages
├── mod.rs                            declares the modules
├── refresh_token_purge.rs            deletes refresh tokens more than seven days past expiry
├── session_authorization.rs          decides a session's current authority in PostgreSQL
├── session_issuance.rs               mints a session and builds the SPA callback redirect
├── session_rotation.rs               single-use refresh rotation and logout under the account lock
├── session_storage.rs                persists sessions and refresh tokens, and revokes them
├── tests/                            unit tests for the permission decisions, the Discord client and profile
└── user_lookup.rs                    loads the live account row behind a Discord id
```

## How it works

- **Sessions.** An access token names a persisted session; `session_authorization.rs` rechecks
  that session and the account's authority in PostgreSQL on every request, through
  `DatabaseSessionAuthority`, which the `AuthUser` extractor in `core` calls. Session writers lock
  the account row before any session or token row (`account_authority::lock_account`), a session
  lasts 30 days, and a development session is refused by a production-configured API.
  `refresh_token_purge.rs` keeps revoked but unexpired tokens, because a replay of one is how
  rotation detects theft.
- **Authority from Discord.** `account_authority.rs` reads the account's verified, guild-scoped
  Discord snapshot and never `users.role`. Cached grants last 48 hours
  (`DEFAULT_MEMBERSHIP_GRACE_PERIOD`) unless an administrator extends them by 1 to 48 hours with a
  reason (`membership_grace_overrides.rs`); a snapshot older than 60 seconds is reported stale. The
  OAuth callback and the reconciler write observations only under the current lease
  (`discord_membership_cache.rs`), so a superseded request cannot restore older grants; a failed
  lookup never changes verified membership. The reconciler admits at most 25 Discord requests per
  second across replicas, and a changed membership queues a re-evaluation of the account's event
  reservations.
- **Arma identity.** A link code is six digits, valid ten minutes, one pending per account.
  Spending it (`identity_linking.rs`) takes the locks of `identity_ownership.rs` (sorted Arma
  identities, sorted accounts, link codes, match and attendance rows, the leaderboard), attributes
  past matches and attendance to the account, recomputes its statistics and appends the required
  audit, all in one transaction.

## Boundaries

- Depends on: `core` (authentication primitives, configuration, errors, the `AuthUser` extractor,
  wire formats); `administration::services::required_audit`; `command_center::services`
  (`user_stats`, `leaderboard_view`); `operations::services` (the reservation re-evaluation queue
  and `participation_attribution`); Discord's REST API over reqwest.
- Used by: `core::application_state`, which holds `DiscordService` and `DatabaseSessionAuthority`;
  the `token_purge_worker`, `discord_role_synchronizer` and `discord_membership_reconciler` workers
  in `apps/website/api_v2/src/background_workers/`; `administration` (role resync, grace extension,
  account locks); `missions`, `operations`, `server_infrastructure` and `match_telemetry`
  (`authorize_on_connection`, `lock_accounts`, `lock_identities`, `holds_administrator_authority`,
  `evaluate_cached_membership_permissions`, the membership enrollment); `command_center`
  (`load_user`); the integration tests in `apps/website/api_v2/tests/`.
- Rules: every writer that touches identities or accounts takes the `identity_ownership.rs` lock
  order; a failed Discord call never downgrades a verified snapshot; `user_lookup.rs` is the one
  read of the account row, so the soft-delete filter lives there once.

## Related documentation

- [Identity transactions](/documentation_v2/website/api_v2/verification_evidence/identity_transactions.md)
  — authorization, Discord observations, linking and their rollout.
