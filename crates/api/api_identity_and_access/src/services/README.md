# Identity and access services

The logic behind sign-in and account authority: sessions and their tokens, the membership
snapshot that decides every account's permissions, the Arma identity link, and the account lookup
other domains share. The Discord OAuth2 and guild-member client and the user profile it returns
live in the `api_discord` crate, `crates/api/api_discord/`. The checks every domain reads (the
session and account authority, the cached membership decision and the identity and account
locks) live in the caller identity crate, `crates/api/api_caller_identity/src/`.

## Contents

```text
crates/api/api_identity_and_access/src/services/
├── account_registration.rs           creates an account or refreshes its Discord profile at sign-in
├── discord_membership_cache.rs       fenced membership observations: lease, accept, record a failure
├── discord_membership_enrollment.rs  enrolls the guilds an event's eligibility needs verified
├── discord_rest_reconciliation.rs    the bot-authenticated reconciler: leases, rate budget, backoff
├── discord_role_sync.rs              re-applies the guild role mapping to every account's display role
├── identity_linking.rs               spends a link code or unlinks: attribution, statistics, audit
├── link_code_issuance.rs             issues the single pending six-digit link code of an account
├── membership_grace_overrides.rs     audited, expiring extensions of cached permissions during outages
├── mod.rs                            the module tree
├── refresh_token_purge.rs            deletes refresh tokens more than seven days past expiry
├── session_issuance.rs               mints a session and builds the SPA callback redirect
├── session_rotation.rs               single-use refresh rotation and logout under the account lock
├── session_storage.rs                persists sessions and refresh tokens, and revokes them
├── tests/                            unit tests for the registration and role sync services
└── user_lookup.rs                    loads the live account row behind a Discord id
```

## How it works

- **Accounts.** `account_registration.rs` is the one write of an account's Discord profile
  columns: the OAuth callback registers the profile Discord returned, creating the account at its
  first sign-in, and the avatar URL reaches `users.avatar_url` only when it is http(s).
- **Sessions.** An access token names a persisted session; `api_caller_identity`'s
  `session_authorization.rs` rechecks that session and the account's authority in PostgreSQL on
  every request, through `DatabaseSessionAuthority`, which the `AuthUser` extractor of
  `api_http_layer` calls. Session writers lock the account row before any session or token row
  (`account_authority::lock_account`), a session
  lasts 30 days, and a development session is refused by a production-configured
  [API](/documentation/glossary/a_to_f.md#api). `refresh_token_purge.rs` keeps revoked but unexpired
  tokens, because a replay of one is how rotation detects theft.
- **Authority from Discord.** `api_caller_identity`'s `account_authority.rs` reads the account's verified, guild-scoped
  Discord snapshot and never `users.role`. Cached grants last 48 hours
  (`DEFAULT_MEMBERSHIP_GRACE_PERIOD`) unless an administrator extends them by 1 to 48 hours with a
  reason (`membership_grace_overrides.rs`); a snapshot older than 60 seconds is reported stale. The
  OAuth callback and the reconciler write observations only under the current lease
  (`discord_membership_cache.rs`), so a superseded request cannot restore older grants; a failed
  lookup never changes verified membership. The reconciler admits at most 25 Discord requests per
  second across replicas, and a changed membership queues a re-evaluation of the account's
  [event](/documentation/glossary/a_to_f.md#event) reservations.
- **Arma identity.** A link code is six digits, valid ten minutes, one pending per account.
  Spending it (`identity_linking.rs`) takes `api_caller_identity`'s `identity_ownership.rs` locks (sorted Arma
  identities, sorted accounts, link codes, match and attendance rows, the leaderboard), attributes
  past matches and attendance to the account, recomputes its statistics and appends the required
  audit, all in one transaction.
- **Failpoints.** Test builds pass the `api_failpoints` points at these boundaries:
  `session_rotation.rs` passes `SessionRotationBeforeCommit` after the successor token is written
  and `SessionRotationAfterCommit` after the commit, and `SessionLogoutBeforeCommit` after the
  logout audit; `identity_linking.rs` passes `IdentityLinkConfirmBeforeCommit` after the
  leaderboard refresh; `discord_rest_reconciliation.rs` passes `DiscordRoleSyncBeforeEffect` once
  the lease and the request budget are held and `DiscordRoleSyncAfterEffect` once Discord has
  answered, before the observation or failure is recorded. A deploy build compiles them out.

## Boundaries

- Depends on: the API crates `api_http_layer` (authentication primitives, the `AuthUser`
  extractor), `api_configuration`, `api_foundation` (errors, wire formats),
  `api_audit_log` (`required_audit`), `api_caller_identity` (the account authority, `UserRole`
  and the locks), `api_member_activity` (`user_stats`, `leaderboard_view`, the reservation
  re-evaluation queue and `participation_attribution`) and `api_discord` (`DiscordService`,
  `GuildMember`, `DiscordUser` and its typed `Error`).
- Used by: the `token_purge_worker`, `discord_role_synchronizer` and `discord_membership_reconciler` workers
  in `crates/api/api_background_workers/src/`; `api_administration`
  ([role](/documentation/glossary/n_to_z.md#role) resync, grace extension); `api_operations` (the
  membership enrollment, `load_user`); `api_command_center` (`load_user`); the `staging-fixtures` host
  tool in `tools/staging/staging_fixtures/src/` (`register_account`, `claim_membership_refresh`,
  `accept_membership_observation`, `issue_refresh`); the integration tests in
  `crates/api/api_server/tests/`.
- Rules: every writer that touches identities or accounts takes `api_caller_identity`'s
  `identity_ownership.rs` lock order; a failed Discord call never downgrades a verified snapshot; `user_lookup.rs` is the one
  read of the account row, so the soft-delete filter lives there once.

## Related documentation

- [Identity transactions](/documentation/crates/api/api_server/verification_evidence/identity_transactions.md)
  — authorization, Discord observations, linking and their rollout.
