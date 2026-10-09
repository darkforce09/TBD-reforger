# API Discord

The `api_discord` crate: the [API](/documentation/glossary/a_to_f.md#api)'s Discord clients. It
builds the OAuth2 consent URL and exchanges the authorization code, reads the user's profile and
guild member, reads guild members with the bot token for the membership reconciliation, and pushes
announcement embeds to the channel webhook, reporting every failure as a typed `Error`.

## Contents

```text
crates/api/api_discord/
├── Cargo.toml  the package: `reqwest` with `rustls`, the API foundation and HTTP layer crates, layout tier 4
└── src/        the OAuth2 and guild-member client, the user profile, the webhook, the failures and their tests
```

## How it works

Both clients install the ring TLS provider once (one shared `tls_crypto_provider` module) and build
a `reqwest` client with a 10 second timeout. A call answered `429` waits the `Retry-After` it
names, within a bound, through
`api_http_layer`'s outbound retry. A non-2xx answer becomes `Error::DiscordStatus` or
`Error::WebhookStatus` with at most 4096 characters of its body; a transport or JSON failure is the
library's own error, so it renders with its causes. The bot-token member read answers a
`MembershipLookupFailure` instead of an error when Discord gives no usable membership, so the
reconciliation can record the outcome and back off. The webhook sanitises every embed field at the
sink, so a copied title can never become a spreadsheet formula.

## Getting started

Run from the repository root:

```bash
cargo test -p api_discord   # the response decoding, the profile checks and the webhook payload; no network
cargo xtask db test-it --test discord_http_clients --test discord_embed_sanitisation
```

The integration suites in `crates/api/api_server/tests/` drive both clients against loopback Discord stand-ins.

## Configuration

No feature of its own. The API's `Config` carries the values the clients take:
`DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, `DISCORD_REDIRECT_URL`, `DISCORD_GUILD_ID`,
`DISCORD_BOT_TOKEN` and `DISCORD_WEBHOOK_URL` (empty disables pushing); `HTTPS_PROXY` and `NO_PROXY` apply to
every request ([environment variables](/documentation/crates/api/api_server/environment_variables.md)).

## Public surface

- `discord_client`: `DiscordService`, `TokenResponse`, `GuildMember`, `DEFAULT_DISCORD_API`.
- `discord_user_profile::DiscordUser` and its derived display name, handle and avatar URL.
- `discord_webhook`: `WebhookService`, `WebhookAnnouncement`, `sanitize_discord_embed_field`.
- `membership_lookup_failure::MembershipLookupFailure`.
- `Error` and `Result`, and `prelude` (both clients, their answers and `Error`).

## Boundaries

- Depends on: `api_http_layer` (the outbound retry, the reconciliation outcome), `api_foundation`
  (the embed field caps), `api_identifiers` (the Discord snowflakes); `reqwest`, `rustls`, `serde`,
  `serde_json`, `chrono`, `url`, `tokio` and `thiserror`.
- Used by: the API application (`crates/api/api_server`): its composition and application state, the OAuth
  handlers and membership reconciliation of `api_identity_and_access`, the announcement push of
  `api_community_content`, the `staging-fixtures` host tool and the integration suites.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); nothing here names a domain;
  no error variant carries a token, a secret or an authorization code.

## Related documentation

- [API Discord source](/crates/api/api_discord/src/README.md) — each client and failure in detail.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
