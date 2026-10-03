# Staging Discord procedure

The `staging_discord` procedure: partner membership, the loss of the partner role, the outage with
its cached grace and staleness warning, the administrator override, recovery, and the rate limit,
observed against the real guilds. The steps and cases are in the
[staging design note](/documentation/apps/api/verification_evidence/staging.md#discord-procedure-staging_discord).

## Contents

```text
tools/commands/staging_procedures/src/discord_procedure/
├── discord_cases.rs           the thirteen declared cases, two not run, and the scenario mapping
├── membership_queries.rs      the probes' targets and committed SELECTs over snapshots, events, audit
├── mod.rs                     `DiscordProcedure`: plan, identities, staged precondition, preflight
├── operator_actions.rs        the numbered action list and the recovery list
├── outage_steps.rs            steps 3-6: drop-in, outage page reads, aged snapshot, override, recovery
├── partner_steps.rs           steps 1-2 with the 60-second measure, and the shared probe builders
├── rate_limit_steps.rs        step 7 (baseline, bucket spend timed on the host) and step 8 (cleanup)
├── reconciliation_readers.rs  the API's `discord_reconciliation` log lines and outcome counter
└── tests/                     recorded runs on the fake clock, planted defects, receipt, lists
```

## How it works

`DiscordProcedure` implements `StagingProcedure` from
`tools/commands/staging_procedures/src/procedure_runner/procedure.rs`. The plan runs eleven steps:
`partner_registration`, `partner_role_removal` (its request is the first bot read showing the role
gone, and `release_within_60_seconds` counts 60 s from it), `outage_install`, `outage_page_reads`,
`snapshot_aging` (the staged 49 h precondition), `grace_override`, `outage_removal`,
`partner_role_restore`, `rate_limit_baseline`, `bucket_spend` (the host reads `next_refresh_at`
and starts the spend 300 ms before it) and `partner_event_deletion`. Browser steps read their
inbox entry; the others read the database, `/metrics`, the API unit's journal and the bot's
member read, parsed by `remote_observers/discord_member_reader.rs`. The receipt's environment
lists the staged precondition as `staged_precondition=membership_snapshot_aged_49h`. The operator
procedure is the runbook
[staging Discord procedure](/documentation/runbooks/staging_verification/discord_procedure.md).

## Boundaries

- Depends on: `tools/commands/staging_procedures/src/procedure_runner/`, the outage drop-in and the
  host tool's aging, member read and bucket-spend commands in `remote_actions/`, the observers in
  `remote_observers/`.
- Used by: `tools/commands/staging_procedures/src/staging_dispatch.rs`.
- Rules: the staged precondition (the aged snapshot) is recorded as staged, never as elapsed time;
  no probe or preflight check prints the bot token.
