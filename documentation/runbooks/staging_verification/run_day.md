**Status:** live

# Record the staging receipts

The run day records the three staging receipts: the load run first (before the Discord bot token
exists), then the fleet run, then the Discord run, each from the repository root.

## Prerequisites

- The [setup checklist](/documentation/runbooks/staging_verification/setup_checklist.md) is done
  and `cargo xtask staging preflight` names no unmet precondition for the load and fleet runs.
- The operator is available for the fleet run (joining server 1, typing the link code, being kicked)
  and for the Discord bot token.

## Steps

1. The operator adds the four secondary source addresses to the workstation's interface.

   ```bash
   sudo ip addr add 192.168.0.240/24 dev enp4s0
   ```

   Expected: repeated for .241, .242 and .243; `staging preflight` shows all five source addresses.

2. Record the load receipt: backup, seeding, keying probe, 30 measured minutes, cleanup.

   ```bash
   cargo xtask staging action-list load
   ```

   Expected: the numbered actions the operator approves; then `cargo xtask staging load --record
   --token-file <scratchpad>/load_tokens` runs and prints `staging_load: PASS <n>/10` or the failing
   cases with their reasons. The synthetic accounts are gone afterwards and the token file is
   deleted.

3. Record the fleet receipt ([fleet procedure](/documentation/runbooks/staging_verification/fleet_procedure.md)).

   ```bash
   cargo xtask staging action-list fleet
   ```

   Expected: the numbered waves the operator approves; `cargo xtask staging fleet --record` prints
   an `AWAIT` line per wave, and the browser action follows each. With one game client the run ends
   `staging_fleet: FAIL` naming the second client as missing.

4. The operator sets `DISCORD_BOT_TOKEN` in the API's `.env` on the host; the API restarts and the
   operator's membership reconciles.

   ```bash
   cargo xtask staging preflight --discord
   ```

   Expected: the bot reads both guilds and the operator's snapshot is fresh.

5. Record the Discord receipt.

   ```bash
   cargo xtask staging action-list discord
   ```

   Expected: the numbered steps the operator approves; `cargo xtask staging discord --record` runs
   them. With no subject account it ends `staging_discord: FAIL` naming the missing account.

## Verify

The receipts and their logs are in `target/staging/receipts/`, the raw journals in
`target/staging/<check>/<run>/`. Record the verdict, counts, timestamps and paths in the results
section of the
[staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md).

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| 429 answers during the load run | a source address is missing or shared | check the addresses in `staging preflight`; never add traffic from the same addresses |
| a fleet wave reports `FAILED (deadline)` | the command was not issued, or the server did not reach the state | read the wave's journal entry; the receipt keeps the failure |

## Related

- [Recovery and cleanup](/documentation/runbooks/staging_verification/recovery_and_cleanup.md) —
  when a run stops early.
- [Staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md) — what
  each step proves.
