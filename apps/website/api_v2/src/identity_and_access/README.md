# `identity_and_access/`

Who the caller is and how they prove it: Discord OAuth2 sign-in with its cookie-host and CSRF
guards, the single-use rotating refresh token, the caller's own profile, the development login
shortcut, and the six-digit handshake that links a Discord account to an in-game Arma identity.
The account row itself lives here; what an administrator *does* to a member lives in
`administration`.

The token primitives themselves (HS256 issuance and verification, opaque-token hashing) are not
domain-specific — they sit on the shared floor in `core/authentication_primitives/`, where the
middleware extractors can reach them without importing this domain.

## Public surface

- **`routes::routes(dev)`** — the domain's `/api/v1` table, merged by
  `core::http_router::api_v1_routes` and nested under `/api/v1`. The literals in `routes.rs` are
  the public URLs: `/auth/discord/login`, `/auth/discord/callback`, `/auth/refresh`,
  `/auth/logout`, `/auth/dev-login` (registered only when `dev`), `/me`, `/me/link`,
  `/me/link/status`, `/ingest/link-confirm`.
- **`services::user_lookup::load_user`** — the single "load the account row behind a Discord id"
  read. Every domain that resolves a caller to a row calls it.
- **`services::discord_client::DiscordService`** — the Discord OAuth2 and guild-member HTTP client;
  `AppState` holds one instance, which is why `core::application_state.rs` names this domain.
- **`services::discord_role_sync::resync_all_roles`** and
  **`services::refresh_token_purge::purge_expired_refresh_tokens`** — called by the scheduled
  workers and by the administration resync endpoint.
- **`models::user_account`** — `User`, `UserRole`, `DiscordRole`, `UserDiscordRole`,
  `IdentityLinkCode`, `RefreshToken`.

## Dependency rules

- Handlers here never import another domain's handlers; `src/tests/architecture_rules.rs` enforces
  it across all eight domains.
- This domain imports `core`, `administration::{models, services}` (audit rows for privileged
  changes) and `command_center::services::user_stats` (the denormalized counters a new link
  affects).
- `core::application_state.rs` and `core::http_router.rs` are the only `core` files allowed to name
  a domain; the former holds `DiscordService`, the latter merges the route tables.

## Files

```text
mod.rs                                 Domain module tree; re-exports `routes`.
routes.rs                              The `/api/v1` route table for identity and access.
handlers/
  mod.rs                               Sign-in, session rotation, the profile, and the Arma link handshake.
  arma_link_codes.rs                   Player-facing link handshake: issue a six-digit code, report status, unlink.
  arma_link_confirmation.rs            Mod-facing link handshake: `POST /ingest/link-confirm` spends a code.
  developer_login.rs                   Development-only login shortcut that mints a session without Discord.
  discord_oauth.rs                     Discord OAuth2 login and callback.
  member_profile.rs                    The caller's own account: `GET /me` and `PATCH /me`.
  oauth_host_guard.rs                  Cookie-host alignment and the CSRF pre-check for the callback.
  session_tokens.rs                    The single-use rotating refresh token: refresh and logout.
  tests/
    arma_link_confirmation.rs          Sibling unit tests for `arma_link_confirmation.rs`.
    discord_oauth.rs                   Sibling unit tests for `discord_oauth.rs`.
    member_profile.rs                  Sibling unit tests for `member_profile.rs`.
    oauth_host_guard.rs                Sibling unit tests for `oauth_host_guard.rs`.
services/
  mod.rs                               The Discord HTTP client, the profile it yields, and session minting.
  discord_client.rs                    Discord OAuth2 and guild-member HTTP client.
  discord_role_sync.rs                 Discord role → web role sync.
  discord_user_profile.rs              The Discord user profile and the names and avatar URL derived from it.
  refresh_token_purge.rs               Refresh-token retention.
  session_issuance.rs                  Session minting and the SPA callback redirect.
  user_lookup.rs                       Loading the account row behind a Discord id.
  tests/
    discord_client.rs                  Sibling unit tests for `discord_client.rs`.
    discord_role_sync.rs               Sibling unit tests for `discord_role_sync.rs`.
    discord_user_profile.rs            Sibling unit tests for `discord_user_profile.rs`.
    session_issuance.rs                Sibling unit tests for `session_issuance.rs`.
models/
  mod.rs                               Database and wire models for identity and access.
  user_account.rs                      The identity account root and the rows that hang off it.
```

Unit tests live in the sibling files above, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
