# API identity and access

The `api_identity_and_access` crate: the [API](/documentation/glossary/a_to_f.md#api)'s
[identity and access](/documentation/glossary/g_to_m.md#identity-and-access) domain. It holds
Discord OAuth2 sign-in, the rotating refresh token and logout, the
[dev login](/documentation/glossary/a_to_f.md#dev-login), the caller's own profile, the six-digit
handshake that links a Discord account to an Arma identity, and the Discord membership snapshot
every account's [role](/documentation/glossary/n_to_z.md#role) comes from, with the `/api/v1`
route table the API's router merges.

## Contents

```text
crates/api/api_identity_and_access/
├── Cargo.toml  the package: `api_state`, `api_caller_identity`, `api_discord`, `api_member_activity`, `api_audit_log`, `api_http_layer`, sqlx (`postgres`), axum, layout tier 6
└── src/        the route table, the handlers, the account, session and membership services, the account models, the error and the prelude
```

## How it works

A browser signs in through Discord: the callback upserts the account, records the guild-member
observation and issues a session, then redirects to the single-page app with an access token and
a refresh token in the URL fragment. Refresh rotates the token once, and a replayed token revokes
the account's sessions. Every request's authority is decided by `api_caller_identity` from the
verified Discord membership snapshot this crate records, never from `users.role`.

A member links an Arma identity by asking for a code on the website and typing it in the game,
whose server confirms it with its `mod_runtime` machine credential; the confirmation attributes
past matches and attendance to the account in the same transaction. The source tree README has
the detail.

## Getting started

Run from the repository root:

```bash
cargo test -p api_identity_and_access
cargo clippy -p api_identity_and_access --all-targets -- -D warnings
cargo xtask db test-it --test auth_refresh --test dev_login_source_contract --test route_acceptance_identity_and_core
```

The unit tests cover the OAuth host and CSRF guards, the OAuth failure classification, the role
sync, the account registration and the link confirmation; the SQL is proved against Postgres by
the API's integration suites.

## Configuration

No feature and no variable of its own. The handlers read the Discord OAuth client, the configured
guild, the frontend URL and the development flag from the `Config` the API loads at boot
(`api_configuration`); [API environment variables](/documentation/crates/api/api_server/environment_variables.md)
lists them.

## Public surface

- `routes(dev)`: the domain's `/api/v1` route table; `dev` registers the development login.
- `handlers`: the sign-in, session token, profile and Arma link handlers, each with its
  `/// @route` tag.
- `services`: session issuance, rotation and storage, the account registration and lookup, the
  Discord membership cache, enrollment, reconciliation and role sync, the membership grace
  overrides, the link code issuance and identity linking, the refresh token purge.
- `models`: `User`, `DiscordRole`, `UserDiscordRole`, `IdentityLinkCode`, `RefreshToken` and the
  profile answers.
- `Error` and `Result` (a read or write failure, converting into `ApiError`), and `prelude`.

## Boundaries

- Depends on: `api_state`, `api_caller_identity`, `api_discord`, `api_member_activity`,
  `api_audit_log`, `api_http_layer`, `api_configuration`, `api_foundation`,
  `api_identifiers`, `fleet_wire_contract`, `http_url_guard`, sqlx, axum, serde, chrono, url,
  tracing, thiserror and uuid; Discord's OAuth2 and REST API. It names no other domain.
- Used by: the API application (`crates/api/api_server`): its router merges `routes`, its background workers
  run the role sync, the membership reconciliation and the refresh token purge, the administration
  and operations domains call the services, and the `staging-fixtures` host tool and the
  integration suites reach the services directly.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); its route table, its handlers
  and its imports follow the domain graph.

## Related documentation

- [API identity and access source](/crates/api/api_identity_and_access/src/README.md) — the files,
  the routes and how sign-in, sessions and the link handshake work.
- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [Identity transactions](/documentation/crates/api/api_server/design_notes/identity_transactions.md)
  — session authorization, Discord observations, linking and attribution.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
