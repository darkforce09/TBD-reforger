# Load population

The synthetic member accounts a staging load run signs in with: `seed-load-population` creates
them as verified members with refresh sessions and writes their tokens to the load engine's account
file, and `clean-load-population` deletes them with every row they own.

## Contents

```text
tools/staging/staging_fixtures/src/load_population/
├── account_file.rs          the account file: its target checks, the load engine's format, the mode-600 write
├── mod.rs                   the two parsers, the reserved id range of a population and the synthetic profiles
├── population_cleanup.rs    `clean-load-population`: the census and the deletion, also used to undo a seeding
└── population_seeding.rs    `seed-load-population`: the guards, the per-account services and the undo
```

## How it works

A seeding runs its guards in every mode: no `DISCORD_BOT_TOKEN` in the API env file (the reconciler
would ask Discord about every synthetic account and demote it to guest), a `DISCORD_GUILD_ID`, no
account of the reserved range, exactly one `discord_roles` row named by `--role` mapped to
`enlisted` or to nothing, and an account file path where nothing stands in a directory only its
owner can enter. An applied seeding then goes account by account, `k` from 0, Discord id
`first + k`:

```text
account_registration::register_account        users row "Load Member <k in five digits>"
  ─▶ claim_membership_refresh (forced)        the main guild's snapshot lease
  ─▶ accept_membership_observation            member, holding the --role Discord role; site role
  ─▶ session_issuance::issue_refresh          a refresh-only session
all accounts ─▶ account_file: {"accounts":[{"discord_id","refresh_token"},…]}, mode 600
            ─▶ staging.load_population_seeded audit row
```

The services commit one by one, so a failure at any step deletes every account of the range
(`population_cleanup::delete_synthetic_accounts`) and the file the run wrote, and exits 1 naming
what happened. The account file is the format `account_rotation.rs` of the developer_tools load
engine decodes: account `k` at index `k`, which also fixes its fixture event (`k mod 10`) and slot
(`k div 10`).

A cleanup counts, then deletes in one transaction and in this order: the registrations of reserved
accounts with their history and participation rows (those restrict the delete of a registration, and
a registration restricts the delete of its quota allocation), their fire missions and link codes (no
foreign key ties them to the account), and the accounts, whose sessions, tokens, snapshots, Discord
roles, reservations, allocations and bookmarks cascade and whose slot assignments clear. The
reservation consistency checks run at the commit and find no synthetic participant left. It appends
a `staging.load_population_cleaned` audit row; audit rows naming the accounts stay.

## Boundaries

- Depends on: `crate::{argument_list, guarded_context, reserved_accounts, secret_files,
  tool_failure}`; the api crates: `api_identity_and_access::services::{account_registration,
  discord_membership_cache, session_issuance}`, `api_discord::discord_client::GuildMember` and
  `api_audit_log::required_audit`; `serde` and `serde_json` for the account file.
- Used by: the subcommand table in `tools/staging/staging_fixtures/src/main.rs`; the
  `staging load` harness, which runs both subcommands on the host; the fixture events of
  `load_fixture_events/`, authored by the population's first account.
- Rules: every id lies in the reserved range (`SyntheticIdRange`); a seeding runs before `seed-load-fixture-events` and a cleanup
  after `clean-load-fixture-events`; the account file keeps exactly the keys `discord_id` and
  `refresh_token`; no token reaches stdout or stderr.

## Related documentation

- [Staging acceptance](/documentation/crates/api/api_server/design_notes/staging.md) — the
  load procedure the population serves.
- [Staging fixtures crate](/tools/staging/staging_fixtures/README.md#staging-fixtures) — the subcommands'
  flags and exit codes.
