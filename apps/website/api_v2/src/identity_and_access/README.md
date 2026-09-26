# Identity and access domain

The [API](/documentation_v2/glossary/a_to_f.md#api)'s
[identity and access](/documentation_v2/glossary/g_to_m.md#identity-and-access) domain: who the caller is
and how they prove it. It holds Discord OAuth2 sign-in with its cookie-host and CSRF guards, the
single-use rotating refresh token, the caller's own profile, the
[dev login](/documentation_v2/glossary/a_to_f.md#dev-login), the six-digit handshake that links a Discord
account to an Arma identity, and the Discord membership snapshot every account's
[role](/documentation_v2/glossary/n_to_z.md#role) comes from.

## Contents

```text
apps/website/api_v2/src/identity_and_access/
├── handlers/  the sign-in, session, profile and Arma link handlers
├── mod.rs     the module tree; re-exports `routes`
├── models/    the account row, its satellite rows and the profile answers
├── routes.rs  the domain's `/api/v1` route table
└── services/  sessions, Discord membership and authority, the Arma link, the account lookup
```

## How it works

A browser signs in through Discord: the callback upserts the account, records the guild-member
observation and issues a session, then redirects to the SPA with an access token and a refresh
token in the URL fragment. The token primitives (HS256 signing, opaque-token hashing) live in
`core::authentication_primitives`, so the middleware can verify a token without importing this
domain. On every request the `AuthUser` extractor verifies the token and asks
`services::session_authorization::DatabaseSessionAuthority` for the session's current authority,
which PostgreSQL decides from the verified Discord snapshot rather than from `users.role`.

Refresh rotates the token once; replaying a spent token revokes the account's sessions. A member
links their Arma identity by asking for a code on the website and typing it in the game, whose
server confirms it with the service token; the confirmation attributes past matches and
attendance to the account in the same transaction. The account row lives here; what an
administrator does to a member lives in `administration`.

## Public surface

- `routes::routes(dev)`: the table `core::http_router` merges under `/api/v1`, one route each:
  - `GET /api/v1/auth/discord/login`: public; starts Discord OAuth2.
  - `GET /api/v1/auth/discord/callback`: public; completes it and redirects to the SPA.
  - `POST /api/v1/auth/refresh`: the refresh token in the body; rotates the session.
  - `POST /api/v1/auth/logout`: the refresh token in the body; revokes the session.
  - `GET /api/v1/auth/dev-login`: registered only in development; `?role=` takes `guest`,
    `enlisted`, `leader`, `mission_maker` or `admin`, and anything else means `admin`.
  - `GET` and `PATCH /api/v1/me`: `AuthUser`; the caller's profile.
  - `POST` and `DELETE /api/v1/me/link`: `AuthUser`; issue a link code, unlink.
  - `GET /api/v1/me/link/status`: `AuthUser`; the link and whether a code is pending.
  - `POST /api/v1/ingest/link-confirm`: `ServiceAuth` (`X-Service-Token`); the game spends a code.
- `services::session_authorization`: `DatabaseSessionAuthority`, which `core::application_state`
  installs behind the `AuthUser` extractor, and `authorize_on_connection`, which `missions`,
  `operations` and `server_infrastructure` call to recheck an administrator inside their
  transactions.
- `services::identity_ownership`: `lock_accounts` and `lock_identities`, the lock order every
  identity, reservation and telemetry transaction takes.
- `services::account_authority::holds_administrator_authority`, read by
  [mission deployments](/documentation_v2/glossary/g_to_m.md#mission-deployment) and
  [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) claims;
  `services::cached_membership_permissions`, read by [event](/documentation_v2/glossary/a_to_f.md#event)
  access in `operations`; `services::discord_membership_enrollment`, called by event administration
  and slotting.
- `services::discord_client::DiscordService`: the Discord HTTP client `core::application_state`
  holds.
- `services::discord_role_sync::resync_all_roles`, `services::refresh_token_purge` and
  `services::discord_rest_reconciliation`: run by the
  [background workers](/documentation_v2/glossary/a_to_f.md#background-workers); `resync_all_roles` also by
  `POST /api/v1/admin/roles/sync`; `services::membership_grace_overrides`, behind the administration
  grace route.
- `services::user_lookup::load_user`: the one read of the account row behind a Discord id.
- `models::user_account`: `User`, `UserRole`, `DiscordRole`, `UserDiscordRole`,
  `IdentityLinkCode`, `RefreshToken`.

## Boundaries

- Depends on:
  - `core`: the application state, configuration, errors, the `AuthUser` and `ServiceAuth`
    extractors, the authentication primitives, the URL guard and the wire formats;
  - `administration::services::required_audit` for the audit rows of link, grace and session
    changes; `command_center::services` for the statistics and leaderboard a link changes;
    `operations::services` for the reservation re-evaluation queue and attendance attribution;
  - Discord's OAuth2 and REST API.
- Used by:
  - `core::http_router`, which merges the route table, and `core::application_state`;
  - the workers in `apps/website/api_v2/src/background_workers/`, and every other domain through
    the services above;
  - over HTTP, the account pages and the navigation frame in `apps/website/frontend/`, and the
    identity link of the mod in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`.
- Rules: handlers never import another domain's handlers, `routes.rs` exports the table the router
  merges, and only `core::application_state` and `core::http_router` name this domain from `core`
  (`apps/website/api_v2/src/tests/architecture_rules.rs` checks all three); every handler carries
  its `/// @route` tag (`cargo xtask verify route-tags`); `models/generated/` is written by
  `cargo xtask ci schema-codegen` and never edited by hand.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes.
- [Identity transactions](/documentation_v2/website/api_v2/verification_evidence/identity_transactions.md)
  — session authorization, Discord observations, linking and attribution.
- [API environment variables](/documentation_v2/website/api_v2/environment_variables.md)
  — the Discord, token and
  session settings, and what an empty one does.
- [Local development](/documentation_v2/runbooks/local_development.md) — the dev login and the
  Discord OAuth2 round trip.
