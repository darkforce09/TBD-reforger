# Identity and access handlers

The HTTP handlers of sign-in, session tokens, the caller's own profile and the handshake that links
a Discord account to an Arma identity. The extractor each handler takes sets its tier; the work
itself runs in the domain's services.

## Contents

```text
apps/website/api_v2/src/identity_and_access/handlers/
├── arma_link_codes.rs         the player half of the Arma link: issue a code, link status, unlink
├── arma_link_confirmation.rs  the game half of the Arma link: the game server spends a code
├── developer_login.rs         the development-only sign-in as a fixed local account per role
├── discord_oauth.rs           Discord OAuth2 login and callback, ending in a session redirect
├── member_profile.rs          the caller's own account: `GET /api/v1/me` and `PATCH /api/v1/me`
├── mod.rs                     the module tree
├── oauth_host_guard.rs        the cookie-host alignment guard and the CSRF check of the callback
├── session_tokens.rs          refresh-token rotation and logout
└── tests/                     unit tests for the link confirmation, OAuth, profile and host guard
```

## How it works

- **Discord sign-in.** `discord_login` sets a ten-minute, host-only `oauth_state` cookie and
  redirects (307) to Discord's consent page. In development it refuses to start when
  `FRONTEND_URL` and `DISCORD_REDIRECT_URL` name different cookie hosts, because the cookie would
  never come back (`oauth_host_guard.rs`); in production it only logs the mismatch.
  `discord_callback` compares the state in constant time, exchanges the code, upserts the `users`
  row, records the guild-member observation (or the failure) under a membership refresh lease,
  refuses a banned account, issues a session and redirects (302) to the SPA's `/auth/callback`
  with the tokens in the URL fragment. Every failure redirects with an `#error=` reason instead,
  and every exit after the state check clears the cookie with `OAUTH_STATE_CLEAR`.
- **[Dev login](/documentation_v2/glossary/a_to_f.md#dev-login).** `dev_login` is registered only in
  development and answers 404 when the configuration says otherwise. `?role=` takes `guest`,
  `enlisted`, `leader`, `mission_maker` or `admin`; anything else signs in as `admin`. Each
  [role](/documentation_v2/glossary/n_to_z.md#role) has its own fixed Discord id and Arma id, and the
  redirect is the one the Discord callback sends.
- **Sessions.** `POST /api/v1/auth/refresh` rotates a single-use refresh token and answers the new
  access token, its expiry and the next refresh token; replaying a consumed token revokes the
  account's current sessions. `POST /api/v1/auth/logout` revokes the session and answers 204, also
  for an unknown token.
- **Profile.** `GET /api/v1/me` answers the stored account with the session's role, `arma_linked`
  and the membership flags (`membership_stale`, `membership_override_active`,
  `can_manage_membership_override`). `PATCH /api/v1/me` changes nothing: every field is owned by
  the Discord sign-in or the link flow, so it answers the stored account.
- **Arma link.** `POST /api/v1/me/link` issues a six-digit code valid for ten minutes and
  supersedes the caller's pending one (201); `GET /api/v1/me/link/status` reports the link and
  whether a code is pending; `DELETE /api/v1/me/link` removes the link. The game server spends the
  code with `POST /api/v1/ingest/link-confirm` (`{code, arma_id, arma_character}`, unknown fields
  refused).

## Boundaries

- Depends on: the domain's services (`session_issuance`, `session_rotation`,
  `discord_membership_cache`, `link_code_issuance`, `identity_linking`, `user_lookup`) and
  `models::current_profile`; `core` for the application state, the `AuthUser` and `ServiceAuth`
  extractors, `authentication_primitives`, `http_url_guard` and the RFC 3339 wire format.
- Used by: the domain's `routes.rs`; over HTTP, the account pages (login, auth callback, settings)
  and the navigation frame under `apps/website/frontend/src/v2/pages/`, the
  [API](/documentation_v2/glossary/a_to_f.md#api) client's token refresh in
  `apps/website/frontend/src/v2/core/api/client/refresh.rs`, and the mod's
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_IdentityLink.c`, which confirms link codes.
- Rules: every handler carries its `/// @route` tag (`cargo xtask verify route-tags`); no handler
  imports another domain's handlers (`apps/website/api_v2/src/tests/architecture_rules.rs`); tokens
  leave the API only in a URL fragment or a JSON body, never in a query string.

## Related documentation

- [Identity transactions](/documentation_v2/website/api_v2/verification_evidence/identity_transactions.md)
  — session authorization, linking and their transactions.
- [Local development](/documentation_v2/runbooks/local_development.md) — the dev login and the
  Discord OAuth2 round trip.
