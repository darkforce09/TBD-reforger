# API Discord source

The source of `api_discord`: everything the API sends to Discord over HTTP, the OAuth2 and
guild-member client, the user profile it returns, the announcement webhook, and their typed
failures.

## Contents

```text
crates/api/api_discord/src/
├── discord_client.rs             `DiscordService`: the consent URL, the token exchange, the profile and guild-member reads
├── discord_user_profile.rs       `DiscordUser`: the `/users/@me` profile and its derived names and avatar URL
├── discord_webhook.rs            `WebhookService`, `WebhookAnnouncement`: posts an announcement embed, answers the message id
├── error.rs                      `Error`: every failure of the OAuth2, guild-member and webhook calls, and `Result`
├── lib.rs                        the crate root: module header, `mod` lines and the re-export of `Error`
├── membership_lookup_failure.rs  `MembershipLookupFailure`: a bot member read with no usable answer
├── prelude.rs                    both clients, their answers and `Error` for glob import
├── tests/                        unit tests for the client, the profile checks and the webhook payload
└── tls_crypto_provider.rs        `ensure_tls_provider`: installs the rustls ring provider once, before either client is built
```

## How it works

`DiscordService` builds the consent URL, exchanges an authorization code for a token, reads the
user's profile and guild member with the user's token, and reads a guild member with the bot
token for the REST membership reconciliation. Its 429 retry is bounded and honours
`Retry-After`. `DiscordUser` refuses a profile with no `username` and derives the display name,
the handle and an avatar URL whose id and hash segments hold only `[A-Za-z0-9_]`, so a hostile
profile cannot walk the URL out of the CDN's avatar path.

`WebhookService` holds the `DISCORD_WEBHOOK_URL` the configuration reads; an empty URL disables
pushing. It posts a `WebhookAnnouncement` (title, body, snippet, footer category and sidebar
colour), which names no announcement type: the announcement push of `api_community_content` maps its
`Announcement` into it. Before posting, `sanitize_discord_embed_field` strips ASCII control
characters and puts a zero-width space before a leading `=`, `+`, `-` or `@`, so a copied title
cannot become a spreadsheet formula.

`error::Error` names each way a Discord call fails: a missing client id, an empty access token, a
membership read Discord could not answer, a non-2xx status with the start of its body (Discord's
or the webhook's), a disabled webhook, and the transport or JSON failures of `reqwest` and
`serde_json`, which render exactly as those libraries render them. The OAuth handlers classify a
failure by its variant (a `Transport` error is an outage or a protocol fault, any other variant
means Discord answered and refused) and log it with every cause through
`api_foundation::error_handling::error_causes::message_with_causes`.

`membership_lookup_failure::MembershipLookupFailure` is what the bot-token member read reports
instead of a membership: a fixed reason, how long to wait, and whether Discord rate-limited the
read. Its `outcome` is the reconciliation outcome the metrics registry counts.

## Boundaries

- Depends on: `api_http_layer` (the 429 retry of `http_client`, the reconciliation outcome of
  `observability`), `api_foundation` (the text caps of `text`), `api_identifiers` (the Discord
  snowflakes); `thiserror`, `reqwest`, `rustls`, `serde`, `serde_json`, `chrono`, `url`, `tokio`.
- Used by: the API's `composition`, which builds both clients; `api_state`'s `AppState`, which holds
  them;
  the OAuth handlers, the account registration and the REST membership reconciliation of
  `api_identity_and_access`; the announcement push of `api_community_content`; the `staging-fixtures` host
  tool; the integration suites in `crates/api/api_server/tests/` (`discord_http_clients.rs`,
  `discord_embed_sanitisation.rs` and the other Discord suites).
- Rules: no file here names a domain; no variant carries a token, a secret or an authorization
  code; the embed is sanitised here, at the sink, and nowhere else.

## Related documentation

- [API Discord](/crates/api/api_discord/README.md) — the crate, its surface and its tests.
