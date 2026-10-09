# Discord member probe

Bot-authenticated reads of one guild member for the staging Discord procedure:
`observe-discord-member` witnesses a membership or a role change once, and
`spend-discord-member-bucket` spends the Get Guild Member rate-limit bucket at a chosen instant so
the API's own refresh meets a 429.

## Contents

```text
tools/staging/staging_fixtures/src/discord_member_probe/
├── bucket_spend.rs      `spend-discord-member-bucket`: the start and hold, the request cap, the summary
├── discord_target.rs    the member, guild and API base of a read, the bot credential and the client
├── member_read.rs       one read, its outcome and rate-limit headers, and `observe-discord-member`
├── mod.rs               the two parsers
└── tests/               unit tests for the target flags, the answer classification and the spend schedule
```

## How it works

Both subcommands take the same target flags, `--discord-id`, `--guild main|partner` (with
`--partner-guild-id` for a partner guild) and the test-only `--discord-api-base`, and resolve them
once the guards pass: the main guild is `DISCORD_GUILD_ID` of the API env file, the bot token is its
`DISCORD_BOT_TOKEN`, and the request goes to `https://discord.com/api/v10` unless the base names an
http(s) URL on a loopback IP address, which is how the suite points the tool at its fake Discord.

A read sends `GET /guilds/{guild}/members/{member}` with `Authorization: Bot <token>` and classifies
the answer the way the API's `fetch_member_with_bot` does, then prints one line:

```text
discord-member-read {"request":1,"guild":"main","guild_id":"…","discord_id":"…",
  "sent_at_unix_ms":…,"answered_at_unix_ms":…,"http_status":200,
  "outcome":"member|nonmember|rate_limited|unavailable","roles":[…]|null,
  "retry_after_ms":…|null,"global":…|null,"reason":"…"|null,
  "rate_limit":{"bucket":…,"limit":…,"remaining":…,"reset_after_ms":…,"scope":…}}
```

(one line on stdout; wrapped here). `observe-discord-member` makes one read and exits 0 when it
observed a membership (member or nonmember), 1 otherwise. `spend-discord-member-bucket` waits until
`--start-at-unix-ms`, reads at once while the bucket has room, and once a read says the bucket is
spent (`remaining` 0 or a 429) reads again only after the reset or retry delay, until
`--hold-seconds` have passed or `--max-requests` (at most 50) are sent; a read that is neither a
membership nor a 429 stops it. It prints a line per read and a summary:

```text
discord-bucket-spend {"requests":…,"rate_limited_responses":…,"bucket_spent":true,
  "first_spent_at_unix_ms":…,"started_at_unix_ms":…,"hold_until_unix_ms":…,
  "stopped_because":"hold_ended|request_cap|discord_unavailable", …}
```

and exits 0 only when it saw the bucket spent and stopped at the hold or the cap.

## Boundaries

- Depends on: `crate::{argument_list, guarded_context, tool_failure}`; the `api_discord` crate's
  `discord_client::DEFAULT_DISCORD_API`; `reqwest` with the `rustls`
  ring provider, `url`, `serde_json` and `tokio`.
- Used by: the subcommand table in `tools/staging/staging_fixtures/src/main.rs`; the
  `staging discord` harness's `discord_member_reader` observer, which runs both on the host and
  parses their lines; `tools/staging/staging_fixtures/tests/staging_fixtures_discord.rs`.
- Rules: the bot token goes only into the `Authorization` header of a request to Discord or to a
  loopback address, and never into a message or a printed line (`tests/discord_target.rs`; the
  integration suite checks every run's output); a spend never sends more than 50 requests, never
  reads a spent bucket before its reset, and never reads after the hold (`tests/bucket_spend.rs`
  and the fake-Discord suite); a dry run sends nothing.

## Related documentation

- [Staging acceptance](/documentation/crates/api/api_server/verification_evidence/staging.md) — the
  Discord procedure's steps that use these reads.
- [Staging fixtures crate](/tools/staging/staging_fixtures/README.md#staging-fixtures) — the subcommands'
  flags and exit codes.
