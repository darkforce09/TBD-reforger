# API v2 verification checkpoint — 2026-09-23

Milestones E, F and M are implemented and verified; overall readiness is **not passing**, because
milestones T, C, B, V and S have not started (see `remaining_milestones.md`) and the operator
chose to skip `cargo xtask verify api-readiness --execute` this round. All work is uncommitted on
main. Do not reset, clean, stash or revert anything in the working tree: it also holds other
people's uncommitted work (another agent is editing `tools_v2/xtask` equipment export, `cli`,
`dispatch` and `mod_ops/mod`).

## Milestone status

| Milestone | State |
|---|---|
| E — eligibility, allocation, occupancy, no-show, machine credentials, runtime sessions | Complete. |
| F — fleet command ledger, host agent, executors, recovery | Complete. |
| M — artifacts, reviews, approval binding, deployments, authored preservation, workspace | Complete. |
| T, C, B, V, S | Not started (`remaining_milestones.md`). |

Migrations 0042–0053 are pinned; the next migration is 0054. Versions 0022–0024 stay retired.

## Verification (logs in `target/api-progress-checkpoint/2026-09-23/`, gitignored)

| Gate | Result |
|---|---|
| `cargo xtask db test-it` (full) | 784 passed, 0 failed |
| Frontend lane (`mk ci-local-leptos`) | 1,568 passed; 1,578 after the profile-poll fix |
| `mk leptos-gates` | editor suite 21/21; DOM oracle 25/25 (7 E/F/M routes accepted with notes: approvals, servercontrol, deployments, events, eventhub, orbat, missionview) |
| `cargo test -p xtask` | 776 passed |
| `cargo test -p developer-tools` | 263 passed, 4 ignored |
| `cargo test -p fleet-host-agent` | 127 passed |
| Clippy `-D warnings` | website-api, fleet-host-agent, developer-tools clean; website-map-engine clean for default, all, none and `store` feature sets |
| `verify file-length` / `route-tags` / `mission-rest-size-limits` | 0 violations (3,177 files) / PASS 144/144 / PASS |
| `ci ci-local-schema` | PASS |
| `mod compile` | clean, 0 TBD warnings |
| `mod world-boot` bare / `--compiled` | PASS / PASS (platform-compiled artifact, four-weapon equip) |
| `mod world-boot --mission=bridgehead-at-levie` | validated 0 errors; warning ratchet red (environment, layers, orbat radio — the documented baseline row) |
| `mod playtest --mission=<uuid>` | deployment CONFIRMED by the runtime session with the exact artifact; transition command cancelled; credential revoked on stop |
| `ci ci-local` | editorconfig passes after re-indenting the register (content identical); the replay then stops at `verify-no-python`, which fails closed on 138 tracked paths deleted in the working tree but not yet committed (108 tbd-export files from the equipment-export restructure, 15 superseded single-file generated API modules, 9 retired xtask files, 2 RCON frontend files, 2 retired mod loaders, the retired allowlist). It passes once those deletions are committed; the remaining ci-local steps have not run. |

## Defects fixed during verification

- Requests issued before the cold-start session restore were discarded by the E9 generation
  guard (compat feed "unavailable" in the Arsenal); requests now wait for the restore
  (`core::auth::session_restore`).
- A 30-second profile poll re-mounted gated pages and the review workspace editor; profile
  adoption now writes only changed signals and the gates render from memoized admission.
- Gate harness: JWT-shaped refresh tokens with a fixed `sid`; the DOM oracle answers bearer-less
  API requests with 401 as the API does.
- xtask `PATH` race in `core::test_environment`; route authorization tests assert the production
  path; dead scrub helpers removed; map-engine `source_scrub` gated on `store`; CI comments
  corrected; the register lists every new E/F/M file, `frontend_quality` ≥ 1,568 and
  `browser_acceptance` requires 25/25 oracle routes.

## Open items

- Re-check the profile-poll fix live in a browser (Credentials sheet stays open past 30 s; the
  review workspace editor does not re-boot).
- Read the `ci ci-local` result.
- Decompose the six pre-existing oversized EnfScript files (`TBD_SpawnManager`,
  `TBD_MissionLoader`, `TBD_FrameworkManager`, `TBD_LobbyService`, `TBD_ResultsReporter`,
  `TBD_AdminService`) in a separate pass verified by in-game playtest.
- Then milestones T, C, B, V and S, with a quiet working tree for `verify api-readiness --execute`.
