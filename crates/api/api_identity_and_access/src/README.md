# API identity and access source

The [API](/documentation/glossary/a_to_f.md#api)'s
[identity and access](/documentation/glossary/g_to_m.md#identity-and-access) domain: who the caller is
and how they prove it. It holds Discord OAuth2 sign-in with its cookie-host and CSRF guards, the
single-use rotating refresh token, the caller's own profile, the
[dev login](/documentation/glossary/a_to_f.md#dev-login), the six-digit handshake that links a Discord
account to an Arma identity, and the Discord membership snapshot every account's
[role](/documentation/glossary/n_to_z.md#role) comes from.

## Contents

```text
crates/api/api_identity_and_access/src/
├── error.rs    `Error` and `Result`: a read or write failure, converting into `ApiError`
├── handlers/   the sign-in, session, profile and Arma link handlers
├── lib.rs      the crate root; re-exports `routes`, `Error` and `Result`
├── models/     the account row, its satellite rows and the profile answers
├── prelude.rs  the services and the account row a caller imports with `prelude::*`
├── routes.rs   the domain's `/api/v1` route table
└── services/   sessions, Discord membership and authority, the Arma link, the account lookup
```

## How it works

A browser signs in through Discord: the callback upserts the account, records the guild-member
observation and issues a session, then redirects to the SPA with an access token and a refresh
token in the URL fragment. The token primitives (HS256 signing, opaque-token hashing) live in
`api_http_layer::authentication_primitives`, so the middleware can verify a token without importing this
domain. On every request the `AuthUser` extractor verifies the token and asks
`api_caller_identity::session_authorization::DatabaseSessionAuthority` for the session's
current authority, which PostgreSQL decides from the verified Discord snapshot rather than from
`users.role`; the caller identity crate (`crates/api/api_caller_identity/src/`) holds that
check, the account authority read, the identity and account locks and `UserRole`, so every domain
reads them without depending on this one.

Refresh rotates the token once; replaying a spent token revokes the account's sessions. A member
links their Arma identity by asking for a code on the website and typing it in the game, whose
server confirms it with its `mod_runtime`
[machine credential](/documentation/glossary/g_to_m.md#machine-credential); the confirmation
attributes past matches and attendance to the account in the same transaction, and its audit row
names the confirming server. The account row lives here; what an
administrator does to a member lives in `api_administration`.

## Public surface

- `routes::routes(dev)`: the table the API's router (`api_server::router`) merges under `/api/v1`, one
  route each:
  - `GET /api/v1/auth/discord/login`: public; starts Discord OAuth2.
  - `GET /api/v1/auth/discord/callback`: public; completes it and redirects to the SPA.
  - `POST /api/v1/auth/refresh`: the refresh token in the body; rotates the session.
  - `POST /api/v1/auth/logout`: the refresh token in the body; revokes the session.
  - `GET /api/v1/auth/dev-login`: registered only in development; `?role=` takes `guest`,
    `enlisted`, `leader`, `mission_maker` or `admin`, and anything else means `admin`.
  - `GET` and `PATCH /api/v1/me`: `AuthUser`; the caller's profile.
  - `POST` and `DELETE /api/v1/me/link`: `AuthUser`; issue a link code, unlink.
  - `GET /api/v1/me/link/status`: `AuthUser`; the link and whether a code is pending.
  - `POST /api/v1/ingest/link-confirm`: `mod_runtime` machine credential (`MachineCaller`); the
    game spends a code.
- `services::discord_membership_enrollment`, called by
  [event](/documentation/glossary/a_to_f.md#event) administration and slotting.
- `services::discord_role_sync::resync_all_roles`, `services::refresh_token_purge` and
  `services::discord_rest_reconciliation`: run by the
  [background workers](/documentation/glossary/a_to_f.md#background-workers); `resync_all_roles` also by
  `POST /api/v1/admin/roles/sync`; `services::membership_grace_overrides`, behind the administration
  grace route.
- `services::user_lookup::load_user`: the one read of the account row behind a Discord id.
- `models::user_account`: `User`, `DiscordRole`, `UserDiscordRole`, `IdentityLinkCode`,
  `RefreshToken`.

## Boundaries

- Depends on:
  - the API crates `api_state` (the application state), `api_configuration`, `api_foundation`
    (errors and wire formats), `api_http_layer` (the `AuthUser` extractor and the authentication
    primitives), and `api_discord` (the Discord clients), and the URL guard
    `http_url_guard`;
  - the API crates `api_audit_log` (the audit log writers for the audit rows of link, grace and
    session changes), `api_caller_identity` (session authority, account authority, locks,
    `UserRole`, the machine caller of the link confirmation) and `api_member_activity` (the
    member activity aggregates a link changes, the reservation re-evaluation queue and
    attendance attribution);
  - Discord's OAuth2 and REST API.
- Used by:
  - the API's router (`crates/api/api_server/src/router.rs`), which merges the route table;
  - the workers in `crates/api/api_background_workers/src/`, and every other domain through
    the services above;
  - over HTTP, the account pages and the navigation frame in `crates/frontend/shell/frontend_application/`, and the
    identity link of the mod in `mod/tbd-framework/Scripts/Game/TBD/API/`.
- Rules: handlers never import another domain's handlers, `routes.rs` exports the table the router
  merges, and only the router names this crate from the API's application source; every handler
  carries its `/// @route` tag; the domain's generated contract types
  (`contract_schema_types::identity_and_access`) are written by `cargo xtask ci schema-codegen`
  and never edited by hand.

## Related documentation

- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [Identity transactions](/documentation/crates/api/api_server/design_notes/identity_transactions.md)
  — session authorization, Discord observations, linking and attribution.
- [API environment variables](/documentation/crates/api/api_server/environment_variables.md)
  — the Discord, token and
  session settings, and what an empty one does.
- [Local development](/documentation/runbooks/local_development.md) — the dev login and the
  Discord OAuth2 round trip.
