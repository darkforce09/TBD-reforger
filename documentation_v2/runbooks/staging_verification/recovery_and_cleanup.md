**Status:** live

# Recover from a stopped staging run

A staging procedure changes the staging host on purpose: synthetic accounts, fixture events, an
outage drop-in on the API unit, an armed relay, secondary source addresses. When a run stops early,
put each of them back before anything else runs. Every command here reads first and changes the host
only after the operator approves.

## Prerequisites

- The run's journal under `target/staging/<check>/<run>/`, which names the last step reached.
- The backups taken before the run (`~/tbd/backups/<date>/`).

## Steps

1. See what the run left behind.

   ```bash
   cargo xtask staging status
   ```

   Expected: the synthetic account count, the fixture events, the drop-in state, the relay state and
   the fleet units, each with its expected resting value.

2. Remove synthetic accounts and fixture events left by a load run.

   ```bash
   cargo xtask staging clean-load
   ```

   Expected: zero synthetic accounts and zero `[Load fixture]` events afterwards. Do this before a
   Discord bot token is set, or the reconciler asks Discord about every synthetic account.

3. Remove an outage drop-in left by a Discord run and restart the API: the `HTTPS_PROXY` drop-in of
   `tbd-website-api` is deleted and the unit restarted as one approved action of the Discord
   procedure's recovery list.

   ```bash
   cargo xtask staging action-list discord --recovery
   ```

   Expected: the list includes the drop-in removal; after it, `staging status` shows no drop-in and
   a fresh membership snapshot.

4. Disarm the relay of instance 5 if a fleet run stopped between arming and the restart.

   ```bash
   cargo xtask staging action-list fleet --recovery
   ```

   Expected: the list includes the relay disarm; `staging status` then shows it disarmed.

5. The operator removes the secondary source addresses.

   ```bash
   sudo ip addr del 192.168.0.240/24 dev enp4s0
   ```

   Expected: repeated for .241, .242 and .243.

6. Only when cleanup cannot restore the database, restore the backup taken before the run, with
   the operator's approval ([database operations](/documentation_v2/runbooks/database_operations.md)).

## Verify

```bash
cargo xtask staging status
```

Expected: every item at its resting value; the stopped run's receipt stays as it was written (a
failure is evidence too) and the next recording replaces it.

## Related

- [Run day](/documentation_v2/runbooks/staging_verification/run_day.md) — the procedures.
- [Setup checklist](/documentation_v2/runbooks/staging_verification/setup_checklist.md) — the resting
  state of the host.
