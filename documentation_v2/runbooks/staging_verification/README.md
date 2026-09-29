**Status:** live

# Staging verification

How to record the three operational receipts of the API acceptance register (`staging_fleet`,
`staging_discord`, `staging_load`) against the staging host: preparing the five-instance fleet and
the Discord guilds once, running the three procedures inside one 24-hour window, and handing the
receipts to `cargo xtask verify api-readiness`. The design, the evidence rules and the case lists are
in the [staging design note](/documentation_v2/website/api_v2/verification_evidence/staging.md).

## Contents

```text
documentation_v2/runbooks/staging_verification/
├── discord_procedure.md       the Discord receipt: preflight, the AWAIT steps, what each case proves, recovery
├── fleet_procedure.md         the fleet receipt: preflight, approval, the waves at each AWAIT line, the verdict
├── load_procedure.md          the load receipt: seeding, keying, 30 measured minutes, cleanup, local rehearsal
├── recovery_and_cleanup.md    undo a stopped run: synthetic accounts, drop-in, relay, addresses, backups
├── run_day.md                 the 24-hour sequence: load, fleet, Discord, then the readiness verdict
└── setup_checklist.md         one-time preparation of the host, the fleet, the guilds and the content
```

## How it works

```text
workstation (repository, hcargo)                 staging host dooley
────────────────────────────────                 ───────────────────
cargo xtask staging <procedure> --record ─ ssh ─▶ read-only psql, journals, console logs, /metrics
   │  AWAIT <step> lines                            staging-fixtures (seeding, aging, bot reads)
   ▼                                                tbd-reforger@1..5, fleet-host-agent@1..5
orchestrator in the operator's browser ─ https ─▶ Server Control, events, Discord
load engine (5 source addresses) ─ http :3080 ──▶ Caddy ─▶ API :8080
   │
   ▼
target/api-readiness/<check>.{json,log,fixture.json}  ──▶  cargo xtask verify api-readiness
```

| Order | Runbook | When |
|---|---|---|
| 1 | [Setup checklist](/documentation_v2/runbooks/staging_verification/setup_checklist.md) | once, after the tooling is committed |
| 2 | [Run day](/documentation_v2/runbooks/staging_verification/run_day.md) | one 24-hour window per receipt set |
| 3 | [Recovery and cleanup](/documentation_v2/runbooks/staging_verification/recovery_and_cleanup.md) | when a run stops early or leaves state behind |

Rules every run follows:

- Every command runs as `hcargo xtask …` from the repository root, so the receipts and the later
  verification see the same PATH and CARGO_TARGET_DIR.
- Nothing edits the repository, the `.env` files or `tools_v2/xtask/deploy/deploy.env` from the first
  recording until `verify api-readiness --execute` has finished; the three receipts and that run fit
  in 24 hours of the first recording.
- The operator approves each procedure run over its numbered action list
  (`cargo xtask staging action-list <procedure>`); anything off the list stops and asks.
- Witnessing is automated: a fact without machine evidence is recorded as missing.
- A dependency that is not available (a second game client, a Discord test account) is named in the
  receipt and its cases are recorded as not run; the receipt then fails honestly.

## Related documentation

- [Staging design note](/documentation_v2/website/api_v2/verification_evidence/staging.md) — the
  procedures, the receipt format and the witness rules.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging host
  and its deploy.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — the API, the proxy and the
  tunnel on the staging host.
