**Status:** live

# Prepare the staging host, fleet and guilds

One-time preparation before the first run day: the Experimental dedicated server updated, the
website and five game-server instances deployed, their credentials provisioned on the host, the
Discord partner guild and the bot in place, and the content the procedures deploy. Every step that
changes the staging host is approved by the operator first, one numbered list per step.

## Prerequisites

- The staging tooling is committed and gated (`git status --short` shows only the two Workbench
  `resourceDatabase.rdb` files).
- `tools/xtask/deploy/deploy.env` names the host and the fleet, and none of the retired
  single-server keys (the deploy refuses them)
  ([staging deploy settings](/documentation/runbooks/game_server_staging/staging_deploy.md)).
- The operator's Chrome runs the Claude extension, signed in to the site as an administrator and to
  Discord.
- The operator's game client is on the Experimental branch; Workbench is closed on run day.

## Steps

1. Take a database backup on the host.

   ```bash
   cargo xtask staging backup --label pre-setup
   ```

   Expected: a verified `pg_dump -Fc` under `~/tbd/backups/<date>/` named in the output.

2. Update the Experimental dedicated server and record its build.

   ```bash
   cargo xtask staging update-game-server
   ```

   Expected: steamcmd `validate` succeeds and the output names the app 1890870 build id.

3. Deploy the website: migrations, the API, the host tools and the proxy.

   ```bash
   cargo xtask deploy website
   ```

   Expected: the deploy completes; `staging-fixtures` and `acknowledgement-dropping-relay` are
   built on the host.

4. Register the five servers and write their credentials on the host.

   ```bash
   cargo xtask staging provision-fleet
   ```

   Expected: five servers "TBD Staging 1" … "TBD Staging 5"; ten credential files of mode 600 under
   `~/tbd/fleet/instance-N/secrets/`; no secret printed. The operator writes the servers' join
   password into `~/tbd/fleet/join-password` (mode 600) by hand.

5. Deploy the five instances, retiring the single-server units; the deploy refuses an instance
   without its credential files.

   ```bash
   cargo xtask deploy staging --migrate-single-instance
   ```

   Expected: `tbd-reforger@1` … `@5` and `fleet-host-agent@1` … `@5` active, the relay unit active
   for instance 5, and each instance's console log showing `session-started`.

6. In the browser: deactivate "TBD Staging POC"; create the partner guild "TBD Staging Partner" with
   the role "Partner Member" held by the operator; invite the bot to both guilds with no
   permissions; author, submit and approve the missions "TBD Staging Everon" and
   "TBD Staging Arland"; upload the vanilla ballistics catalog; add the fleet scenario rows for
   `everon` and `arland`.

   Expected: `cargo xtask staging status` lists five active servers, two live missions with
   approved artifacts, two fleet scenario rows and one ballistics catalog.

   Then deploy "TBD Staging Everon" to each of the five servers in Server Control, so every
   instance runs an Everon deployment before the fleet run (its same-terrain wave needs one).

7. Run the capacity trial: all five instances idle for 10 minutes, then one with the operator
   connected.

   ```bash
   cargo xtask staging status --capacity
   ```

   Expected: free memory, load average and each unit's memory recorded; if the host cannot carry
   five instances, stop and ask the operator.

8. Boot one instance on the Arland mission header and read its verdict.

   ```bash
   cargo xtask mod remote-logs --instance 1
   ```

   Expected: `VERDICT: PASS`.

9. Write the partner guild id, its role id and the operator's Discord id into `deploy.env`, then
   check the tree and the fingerprints.

   ```bash
   cargo xtask staging fingerprints
   ```

   Expected: the source and configuration digests printed; they must not change until the
   readiness verdict of the run day.

## Verify

```bash
cargo xtask staging preflight
```

Expected: every precondition of the three procedures listed as met, or each unmet one named.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| an instance exits right after start | two instances share a port | check `TBD_FLEET_*_PORT_BASE`; A2S and game ports must differ |
| `provision-fleet` refuses | a secret file already exists | secret files are never overwritten; remove the instance's `secrets/` only after revoking its credentials |
| the bot cannot read members | the bot is not in the guild | invite it again to that guild |

## Related

- [Run day](/documentation/runbooks/staging_verification/run_day.md) — the procedures that follow.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the host and its
  deploy.
