**Status:** live

# Restructure progress

The single progress tracker of the restructure program. The orchestrator updates it after every
agent report and gate run, and commits and pushes it at every wave boundary
(`docs(restructure): progress <stage>/<wave>`), so a lost session costs at most one wave. A fresh
session reads the header and the Handoff section first.

## Current state

| Field | Value |
|---|---|
| Branch | `main` (decision D16; S0 was merged from the cloud session branch) |
| Current stage | S1 Global renames |
| Last green commit | b2b317446 (S0) |
| Next action | Continue on the operator's local machine: S1 agent R1 (see Handoff) |
| Blocked on | nothing |

Row format: `- [ ] <ID> (<budget>) <role> — status — commit — notes`. A status is `pending`,
`running`, `done`, `blocked` or `fix-list`. Operator checkpoints are rows of their own.

## Stages

### P0 Program documents
- [x] P0 (orchestrator) program documents, research archive, pointers — done — commit: 74735b80c — documentation gate run before commit

### S0 Tooling
- [x] Orchestrator: unshallow the clone (`git fetch --unshallow`) in every fresh container before baselines — done (this container)
- [x] Orchestrator: container setup — `dockerd` started, `cargo xtask db up`, trunk 0.21.14 installed, builds without debug info (disk) — done
- [ ] T1 (L) relocation tool, xtask refactor relocate — stopped by a container restart; about 5,200 lines left in the tree
- [x] T1b (L) finish the relocation tool from T1's partial work — done, awaiting the stage commit — 28 `relocate_` tests; real-tree dry run `assets_v2` → `assets`: 4651 files moved, 644 references in 259 files, 0 unresolved; the same plan applied in a scratch clone verifies clean
- [x] T2 (M) workspace manifest hoisting, dependency fixes — done, awaiting the stage commit — lockfile changed only by the two intended removals; resolved graph otherwise identical (865 nodes)
- [x] T3 (M) build output under target, `rust-sqlx-prepare` removed, Chromium discovery — done, awaiting the stage commit — ran over budget (about 310k); dev-api, ci and gate-* folders now under `target/`; Chromium found with `CHROME_HEADLESS_SHELL` unset
- [x] T4 (L) new laws (crate-tiers, crate-anatomy, strangler, frontend-layering, tailwind-sources) and their wiring — done, awaiting the stage commit — five gates pass on the real tree; frontend layering ceiling 12 production + 3 test edges; every law red under perturbation and restored
- [x] T4b (M) fail-closed source roots derived from workspace members — done, awaiting the stage commit — roots follow the members; missing member folder or empty workspace is TargetMissing; old judged set ⊆ new; 10 `source_roots_` tests; file-length 4606 files, 0 violations
- [x] G0 (M) closing batch: F-014 (prose rule, D13), F-007 (clippy.toml, D14), F-005, F-006, F-009, F-010 — done, awaiting the stage commit
- [x] T5 (S) `Generated` exemption (F-001) and vestigial sqlx offline settings removed — done, awaiting the stage commit — readme-coverage 0 violations (from 19); 5 `generated_folder_exemption_*` tests; perturbation red on exactly the 19 folders and restored
- [x] Baselines recorded (see the execution log) — done
- [x] Stage commit — done (see the execution log)

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
| 2026-10-01 | S0 T3 report | Build output under `target/<purpose>` (dev-api, ci, gate-trunk, gate-dist-frontend, gate-check, gate-schema, gate-api, gate-map-engine, gate-frontend, gate-tools, gate-slice-frontend-<slice>); reclaim deletes retired root folders; `rust-sqlx-prepare` removed; `find_chromium` moved to `cdp/chromium_discovery.rs` and honours `PLAYWRIGHT_BROWSERS_PATH`; 15 tests added; perturbation red on 3 tests and restored. Out-of-list edits reviewed and accepted: `cdp.rs` module wiring and the Chromium and deploy docs (law 10). Reviewed: `.gitignore` and the dev-api pin match the report | logs/T3-* |
| 2026-10-01 | S0 T4 report | `workspace_members.rs`, a hand-written TOML subset reader under `cargo_manifest/`, and `workspace_laws/` (10 modules) in verification-core; xtask wrappers, the `verify-workspace-laws` ci task (a step of ci-local), five ci.yml steps, coding standards gates WS-1 to WS-5. 38 verification-core tests and 3 xtask tests added. Item 7 (fail-closed source roots) not reached: becomes T4b. Unowned edits reviewed and accepted: `ci/tests/task_runner.rs` (pins the new ci-local step), `task_definitions/README.md` and `workspace_law_steps.rs` (keeps `task_definitions.rs` under 500 lines) | logs/T4-* |
| 2026-10-01 | S0 restart | Container restart stopped T1 mid-run; its partial work survived on disk. T1b launched to finish it from that state | — |
| 2026-10-01 | S0 T1b report | T1's code compiled; T1b added the READMEs, one test, formatting and clippy fixes; `--verify` perturbation red and restored. All S0 agents done; PAUSED at the operator's request before the gate | logs/T1b-* |
| 2026-10-01 | S0 T5 report | Exemption covers `generated` and `Generated` (`path_regions.rs`); the three `SQLX_OFFLINE` lines and two emptied `env:` keys removed from ci.yml (YAML valid); readme standard and documentation standards updated. Reviewed: diff matches the report | logs/T5-* |
| 2026-10-02 | S0 decisions | Operator: D13 keep `legacy/` with a prose-rule exemption for the folder name; D14 allow `unwrap()` in tests; finish S0 and continue into S1 without pausing. Container restarted: `dockerd` and Postgres restarted, `target/` cleaned (18.5 GiB) | — |
| 2026-10-02 | S0 T4b report | `law_source_roots` walks every workspace member (nested members once); `FILE_LENGTH_PINS` became `PINNED_SCRIPT_ROOTS` (the mod script roots); perturbation red and restored; renaming a member folder on a scratch copy gives exit 2 | logs/T4b-* |
| 2026-10-02 | S0 G0 report | Prose rule exempts the parking folder name (`legacy/`, `"legacy"`) only; the retired `crates/(tbd|map)` needle now ends at the hyphen so the planned snake_case category folders are live names (reviewed and accepted: every retired crate folder was hyphenated, a test proves it still bites); root `clippy.toml` allows `unwrap()` in tests; mcp daemon and db selftest build under `target/`; stale docs fixed. Orchestrator fixed the last `target-<slice>` hint in `wave_execution/flush.rs` | logs/G0-* |
| 2026-10-02 | S0 gate (first pass) | fmt OK; workspace clippy `-D warnings` (frontend excluded, F-004) OK; frontend wasm32 warnings 116 + 137, equal to the baseline; relocate --verify OK; crate-tiers, crate-anatomy, strangler, frontend-layering, tailwind-sources, file-length, ci-schema-parity OK; API release check OK; verification-core 185/186 and xtask 1322/1324 (only the root-permission cases, F-008); ci-local stopped at `rust-test-it` because it calls `podman` (environment: a `podman`→`docker` shim added to the program env); ticket-engine round-trip red on T-086 (F-016) and `ticket check --strict` red on a retired id spelling (F-017), both pre-existing; leptos-gates red on satellite, roads and buildings because the Everon LFS objects were not pulled (pulled: all 2013) | logs/s0-gate-* |
| 2026-10-02 | S0 G0b report | F-016: T-086 rewritten by `ticket set-status` (a pure line move, no value changed); F-017: plan slice ids in the mod modularisation checkpoint respelled; ticket-engine 236 passed, `ticket check --strict` OK, verify-documentation OK | logs/G0b-* |
| 2026-10-02 | S0 gate (final) | `ci-local` green with a `podman`→`docker` shim: 172 test binaries, 5189 passed, 0 failed (includes API integration tests, trunk release build, documentation gates, the five new laws); browser gates: every smoke passes except outliner-drag and perf, which time out in `Runtime.evaluate`; outliner-drag fails the same way on the pre-S0 commit (F-019). Operator: too much testing — decision D15 (lean gates) | logs/s0-gate-*, s0-smoke-* |

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
- **F-007 (decided, D14): `clippy::unwrap_used` in the workspace lint policy also fires in `#[cfg(test)]` code.** New crates' tests then use `expect`. The alternative is a root `clippy.toml` with `allow-unwrap-in-tests = true`. Reported by T2.
- **F-008 (NOTE, environment): one verification-core `file_length` test fails in this container because it runs as root, which can read a mode-000 file.** The test assumes a non-root user, as CI is. Gates run here record it as environmental. Reported by T2.
- **F-009 (FIX, closing batch): two tools still create root-level build folders.** `tools_v2/xtask/src/commands/mcp/daemon.rs:96` (`target-dev-mcpd`) and `tools_v2/xtask/src/commands/db/operations/selftest.rs:365` (`target-mk-db-selftest`) move under `target/`. Reported by T3.
- **F-010 (FIX, closing batch): a slice-agent brief still tells agents to use `target-<id>-api`** (`slice_agent_brief.md:72`). Reported by T3.
- **F-011 (NOTE, environment): eight developer-tools library tests read Git LFS objects this container has not pulled.** The S0 gate pulls the LFS objects those tests need before running them.
- **F-012 (decided, D13): the name of the parking folder.** The repository's prose rule (`tools_v2/xtask/src/tests/tooling_prose_rules.rs:65`) bans the word "legacy" as history vocabulary, which is the planned folder name of decision D12. The laws T4 wrote name that folder, so one xtask prose test fails now. Either rename the parking folder to a present-tense name, or exempt the folder name from the rule. Reported by T4.
- **F-013 (NOTE, S2): a legacy map engine that re-exports the graphics engine would show about 28 shims.** The S2 and S4 prompts must cut or switch them in the stage that creates them. Reported by T4.
- **F-014 (FIX, before the S0 gate): the xtask prose-rule tests fail on T4's law files.** `tools_v2/xtask/src/tests/tooling_prose_rules.rs` flags the word "legacy" (see F-012) and planned paths such as `crates/map_rendering` and `prelude.rs` written in prose. Reported by T1b and T4.
- **F-015 (NOTE): a relocation `path` row never rewrites `mod` declarations**; each stage's author edits them. Reported by T1b.
- **F-016 (FIX, done by G0b): `.ai/tickets/T-086.toml` was not in canonical form** (hand-edited in milestone S), so the ticket store's round-trip test failed.
- **F-017 (FIX, done by G0b): `ticket check --strict` rejected plan slice ids** in `documentation_v2/mod/script_modularisation_progress_checkpoint.md` that match the retired priority-backlog id pattern.
- **F-018 (NOTE, environment): the `rust-test-it` task calls `podman` directly** (`tools_v2/xtask/src/commands/ci/task_definitions.rs`), unlike `cargo xtask db test-it`, which resolves the runtime. This container uses a `podman`→`docker` shim. A code fix to resolve the runtime in that task belongs to S2 (A2 owns the db lane).
- **F-019 (NOTE, environment): the outliner-drag and perf browser smokes time out in `Runtime.evaluate` in this container.** Outliner-drag fails identically on the pre-S0 commit; perf shows the same signature. The container's Chromium is 141, while the gate pins 149.
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

S0 is committed (`b2b317446`) and pushed; the program continues on the operator's local machine.

Next step, S1 global renames:
1. Pull `main` and run `cargo xtask ci verify-documentation` and
   `cargo xtask refactor relocate --verify` once, to confirm the checkout is green on that machine.
2. Launch agent R1 with
   [its document](/documentation_v2/restructure/agent_briefs/s1_r1_global_renames.md). It runs
   alone, because it rewrites paths across the whole tree.
3. When R1 reports, fill the
   [R2 to R4 template](/documentation_v2/restructure/agent_briefs/s1_r2_to_r4_template.md) from
   its report and run R2, R3 and R4 in parallel.
4. Run the stage gate (decision D15) and commit S1.

Plan amendment A1: the ticket engine is renamed to its snake_case tools folder in S1 and is not
parked under `legacy/`; S11 dissolves it.

Machine notes. This cloud container needed several workarounds:
- `git fetch --unshallow`;
- `dockerd` plus a `podman`→`docker` shim (F-018);
- an Everon Git LFS pull;
- trunk 0.21.14 and wasm-bindgen-cli 0.2.126;
- build flags without debug info to fit its disk;
- `CHROME_HEADLESS_SHELL` for its older Chromium.

A local machine with the usual tool chain needs none of these. The root-permission test failures
(F-008) and the two browser smoke timeouts (F-019) are expected only in this container.
