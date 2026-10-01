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
| Current stage | S0 Tooling (T1–T5 running) |
| Last green commit | 74735b80c (P0; readme-coverage red only on F-001) |
| Next action | Review T1–T5 reports as they land; then the S0 gate |
| Blocked on | nothing |

Row format: `- [ ] <ID> (<budget>) <role> — status — commit — notes`. A status is `pending`,
`running`, `done`, `blocked` or `fix-list`. Operator checkpoints are rows of their own.

## Stages

### P0 Program documents
- [x] P0 (orchestrator) program documents, research archive, pointers — done — commit: 74735b80c — documentation gate run before commit

### S0 Tooling
- [x] Orchestrator: unshallow the clone (`git fetch --unshallow`) in every fresh container before baselines — done (this container)
- [x] Orchestrator: container setup — `dockerd` started, `cargo xtask db up`, trunk 0.21.14 installed, builds without debug info (disk) — done
- [ ] T1 (L) relocation tool, xtask refactor relocate — running
- [x] T2 (M) workspace manifest hoisting, dependency fixes — done, awaiting the stage commit — lockfile changed only by the two intended removals; resolved graph otherwise identical (865 nodes)
- [ ] T3 (M) build output under target, trunk and wasm-bindgen-cli, Chromium discovery — running
- [ ] T4 (L) new laws and fail-closed roots — running
- [x] T5 (S) `Generated` exemption (F-001) and vestigial sqlx offline settings removed — done, awaiting the stage commit — readme-coverage 0 violations (from 19); 5 `generated_folder_exemption_*` tests; perturbation red on exactly the 19 folders and restored
- [x] Baselines recorded (see the execution log) — done
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
- [ ] F5 (M) frontend clippy-clean on native and wasm32 under `-D warnings` (finding F-004) — pending
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
| 2026-10-01 | S0 baseline | Pre-change tree (`74735b80c`): fmt OK; workspace clippy `-D warnings` red only in website-frontend native (F-004); frontend wasm32 clippy `-D warnings` red, 253 errors (F-004); editorconfig, no-python, no-node, no-shell, ci-shell, engine-layers, coding-standards, staging-compose-paths, mission-rest-size-limits, ci-schema-parity OK; verify-documentation red only on F-001; ci-local-schema red only because the Everon DEM was an unpulled LFS pointer (pulled afterwards); wasm-ci OK; rust-build OK; db test-it 159 binaries, 1425 passed, 0 failed, 0 ignored; ticket check OK; 2013 LFS files, 16226 tracked files. ci-local-schema, ci-local-leptos and leptos-gates are measured at the S0 gate, and against the base commit only if they fail | session scratchpad logs/s0-baseline-* |
| 2026-10-01 | S0 launch | T1–T5 launched in parallel from the scratchpad brief and prompts; `db test-it` baseline still running (its build finished before launch) | session scratchpad |
| 2026-10-01 | S0 T2 report | Root `[workspace.package]`, `[workspace.dependencies]` (identical requirements only) and `[workspace.lints]` (unsafe_code deny; missing_docs, unreachable_pub, dbg_macro, todo, unwrap_used warn); 11 member manifests inherit; `png` removed from xtask, `toml` made a dev-dependency, `earcutr` removed from the map engine; workspace and wasm32 checks pass; lint policy perturbation red on all 6 lints. Reviewed: lock diff and root manifest match the report | logs/T2-* |
| 2026-10-01 | S0 T5 report | Exemption covers `generated` and `Generated` (`path_regions.rs`); the three `SQLX_OFFLINE` lines and two emptied `env:` keys removed from ci.yml (YAML valid); readme standard and documentation standards updated. Reviewed: diff matches the report | logs/T5-* |

## Amendments

| Date | Agent | Amendment |
|---|---|---|

## Open findings

Each finding is triaged FIX, NOTE or CLOSE as the orchestration runbook defines; NOTE findings
become tickets at S12.

- **F-001 (FIX, S0 T5; operator: exempt the capitalised `Generated` too): red baseline in readme-coverage.** The 19 category folders under
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/Generated/`
  have no README. The coverage exemption matches only a lowercase `generated` folder. The fix is
  to make the exemption match the capitalised spelling Enfusion uses, or to have the gameplay
  policy generator emit the READMEs. This predates the program. Fixed by T5 (exemption); closes with
  the S0 commit.
- **F-005 (FIX, closing batch): `documentation_v2/standards/where_does_x_go.md:82` names only the
  lowercase `generated` exemption.** Reported by T5; the file was outside its ownership.
- **F-006 (FIX, closing batch): the map engine READMEs still list `earcutr`** (`apps/website/map-engine/README.md:103`, `apps/website/map-engine/src/README.md:86`). Reported by T2.
- **F-007 (NOTE, operator): `clippy::unwrap_used` in the workspace lint policy also fires in `#[cfg(test)]` code.** New crates' tests then use `expect`. The alternative is a root `clippy.toml` with `allow-unwrap-in-tests = true`. Reported by T2.
- **F-008 (NOTE, environment): one verification-core `file_length` test fails in this container because it runs as root, which can read a mode-000 file.** The test assumes a non-root user, as CI is. Gates run here record it as environmental. Reported by T2.
- **F-002 (CLOSE, P0): shallow clone.** The container's clone was shallow, so 353 archive
  permalinks failed link-check as unknown objects. `git fetch --unshallow` in P0 fixed it, and
  link-check passes (2044 checks). A fresh container clones shallow again, so every new session
  unshallows before running the gates.
- **F-003 (CLOSE, P0): the blueprint draft cited planned paths.** Its four backticked planned
  paths broke link-check. Fixed by archiving it as
  [the blueprint draft](/documentation_v2/archive/restructure_research/00_architecture_blueprint_draft.md).
- **F-004 (FIX, S3): the frontend is not clippy-clean.**
  - The native build has 764 clippy errors under `-D warnings`, mostly imports and code used only by
    the wasm build.
  - The wasm32 build has 253 errors (116 bin, 137 test).
  - CI lints the frontend for wasm32 only, without `-D warnings`, so this predates the program.
  - Until S3, GS step 2 runs with `--exclude website-frontend`, and step 3 may not raise the wasm
    warning count above the baseline.
  - S3 makes the frontend clean on both targets, because it touches every frontend file. From S3
    the GS steps apply in full.

## Handoff

P0 is complete: the program documents, the research archive and the pointers are in place. The
next step is S0. Write the session brief and the S0 agent prompts from the
[program plan](/documentation_v2/restructure/program_plan.md), take the baselines, then launch T1
to T5 in parallel.
