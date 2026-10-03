# Staging fixtures host tool

The `staging-fixtures` binary: the tool that stages the fixtures of a staging verification run in
the staging database and on the staging host, behind one set of guards. Its commands are documented
in the [API executables README](/apps/api/src/bin/README.md#staging-fixtures).

## Contents

```text
apps/api/src/bin/staging_fixtures/
├── argument_list.rs        splits the flags after the subcommand name and refuses any nobody takes
├── credential_rotation.rs  `rotate-credential`: stages a new credential file, then promotes it
├── discord_member_probe/   `observe-discord-member` and `spend-discord-member-bucket`: bot reads of a member
├── fleet_provisioning.rs   `provision-fleet`: registers the fleet servers and writes their credentials
├── guarded_context.rs      the run mode, the API env file values and the administrator actor check
├── load_fixture_events/    `seed-load-fixture-events` and `clean-load-fixture-events`: the load run's events
├── load_population/        `seed-load-population` and `clean-load-population`: the load run's accounts
├── main.rs                 the subcommand table, the guards and the exit codes
├── membership_aging.rs     `age-membership-snapshot`: stages a main-guild snapshot as verified hours ago
├── reserved_accounts.rs    the Discord id range reserved for synthetic staging accounts
├── secret_files.rs         the per-instance secret layout, exclusive mode-600 writes and promotion
├── tests/                  unit tests for the argument list and the reserved range
└── tool_failure.rs         refused (exit 2) and failed (exit 1), and their messages
```

## How it works

`main.rs` holds the subcommand table: each row names a subcommand, its flags and the parser that
takes them. A run goes through the same steps whatever the subcommand:

```text
argv ─▶ ArgumentList ─▶ --apply, --confirm-database, --api-env-file taken
     ─▶ the row's parser takes its flags and validates them ─▶ ArgumentList::finish
     ─▶ ApiEnvironment::read(API env file) ─▶ database::connect(DATABASE_URL)
     ─▶ current_database() == --confirm-database ─▶ GuardedContext
     ─▶ census of reserved accounts ─▶ the subcommand, in RunMode::DryRun or RunMode::Apply
```

Nothing connects before every flag has parsed, and nothing writes before the database confirmation.
A dry run performs every read-only check an apply performs and prints the plan. The fleet
subcommands act for `--actor`, whom `GuardedContext::require_administrator_actor` accepts only
outside the reserved range and holding administrator authority in the `DISCORD_GUILD_ID` guild of
the API env file, as the API judges an administrator without a session.

`fleet_provisioning.rs` and `credential_rotation.rs` write rows through the services the
administrator routes use (`server_registration::register_server` and
`machine_credentials::issue_machine_credential`, each with its audit row) on one transaction, and
commit only after `secret_files.rs` has created every secret file with `O_EXCL` and mode 600 and
synced it; on a failure they roll back and remove the files they wrote. A promotion is one rename of
the staged file over the live one.

`load_fixture_events/` creates the ten `[Load fixture]` events of a load run through
`event_authoring::event_creation` and `event_authoring::mission_attachment`, authored by the load
population's first account, and removes them with their registrations before that population goes.

`load_population/` seeds that population, the synthetic members of the reserved range, through the
services the API's own sign-in uses (`account_registration::register_account`, the membership
lease and observation of `discord_membership_cache`, and `session_issuance::issue_refresh`), only
while `DISCORD_BOT_TOKEN` is unset and no reserved account exists, and writes their refresh tokens
to the load engine's account file through `secret_files.rs`; its cleanup deletes the accounts and
every row they own, audit rows excepted. `membership_aging.rs` stages the operator's main-guild
snapshot as verified hours ago, under the account lock, for the Discord procedure's grace step.
`discord_member_probe/` reads one guild member with the bot token of the API env file, once or
until the Get Guild Member bucket is spent, never more than 50 requests, and prints each read as a
`discord-member-read` JSON line; its test-only `--discord-api-base` accepts a loopback address
alone, so the token goes nowhere but Discord.

Every message names paths, ids and flags only: the API env file's values, the secrets and the
database URL never reach stdout or stderr, and `ApiEnvironment`'s `Debug` prints keys only.

## Boundaries

- Depends on: the `api` library: `core::database`, `core::authentication_primitives`,
  `core::error_handling`, `server_infrastructure::services::{server_registration,
  machine_credentials}` and `identity_and_access::services::account_authority`,
  `operations::services::event_authoring::{event_creation, mission_attachment}`,
  `identity_and_access::services::{account_registration, discord_membership_cache,
  session_issuance, discord_client}` and `administration::services::required_audit`;
  `fleet_wire_contract` for `ExecutorKind` and the secret-file limits the host agent reads its
  credential files under; `dotenvy` for
  the API env file, `reqwest` and `rustls` for the Discord reads, `tracing-subscriber` for the
  services' error logs on stderr.
- Used by: the operator on the staging host;
  `apps/api/tests/staging_fixtures_fleet.rs`,
  `apps/api/tests/staging_fixtures_fixture_events.rs`,
  `apps/api/tests/staging_fixtures_population.rs` and
  `apps/api/tests/staging_fixtures_discord.rs`, which run the built binary.
- Rules: every subcommand is a row of `SUBCOMMANDS` whose parser takes all its flags before any
  guard runs; a subcommand writes only in `RunMode::Apply`; synthetic accounts take their ids from
  `reserved_accounts.rs` and nothing deletes outside that range; secret files are created only
  through `secret_files.rs`; test functions start with `staging_fixtures_`.

## Related documentation

- [API executables](/apps/api/src/bin/README.md) — the synopsis, flags and exit codes of
  each subcommand.
- [Server infrastructure services](/apps/api/src/server_infrastructure/services/README.md)
  — the registration and credential services the fleet subcommands write through.
