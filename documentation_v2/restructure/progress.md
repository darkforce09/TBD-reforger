**Status:** live

# Restructure progress

The single progress tracker of the restructure program. The orchestrator updates it after every
agent report and gate run, and commits and pushes it at every wave boundary
(`docs(restructure): progress <stage>/<wave>`), so a lost session costs at most one wave. A fresh
session reads the header and the Handoff section first.

## Current state

| Field | Value |
|---|---|
| Branch | `claude/compassionate-cannon-nz3hts` (the operator merges to `main`) |
| Current stage | P0 Program documents |
| Last green commit | not yet recorded |
| Next action | Commit and push P0, then wait for the operator's go-ahead to start S0 |
| Blocked on | nothing |

Row format: `- [ ] <ID> (<budget>) <role> — status — commit — notes`. A status is `pending`,
`running`, `done`, `blocked` or `fix-list`. Operator checkpoints are rows of their own.

## Stages

### P0 Program documents
- [x] P0 (orchestrator) program documents, research archive, pointers — done — commit: this stage's commit — documentation gate run before commit

### S0 Tooling
- [ ] Orchestrator: unshallow the clone (`git fetch --unshallow`) in every fresh container before baselines — pending
- [ ] T1 (L) relocation tool, xtask refactor relocate — pending
- [ ] T2 (M) workspace manifest hoisting, dependency fixes — pending
- [ ] T3 (M) build output under target, trunk and wasm-bindgen-cli, Chromium discovery — pending
- [ ] T4 (L) new laws and fail-closed roots — pending
- [ ] T5 (S) vestigial sqlx offline settings removed — pending
- [ ] Baselines recorded (GS counts, LFS count, test census) — pending
- [ ] Stage commit — pending

### S1 Global renames
- [ ] R1 (M) scripted renames and ticket path rewrites — pending
- [ ] R2 (M) layout modules and xtask literals — pending
- [ ] R3 (S) LFS rules, workflows, editor config, gate environment, codegen header — pending
- [ ] R4 (M) link-check spellings, READMEs, agent instruction files, mirror rule — pending
- [ ] Stage commit — pending

### S2 Apps and deploy
- [ ] A1 (M) app flattening, legacy parking, deploy folder — pending
- [ ] A2 (M) xtask deploy, staging and db — pending
- [ ] A3 (M) wave execution paths — pending
- [ ] A4 (S) Dockerfile, systemd, env example, runtime fallbacks — pending
- [ ] A5 (M) http_url_guard and offline_cache_policy — pending
- [ ] OC-deploy (operator) gitignored env files and server environment file moved — pending
- [ ] Stage commit — pending

### M1, M2 Mod
- [ ] M1 (S) References folder — pending
- [ ] M2 (S) Objectives Engine grouping — pending
- [ ] OC-mod (operator) ignored folders moved, `.rdb` regenerated, compile, world boot — pending
- [ ] Stage commit — pending

### S3 Frontend in place
- [ ] F1 (M) src/v2 split — pending
- [ ] F2 (M) shell extraction, editor session rename — pending
- [ ] F3 (S) UI tokens, logout hooks, route table — pending
- [ ] F4 (S) review workspace, byte formatting, mission review feature — pending
- [ ] Stage commit — pending

### S4 Tier 0–1
- [ ] B0 (S) cuts — pending
- [ ] B1 (L) tool foundations — pending
- [ ] B2 (M) foundation crates — pending
- [ ] B3 (M) geometry crates — pending
- [ ] B4 (M) render primitives — pending
- [ ] B5 (S) world file formats — pending
- [ ] B6 (M) contract crates — pending
- [ ] X4 (M) switch — pending
- [ ] B7 (S) engine rules retargeted — pending
- [ ] Stage commit — pending

### S5 Mission and ballistics
- [ ] D1 (L) mission authoring crates — pending
- [ ] D2 (M) ballistics crates — pending
- [ ] D3 (L) CRDT, document, formation geometry, operations — pending
- [ ] X5 (M) switch — pending
- [ ] D4 (S) data rules retired, two features deleted — pending
- [ ] Stage commit — pending

### S6 World CPU
- [ ] P0c (L) cuts — pending
- [ ] P1 (M) spatial, prefab, chunks — pending
- [ ] P2 (L) overlay CPU — pending
- [ ] P3 (L) elevation, relief, satellite, roads — pending
- [ ] P4 (M) water, vegetation, interiors — pending
- [ ] P5 (M) place names, world store — pending
- [ ] P6 (L) line of sight — pending
- [ ] X6 (M) switch — pending
- [ ] Stage commit — pending

### S7 Streaming CPU and editor
- [ ] Q0 (M) cuts — pending
- [ ] Q1 (M) scheduler and draw buffers — pending
- [ ] Q2 (L) editing session, commands, persistence — pending
- [ ] Q3 (M) editing tools — pending
- [ ] X7 (M) switch — pending
- [ ] Stage commit — pending

### S8 Rendering
- [ ] V0 (L) exports stripped, renderer redesign in place — pending
- [ ] V1 (L) GPU device, frame, core — pending
- [ ] V2 (M) symbology layers — pending
- [ ] V3 (M) paper doll — pending
- [ ] V4 (L) asset loading, streaming host — pending
- [ ] V5 (M) world layers — pending
- [ ] V6 (L) renderer and diagnostics — pending
- [ ] X8 (M) switch, legacy engines deleted — pending
- [ ] V7 (S) engine layer rules deleted — pending
- [ ] OC-web walkthrough online and with the API stopped — pending
- [ ] Stage commit — pending

### S9 API
- [ ] K0 (L) kernel cuts — pending
- [ ] K1 (M) infrastructure crates — pending
- [ ] K2 (M) state and kernel crates — pending
- [ ] K3a–h (M) eight domain crates — pending
- [ ] K4 (M) workers, thin app, staging fixtures — pending
- [ ] Stage commit — pending

### S10 Frontend crates
- [ ] H0 (L) editor untangle — pending
- [ ] H0b (S) pins, source lines, test support — pending
- [ ] H1 (L) foundation and feature crates — pending
- [ ] H2a–g (S/M) page crates — pending
- [ ] H3 (L) workspace crates — pending
- [ ] H4 (M) thin app — pending
- [ ] H5 (M) edition 2024 — pending
- [ ] Walkthrough — pending
- [ ] Stage commit — pending

### S11 Tools
- [ ] J0 (M) tool cycles — pending
- [ ] J1 (L) ticket crates, legacy ticket engine deleted — pending
- [ ] J2a–f (M) command and check crates — pending
- [ ] J3a–d (M) developer-tools crates — pending
- [ ] J4 (S) thin binaries, anyhow out of libraries — pending
- [ ] Stage commit — pending

### M3 Objective behaviours
- [ ] M3a (S) switch-site catalogue — pending
- [ ] M3b (M) behaviour base and types — pending
- [ ] M3c (M) Registry rewiring — pending
- [ ] M3d (M) Runtime and systems rewiring — pending
- [ ] OC-mod playtest of capture, destroy and hold-until — pending
- [ ] Stage commit — pending

### S12 Close
- [ ] G1… closing-fix batches — pending
- [ ] Agent instruction files and standards rewritten — pending
- [ ] Tree diff against the target file tree — pending
- [ ] Final sweep, perturbation proofs, walkthrough — pending
- [ ] Program folder archived, pointers removed — pending
- [ ] Stage commit — pending

## Execution log

| Date | Event | Detail | Logs |
|---|---|---|---|
| 2026-10-01 | Research | Explorer, planning and verification reports archived in the restructure research folder | — |
| 2026-10-01 | Decisions | D1–D12 recorded in the program plan | — |
| 2026-10-01 | P0 | Program documents written; blueprint draft archived; clone unshallowed. link-check OK (2044), markdown-placement OK (1324), readme-coverage red only on F-001 | session scratchpad |

## Amendments

| Date | Agent | Amendment |
|---|---|---|

## Open findings

Each finding is triaged FIX, NOTE or CLOSE as the orchestration runbook defines; NOTE findings
become tickets at S12.

- **F-001 (FIX, S0 T4): red baseline in readme-coverage.** The 19 category folders under
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/Generated/`
  have no README. The coverage exemption matches only a lowercase `generated` folder. The fix is
  to make the exemption match the capitalised spelling Enfusion uses, or to have the gameplay
  policy generator emit the READMEs. This predates the program.
- **F-002 (CLOSE, P0): shallow clone.** The container's clone was shallow, so 353 archive
  permalinks failed link-check as unknown objects. `git fetch --unshallow` in P0 fixed it, and
  link-check passes (2044 checks). A fresh container clones shallow again, so every new session
  unshallows before running the gates.
- **F-003 (CLOSE, P0): the blueprint draft cited planned paths.** Its four backticked planned
  paths broke link-check. Fixed by archiving it as
  [the blueprint draft](/documentation_v2/archive/restructure_research/00_architecture_blueprint_draft.md).

## Handoff

P0 is complete: the program documents, the research archive and the pointers are in place. The
next step is S0. Write the session brief and the S0 agent prompts from the
[program plan](/documentation_v2/restructure/program_plan.md), take the baselines, then launch T1
to T5 in parallel.
