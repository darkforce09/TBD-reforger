**Status:** live

# Record the Discord receipt

How the orchestrator and the operator run `cargo xtask staging discord --record`: the checks
before it, the approval of its numbered actions, what to do at each `AWAIT` line of its eight
procedure steps, what each case proves, and how to put everything back after a stopped run. The
design of the procedure is in the
[staging design note](/documentation/website/api_v2/verification_evidence/staging.md#discord-procedure-staging_discord).

## Prerequisites

- The [setup checklist](/documentation/runbooks/staging_verification/setup_checklist.md) is
  complete: the partner guild exists with its partner role and invites, the bot is a member of the
  main and the partner guild, and the operator holds the partner role.
- `tools/xtask/deploy/deploy.env` sets `TBD_STAGING_OPERATOR_DISCORD_ID`,
  `TBD_STAGING_PARTNER_GUILD_ID`, `TBD_STAGING_PARTNER_ROLE_ID` and `TBD_STAGING_DB_CONTAINER`; a
  missing key refuses the plan before the recording begins.
- The load and fleet receipts of the same 24-hour window are recorded
  ([run day](/documentation/runbooks/staging_verification/run_day.md)), and the API env file on
  the host sets `DISCORD_BOT_TOKEN`.
- The pre-Discord backup is taken:

  ```bash
  hcargo xtask staging backup --label pre-discord
  ```

- Preflight passes with the Discord checks, which only read: the bot token key set (its value is
  never read), the bot's read of the operator in the main guild, the bot's read in the partner guild
  with the partner role held, the operator's main-guild snapshot verified within 60 s with no error,
  and a pre-Discord backup from the last 24 hours:

  ```bash
  hcargo xtask staging preflight --discord
  ```

## Steps

1. Print the numbered action list and have the operator approve it; anything off the list stops
   and asks:

   ```bash
   hcargo xtask staging action-list discord --cases
   ```

2. Start the recording in the background from the repository root and follow its `AWAIT` lines; it
   never reads stdin, and every browser read goes to the `browser_inbox/<step>.json` path it prints,
   saved unchanged with its capture time:

   ```bash
   hcargo xtask staging discord --record
   ```

3. `partner_registration`: create the partner-only event "[Discord staging] Partner event" with its
   partner group, register the operator, save the refused answer
   (`MEMBERSHIP_VERIFICATION_REQUIRED`) in the inbox, then register again.
4. `partner_role_removal`: in Discord, remove the partner role from the operator in the partner
   guild. The harness polls the bot's read until the role is gone.
5. `outage_install` and `outage_page_reads`: the harness installs the `HTTPS_PROXY` drop-in and
   restarts the API; once the staleness banner shows, save the `/api/v1/me` response body, the
   banner text and the `/api/v1/events` request line with its status in one inbox entry.
6. `snapshot_aging` and `grace_override`: the harness ages the operator's main-guild snapshot to
   49 h with `staging-fixtures age-membership-snapshot`; save `/api/v1/me` showing the guest role,
   then grant the override from the banner form (Extend for 48 hours) with a reason and save
   `/api/v1/me` again.
7. `outage_removal` and `partner_role_restore`: the harness removes the drop-in and restarts the
   API; in Discord, give the partner role back to the operator.
8. `rate_limit_baseline` and `bucket_spend`: the harness reads the rate-limited count, then spends
   the main guild's Get Guild Member bucket from 300 ms before the operator's `next_refresh_at`,
   read on the host, holding 3 s with at most 50 requests.
9. `partner_event_deletion`: delete the partner event.

## What each case proves

| Case | Proven by |
|---|---|
| `partner_membership` | the refused registration names `MEMBERSHIP_VERIFICATION_REQUIRED`, the partner snapshot is a verified membership, the retry registers, the site role stays admin |
| `eligibility_release` | the reservation is released `eligibility_lost` and an `event.reservation_released` audit row follows |
| `partner_role_propagation_within_60_seconds` | the release's `withdrawn_at` is at most 60 s after the first bot read that shows the role gone |
| `network_outage` | during the drop-in the API logs and counts `unavailable` main-guild reconciliations, and the snapshot keeps its membership with `last_error` set |
| `staleness_warning` | `/api/v1/me` reports the membership stale and the banner shows |
| `cached_grace` | the cached admin role still applies while stale |
| `non_blocking_during_outage` | `/api/v1/events` answers 200 during the outage |
| `admin_override` | the snapshot is staged 49 h old (recorded as staged, never as elapsed time), `/api/v1/me` shows a guest, the override row lasts 48 h, a `membership.grace_extended` audit row exists, and `/api/v1/me` shows the override active with the admin role |
| `outage_recovery` | after the drop-in comes off the snapshot is verified again, and the platform's copy of the partner roles holds the role again |
| `rate_limit` | the API logs a `rate_limited` main-guild reconciliation and `tbd_discord_reconcile_outcomes_total{outcome="rate_limited"}` rises above the count read before the spend |
| `rate_limit_recovery` | a verification follows the logged rate limit |
| `role_demotion`, `departure_to_guest` | not run: they need a test subject Discord account other than the operator's |

## Verify

- The receipt ends `staging_discord: FAIL 11/13` while no test subject account exists: it exits
  1, carries no success marker, lists `missing: test subject Discord account`, and still carries
  every real observation.
- Its environment lists `staged_precondition=membership_snapshot_aged_49h`, and its fixture
  manifest records the staged aging in `staged_preconditions`.
- The receipt and its log land in `target/api-readiness/`; the journal and the inbox archive land
  under `target/staging/staging_discord/<run>/`.

## Recovery

A run that stops early leaves the drop-in, a removed partner role or the partner event behind.
Print the recovery list and run it as approved actions:

```bash
hcargo xtask staging action-list discord --recovery
```

1. The harness removes the `HTTPS_PROXY` drop-in and restarts the API; `hcargo xtask staging
   status` then shows the drop-in absent.
2. The orchestrator gives the partner role back to the operator in the partner guild.
3. The orchestrator deletes the partner event if it still exists.

The rest of the cleanup is in
[recovery and cleanup](/documentation/runbooks/staging_verification/recovery_and_cleanup.md).

## Related

- [Staging design note](/documentation/website/api_v2/verification_evidence/staging.md) — the
  procedures, the receipt format and the witness rules.
- [Run day](/documentation/runbooks/staging_verification/run_day.md) — where the Discord run sits
  in the 24-hour window.
