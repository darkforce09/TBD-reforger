**Status:** live

# API v2 verification checkpoint — 2026-09-26

Milestones E, F, M and T are implemented and verified; overall readiness is **not passing**,
because milestones C, B, V and S have not started (see `remaining_milestones.md`) and
`cargo xtask verify api-readiness --execute` was not run (it needs a quiet working tree). T is
committed on main as 0ef292758. Do not reset, clean, stash or revert anything in the working tree:
it holds another agent's uncommitted work (equipment/vehicle export and the equipment data viewer in
`tools_v2/xtask`, `apps/website/api_v2/src/community_content`, the frontend `data_viewer`,
`contracts_v2/definitions/equipment-*` and `assets_v2/equipment`).

## Milestone status

| Milestone | State |
|---|---|
| E — eligibility, allocation, occupancy, no-show, machine credentials, runtime sessions | Complete. |
| F — fleet command ledger, host agent, executors, recovery | Complete. |
| M — artifacts, reviews, approval binding, deployments, authored preservation, workspace | Complete. |
| T — match identity, revisions, corrections, atomicity, detailed events, telemetry queue, fleet dashboard, statistics | Complete (design: `telemetry.md`). |
| C, B, V, S | Not started (`remaining_milestones.md`). |

Migrations 0054–0056 are pinned; the next migration is 0057. Versions 0022–0024 stay retired.

## T in brief

- Every game-server route takes the server's machine credential; `ServiceAuth`, `SERVICE_TOKEN`
  and `X-Service-Token` are gone. `/metrics` and the detailed `/healthz` take the operator's
  `OBSERVABILITY_TOKEN`. `/api/v1/ingest/*` is on the global rate tier.
- Matches are registered per server before any report; results are numbered revisions with
  payload digests; event batches (seven kinds) are validated whole and counted once.
- The mod keeps outbound telemetry in a durable, bounded, checksummed queue under
  `$profile:TBD/Telemetry/`; heartbeats and server status carry its backlog, drops and oldest age.
- The dashboard answers the configured fleet (`servers.is_active`) with totals; the publisher and
  member reads skip inactive servers.
- Operator decisions of 2026-09-26 are recorded in `telemetry.md` (corrections merge per line with
  `removed_lines`; flat counter keys retired; T-1092's modularised mod layout used as is; events
  surface as a read route and DTO only).

## Verification (logs in `target/api-progress-checkpoint/2026-09-26/`, gitignored)

| Gate | Result |
|---|---|
| `cargo xtask db test-it` (full) | 877 passed, 0 failed (`g-db-test-it.log`); T prefixes: match_identity 14, telemetry_revisions 7, telemetry_corrections 7, telemetry_atomicity 6, detailed_events 9, telemetry_queue 6, fleet_dashboard 6, statistics_recomputation 6 |
| `cargo test -p website-api --lib` | 395 passed |
| Clippy `-D warnings` | website-api all targets clean; xtask clean |
| rustfmt | clean on every file of this milestone (other agent's files not formatted) |
| Frontend lane | fmt step stops on the other agent's `data_viewer` files; clippy wasm32 exit 0; `cargo test -p website-frontend` 1,602 passed, 1 failed (`doc_audit`, only the other agent's `data_viewer` files); trunk release build passes |
| `mk leptos-gates` | editor suite 21/21; DOM oracle 25/25 after auditing and accepting `dashboard` (fleet list) and `servercontrol` (telemetry queue column, Inactive badge) |
| `ci ci-local-schema` / codegen freshness | PASS |
| `verify file-length` / `enfusion-comments` | 0 violations (3,829 files) / 0 findings |
| `verify route-tags` | 147 routes all tagged; FAIL only on 12 unwired equipment-viewer tags of the other agent |
| Documentation gates | markdown-placement OK; readme-coverage and link-check fail only on the other agent's equipment and improved_layout folders and the T-1092 checkpoint link |
| `mod compile` | clean |
| `mod world-boot` bare / `--compiled` | PASS / PASS (roll-call includes `MatchTelemetry=ok`; four-weapon equip) |
| `mod playtest --mission=6d0af8b0-… --timeout=420 --require-telemetry` | deployment CONFIRMED; heartbeat telemetry queue 0/512, dropped 0; pre-stop drain ran on timeout; credential revoked. No round went LIVE (0 players), so registration, events and results were not exercised in-engine |
| `cargo test -p xtask` | 1,043 passed, 4 failed: 3 in the other agent's files, 1 (`every_rust_file_named_in_prose_exists`) because this milestone's new tooling files are untracked until staged |
| `cargo test -p developer-tools` / `-p fleet-host-agent` | 265 passed, 4 ignored / 127 passed |
| `ci ci-local` | stops at step 1 (`verify-editorconfig`): 3,205 findings, all in the other agent's untracked `assets_v2/equipment/`; the later steps were run individually above |

## Open items

- In-engine exercise of registration, events and results needs a LIVE round with a connected
  admin client (the two-client playtest runbook); the API side is covered by the integration
  suites.
- T-1222 is marked shipped (`ticket check` stays red until `cargo xtask ticket stamp-sha T-1222
  <sha>` after the commit). T-940.13 (detailed events) is implemented; ship it with the commit.
- `apps/website/frontend/tests/fixtures/api/GET__dashboard.json` keeps its pre-existing
  `next_event`/`my_assignment` (the seed's dates are past rule 4); the V milestone re-baselines the
  goldens.
- The dashboard `fleet` and `MatchEventPage` shapes cite `match-telemetry.schema.json`; a dashboard
  response schema is part of V's contract parity.
- Then milestones C, V and S, with a quiet working tree for `verify api-readiness --execute`;
  game ballistics (B) follows in its own later phase by operator decision.
