**Status:** live

# Staging load procedure

How to record the `staging_load` receipt: 1,100 synthetic members seeded on the staging host, 30
measured minutes of member load from five workstation addresses through Caddy on the LAN, and the
game operations measured beside it; then the cleanup. The cases and the measurement rules are in
the
[staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md#load-procedure-staging_load);
the committed workload and population are in `tools/xtask/staging/`.

## Before you start

- `deploy/deploy.env` names `TBD_LOAD_TARGET_ORIGIN` (the Caddy listener on the LAN,
  plain HTTP) and the five `TBD_LOAD_SOURCE_ADDRESSES`, the first being the workstation's own.
- The host's API env file has an empty `DISCORD_BOT_TOKEN` and a `TRUSTED_PROXIES` that covers
  `127.0.0.1` (Caddy) and none of the source addresses.
- The live mission "TBD Staging Everon" exists, the five fleet servers heartbeat, and no synthetic
  account or `[Load fixture]` event is left (`cargo xtask staging status`).
- Pick a token file path in a private folder, for example `<scratchpad>/load_tokens`; it must not
  exist yet.

## Steps

1. Read the numbered actions and have the operator approve them.

   ```bash
   cargo xtask staging action-list load
   ```

   Expected: backup, the secondary addresses, preflight, `seed-load`, the recording, `clean-load`,
   the token file's deletion and the addresses' removal, each with its exact command.

2. Back up the database, add the four secondary addresses, and check the preconditions.

   ```bash
   cargo xtask staging backup --label before-load
   cargo xtask staging preflight
   ```

   Expected: the backup names its file; every `load:` check is met, including one invalid refresh
   per source address answered 401 and one `strict|<address>` bucket per address.

3. Seed the population and the fixture events.

   ```bash
   cargo xtask staging seed-load --token-file <scratchpad>/load_tokens
   ```

   Expected: the staging mission's id, the host tool's lines for 1,100 accounts and ten fixture
   events, then `token file: <path> (1100 accounts, mode 600)`; the host copy is removed. Add
   `--dry-run` first to see the plan without connecting.

4. Record the receipt.

   ```bash
   cargo xtask staging load --record --token-file <scratchpad>/load_tokens
   ```

   Expected: an `AWAIT` line and effect lines for `population`, `keying_probe`,
   `game_operations_baseline`, `member_load` (about 31 minutes, a census read every minute) and
   `game_operations_delta`, then `staging_load: PASS 10/10` or `staging_load: FAIL <ok>/10` with
   each failing case's reason. The receipt is in `target/staging/receipts/staging_load.{json,log,fixture.json}`.

5. Clean up.

   ```bash
   cargo xtask staging clean-load
   rm -f <scratchpad>/load_tokens
   ```

   Expected: the fixture events go first, then the population, then any host copy of the account
   file; `cargo xtask staging status` shows zero synthetic accounts and zero fixture events. Then
   the operator removes the secondary addresses.

## Rehearsal on the local stack

`cargo xtask staging load --rehearse-local` runs the same load path against the local API on
`http://127.0.0.1:8080` (started as in the
[local development runbook](/documentation/runbooks/local_development.md), with the seeds applied
and one live mission): it seeds the population and the fixture events into the local database with
`staging-fixtures`, keys 127.0.0.2 to 127.0.0.6, runs the committed workload for the shortest
ramp the sign-ins allow (50 s for 100 clients on five addresses at 80 % of the auth ceiling) and 60
measured seconds, judges the member-load cases against thresholds scaled to that short window (the
recorded run keeps the acceptance thresholds), and always cleans the fixtures and the account
file. It records nothing, and the two game-operation cases are not rehearsed.

## When a run stops

```bash
cargo xtask staging action-list load --recovery
```

The recovery list is `cargo xtask staging clean-load`, then deleting the token file. Details are in
[recovery and cleanup](/documentation/runbooks/staging_verification/recovery_and_cleanup.md).
