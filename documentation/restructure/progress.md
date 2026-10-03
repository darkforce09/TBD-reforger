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
| Current stage | S5 and the later stages under the coordinator (decision D22); S3, S4 (S4a, S4b) and S6 landed |
| Last green commit | the S6 stage commit (S3: 90cfa9aba, S4b: 65b7978ea, S4a: dcfeda907) |
| Next action | Stage orchestrators in their worktrees, managed by the coordinator (see Handoff and the stage logs); the relocation tool fix landed (`640398d6e`) |
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
- [x] T1b (L) finish the relocation tool from T1's partial work — done, awaiting the stage commit — 28 `relocate_` tests; real-tree dry run `assets` → `assets`: 4651 files moved, 644 references in 259 files, 0 unresolved; the same plan applied in a scratch clone verifies clean
- [x] T2 (M) workspace manifest hoisting, dependency fixes — done, awaiting the stage commit — lockfile changed only by the two intended removals; resolved graph otherwise identical (865 nodes)
- [x] T3 (M) build output under target, `rust-sqlx-prepare` removed, Chromium discovery — done, awaiting the stage commit — ran over budget (about 310k); dev-api, ci and gate-* folders now under `target/`; Chromium found with `CHROME_HEADLESS_SHELL` unset
- [x] T4 (L) new laws (crate-tiers, crate-anatomy, strangler, frontend-layering, tailwind-sources) and their wiring — done, awaiting the stage commit — five gates pass on the real tree; frontend layering ceiling 12 production + 3 test edges; every law red under perturbation and restored
- [x] T4b (M) fail-closed source roots derived from workspace members — done, awaiting the stage commit — roots follow the members; missing member folder or empty workspace is TargetMissing; old judged set ⊆ new; 10 `source_roots_` tests; file-length 4606 files, 0 violations
- [x] G0 (M) closing batch: F-014 (prose rule, D13), F-007 (clippy.toml, D14), F-005, F-006, F-009, F-010 — done, awaiting the stage commit
- [x] T5 (S) `Generated` exemption (F-001) and vestigial sqlx offline settings removed — done, awaiting the stage commit — readme-coverage 0 violations (from 19); 5 `generated_folder_exemption_*` tests; perturbation red on exactly the 19 folders and restored
- [x] Baselines recorded (see the execution log) — done
- [x] Stage commit — done (see the execution log)

### S1 Global renames
- [x] R1 (M) scripted renames and ticket path rewrites — done, awaiting the stage commit — 7685 files moved, 2756 rewritten (17234 references), 0 unresolved after one pre-apply fix; workspace compiles; `relocate --verify` red only on T-1073 (F-020)
- [x] R2 (M) layout modules and xtask literals — done, awaiting the stage commit — verification_core 187, xtask 1334 (1 red until R5's file is staged), ticket_engine red only on F-023
- [x] R3 (M) apps, LFS rules, workflows, editor config — done, awaiting the stage commit — every app suite green; 36 moved-path API integration binaries 262 passed
- [x] R4 (M) link-check spellings, READMEs, agent instruction files, mirror rule, architecture docs — done, awaiting the stage commit — documentation gates red only on untracked files, R5's README and F-023
- [x] R5 (M) relocation tool blind spots and a dry run that verifies its plan (F-021, F-022, F-023, F-024) — done — 11 `relocate_*` tests added (40 pass twice); the dry run verifies the planned tree and `--apply` refuses a plan whose dry run fails; clone proof of the S1 manifest at `3ed754a64` matches R1's hand fixes byte for byte
- [x] Stage commit — done (see the execution log)

### S2 Apps and deploy
- [x] A1 (M) app flattening, legacy parking, deploy folder — done, awaiting the stage commit — 3864 row moves, 13104 references, 0 unresolved; 16301 tracked and 2013 LFS files unchanged; workspace compiles
- [x] A2 (M) xtask deploy, staging and db; ci task id `api-test` — done (A2b finished it), awaiting the stage commit
- [x] A3 (M) wave execution paths — done, awaiting the stage commit — four fail-open defects fixed on the way (edition, member globs, golden-only changes, include consumers)
- [x] A4 (S) Dockerfile repair (D18), systemd, env example, runtime fallbacks — done, awaiting the stage commit — image builds; `.dockerignore`; compose files name their projects
- [x] A5 (M) http_url_guard and offline_cache_policy — done, awaiting the stage commit — both crates pass crate anatomy, tiers and strangler; 10 `include!` sites replaced
- [x] A6 (S) documentation — done, awaiting the stage commit
- [x] A7 (S) relocation tool false rewrites (F-030) — done, awaiting the stage commit — separators-only literals name no path; a plain token is read only when its whole path is tracked; clone proof of the S2 manifest clean without the extra row
- [x] A2b (M) finish A2: `db` compose calls name `deploy/compose.dev.yml`, F-018 closed — done
- [x] G-S2 (S) fail-closed documentation gate roots, website checkout set — done
- [x] G2a (M) API test paths, golden index, build-lane runtime, citation roots, member-derived CI tests — done
- [x] G2b (S) Caddyfile in `deploy/caddy/`, mounted alone — done
- [x] G2c (M) documentation, executed briefs archived — done
- [x] OC-deploy (operator) local part — done: ignored files rode along; on request the orchestrator removed five retired keys from the local `deploy/deploy.env`, repointed `EQUIPMENT_DATA_DIR`, deleted the empty old uploads folder. Server steps and GitHub required checks are the operator's, before the next deploy (archived S2 brief, OC-deploy section)
- [x] Stage commit — done (see the execution log)

### M1, M2 Mod
- [x] M1 (S) References folder, tools fail closed — done (M1b finished it), awaiting the stage commit — lanes in `apps/mod/References/`, constants in both layout modules, leak gate fails closed and runs in 8 s, deploy excludes one References rule
- [x] M2 (S) Objectives Engine grouping — done, awaiting the stage commit — four folders moved into `Engine/` by manifest `m2_objectives_engine.tsv`
- [x] OC-mod — done: lanes moved and PlayableSelector copied by the orchestrator; the operator regenerated both `.rdb` files in Workbench; `mod compile` clean (world boot moved to S12, D20)
- [x] Stage commit — done (see the execution log)

### S3 Frontend in place
- [x] F1 (L) every move: src/v2 split, transport, shell, route table, editor session, review workspace, mission review record, documentation mirrors — done — 1247 files moved, layering {6, 0}
- [x] F2 (L) foundation cycles: `TokenProvider`, wire types into transport, gates, UI tokens, logout hooks, route guard over the route table — done — foundation edges follow the declared order
- [x] F3 (S) pages and workspaces edges: byte formatting, review workspace — done — layering 0 and 0
- [x] F4 (S) frontend-layering hard at zero with the foundation order; frontend clippy `-D warnings` in CI — done — hard at zero with the foundation order; zero pending F3
- [x] F5a (M) ∥ F5b (L) ∥ F5c (S), then F5z (S): frontend clippy-clean on native and wasm32 under `-D warnings` (finding F-004) — done — with follow-ups F5ab, F5bb, F5z, F5y: both targets at zero, 2012 tests
- [x] Stage commit — done (see stage_logs/s3.md); frontend clippy `-D warnings` on both targets is the norm from here

### S4 Tier 0–1
S4 lands in two commits (operator): S4a, the crates S5 and S6 need; S4b, tool foundations and
contract crates. Log: [stage_logs/s4.md](/documentation/restructure/stage_logs/s4.md).
- [x] B0 (S) cuts: the `dem/sample` re-export module deleted, its 26 tests re-homed — done
- [x] B2a (M) `newtype_ids`, `time_source`, `deterministic_random` — done
- [x] B2b (M) `content_digest` (xtask's sha2 switched), `browser_platform` — done
- [x] B3 (M) `geometry_primitives`, `map_coordinates`, `camera_math` — done
- [x] B4 (M) `render_primitives` — done
- [x] B5 (S) `world_file_formats` — done; B5b (M) its typed ids (F-S4-01) — done
- [x] X4a (M) S4a switch: 51 shims and every forwarding facade deleted — done
- [x] S4a stage commit — done (see the stage log)
- [x] B1a (L) `verification_core`, `process_runner`, `repository_laws` — done
- [x] B1b (M) `repository_layout`; B1c (M) developer_tools' root finder and test roots — done
- [x] B6a (M) `fleet_wire_contract` — done
- [x] B6b (L) `contract_schema_types` — done
- [x] B7 (S) engine rules retargeted — done
- [x] S4b stage commit — done (see the stage log)

### S5 Mission and ballistics
- [ ] D1 (L) mission authoring crates — pending
- [ ] D2 (M) ballistics crates — pending
- [ ] D3 (L) CRDT, document, formation geometry, operations — pending
- [ ] X5 (M) switch — pending
- [ ] D4 (S) data rules retired, two features deleted — pending
- [ ] Stage commit — pending

### S6 World CPU
- [x] P0c (M) cuts K1–K12 inside the map engine — done — map engine 1665 library tests equal to the baseline; rebased onto S4a by R6 (30 conflicts)
- [x] P1 (M) spatial_indexes, prefab_catalog, world_chunks — done — one flat-tree build core shared with the world TLAS; `ChunkId`, `TerrainId`
- [x] P2 (L) map_draw_lanes, label_layout, unit_symbology, overlay_instances — done — glyph-math copies deleted; ORBAT coherency pins retargeted
- [x] P3 (L) terrain_elevation, terrain_relief, satellite_imagery, water_bodies — done (water swapped in from P4, operator)
- [x] P4 (M) road_network, vegetation, building_interiors — done (roads swapped in from P3) — section index on the shared tree core
- [x] P5 (M) place_names, world_store — done
- [x] P6 (L) terrain, interior and world line of sight — done — one 3D `evaluate_los`
- [x] X6 (L) switch — done — 79 shims switched and deleted; `bvh` and `io` features deleted; strangler catches aliased re-exports; rebased onto S4b and S3 by R7 first
- [x] G6 (M) closing batch — done — `PrefabId` with a typed refusal of bad ids; 498 placeholder field docs rewritten
- [x] Stage commit — done — `refactor(restructure): S6 world CPU crates`; handoff to S7 and S8 in `stage_logs/s6.md`

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
- [ ] J3a–d (M) developer_tools crates — pending
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
| 2026-10-01 | S0 baseline | Pre-change tree (`74735b80c`): fmt OK; workspace clippy `-D warnings` red only in frontend native (F-004); frontend wasm32 clippy `-D warnings` red, 253 errors (F-004); editorconfig, no-python, no-node, no-shell, ci-shell, engine-layers, coding-standards, staging-compose-paths, mission-rest-size-limits, ci-schema-parity OK; verify-documentation red only on F-001; ci-local-schema red only because the Everon DEM was an unpulled LFS pointer (pulled afterwards); wasm-ci OK; rust-build OK; db test-it 159 binaries, 1425 passed, 0 failed, 0 ignored; ticket check OK; 2013 LFS files, 16226 tracked files. ci-local-schema, ci-local-leptos and leptos-gates are measured at the S0 gate, and against the base commit only if they fail | session scratchpad logs/s0-baseline-* |
| 2026-10-01 | S0 launch | T1–T5 launched in parallel from the scratchpad brief and prompts; `db test-it` baseline still running (its build finished before launch) | session scratchpad |
| 2026-10-01 | S0 T2 report | Root `[workspace.package]`, `[workspace.dependencies]` (identical requirements only) and `[workspace.lints]` (unsafe_code deny; missing_docs, unreachable_pub, dbg_macro, todo, unwrap_used warn); 11 member manifests inherit; `png` removed from xtask, `toml` made a dev-dependency, `earcutr` removed from the map engine; workspace and wasm32 checks pass; lint policy perturbation red on all 6 lints. Reviewed: lock diff and root manifest match the report | logs/T2-* |
| 2026-10-01 | S0 T3 report | Build output under `target/<purpose>` (dev-api, ci, gate-trunk, gate-dist-frontend, gate-check, gate-schema, gate-api, gate-map-engine, gate-frontend, gate-tools, gate-slice-frontend-<slice>); reclaim deletes retired root folders; `rust-sqlx-prepare` removed; `find_chromium` moved to `cdp/chromium_discovery.rs` and honours `PLAYWRIGHT_BROWSERS_PATH`; 15 tests added; perturbation red on 3 tests and restored. Out-of-list edits reviewed and accepted: `cdp.rs` module wiring and the Chromium and deploy docs (law 10). Reviewed: `.gitignore` and the dev-api pin match the report | logs/T3-* |
| 2026-10-01 | S0 T4 report | `workspace_members.rs`, a hand-written TOML subset reader under `cargo_manifest/`, and `workspace_laws/` (10 modules) in verification_core; xtask wrappers, the `verify-workspace-laws` ci task (a step of ci-local), five ci.yml steps, coding standards gates WS-1 to WS-5. 38 verification_core tests and 3 xtask tests added. Item 7 (fail-closed source roots) not reached: becomes T4b. Unowned edits reviewed and accepted: `ci/tests/task_runner.rs` (pins the new ci-local step), `task_definitions/README.md` and `workspace_law_steps.rs` (keeps `task_definitions.rs` under 500 lines) | logs/T4-* |
| 2026-10-01 | S0 restart | Container restart stopped T1 mid-run; its partial work survived on disk. T1b launched to finish it from that state | — |
| 2026-10-01 | S0 T1b report | T1's code compiled; T1b added the READMEs, one test, formatting and clippy fixes; `--verify` perturbation red and restored. All S0 agents done; PAUSED at the operator's request before the gate | logs/T1b-* |
| 2026-10-01 | S0 T5 report | Exemption covers `generated` and `Generated` (`path_regions.rs`); the three `SQLX_OFFLINE` lines and two emptied `env:` keys removed from ci.yml (YAML valid); readme standard and documentation standards updated. Reviewed: diff matches the report | logs/T5-* |
| 2026-10-02 | S0 decisions | Operator: D13 keep `legacy/` with a prose-rule exemption for the folder name; D14 allow `unwrap()` in tests; finish S0 and continue into S1 without pausing. Container restarted: `dockerd` and Postgres restarted, `target/` cleaned (18.5 GiB) | — |
| 2026-10-02 | S0 T4b report | `law_source_roots` walks every workspace member (nested members once); `FILE_LENGTH_PINS` became `PINNED_SCRIPT_ROOTS` (the mod script roots); perturbation red and restored; renaming a member folder on a scratch copy gives exit 2 | logs/T4b-* |
| 2026-10-02 | S0 G0 report | Prose rule exempts the parking folder name (`legacy/`, `"legacy"`) only; the retired crates/(tbd|map) needle now ends at the hyphen so the planned snake_case category folders are live names (reviewed and accepted: every retired crate folder was hyphenated, a test proves it still bites); root `clippy.toml` allows `unwrap()` in tests; mcp daemon and db selftest build under `target/`; stale docs fixed. Orchestrator fixed the last `target-<slice>` hint in `wave_execution/flush.rs` | logs/G0-* |
| 2026-10-02 | S0 gate (first pass) | fmt OK; workspace clippy `-D warnings` (frontend excluded, F-004) OK; frontend wasm32 warnings 116 + 137, equal to the baseline; relocate --verify OK; crate-tiers, crate-anatomy, strangler, frontend-layering, tailwind-sources, file-length, ci-schema-parity OK; API release check OK; verification_core 185/186 and xtask 1322/1324 (only the root-permission cases, F-008); ci-local stopped at `rust-test-it` because it calls `podman` (environment: a `podman`→`docker` shim added to the program env); ticket_engine round-trip red on T-086 (F-016) and `ticket check --strict` red on a retired id spelling (F-017), both pre-existing; leptos-gates red on satellite, roads and buildings because the Everon LFS objects were not pulled (pulled: all 2013) | logs/s0-gate-* |
| 2026-10-02 | S0 G0b report | F-016: T-086 rewritten by `ticket set-status` (a pure line move, no value changed); F-017: plan slice ids in the mod modularisation checkpoint respelled; ticket_engine 236 passed, `ticket check --strict` OK, verify-documentation OK | logs/G0b-* |
| 2026-10-02 | S0 gate (final) | `ci-local` green with a `podman`→`docker` shim: 172 test binaries, 5189 passed, 0 failed (includes API integration tests, trunk release build, documentation gates, the five new laws); browser gates: every smoke passes except outliner-drag and perf, which time out in `Runtime.evaluate`; outliner-drag fails the same way on the pre-S0 commit (F-019). Operator: too much testing — decision D15 (lean gates) | logs/s0-gate-*, s0-smoke-* |
| 2026-10-02 | S1 launch | Local machine: pre-flight on `36c2fa22a` green (verify-documentation OK, 16294 tracked files; `relocate --verify` OK); 2013 LFS files; builds run on the host against the shared warm cache through a cargo shim (amendment A3); the two Workbench-regenerated `resourceDatabase.rdb` files stay uncommitted and foreign; 36 API integration binaries that spell a moved path selected for the S1 gate. Operator: continue into S2 after the S1 commit | target/api-progress-checkpoint/2026-10-02-restructure-s1/logs/s1-preflight-* |
| 2026-10-02 | S1 R1 report | Dry run 2: 8447 row moves (7685 files), 2756 files rewritten, 17234 references, 0 unresolved, 3222 binaries moved unread; apply moved and wrote everything, then its verify flagged 5 retired spellings the dry run had not (four split test listings and `file://` lines fixed by hand; T-1073 left, F-020); LFS 2013 with `filter=lfs`, `git lfs fsck --pointers` OK; `cargo check --workspace --all-targets --locked` OK; lock diff name-only (8 lines). Reviewed: index holds 7685 renames, every old path gone | logs/R1-* |
| 2026-10-02 | S1 wave 2 | R2 (tools code), R3 (apps and repository config, 36 moved-path API integration binaries), R4 (documentation) and R5 (relocation tool, added) launched in parallel with disjoint ownership | prompt_R2..R5.md |
| 2026-10-02 | S1 R3 report | map engine (`--all-features`) 1665 lib tests and its integration tests, graphics engine 44, service worker 28, fleet host agent 148, ticketboard 179, frontend native 2005, API lib 566, 36 moved-path API integration binaries 262 passed 0 failed; `verify ci-schema-parity` OK; workflows parse; LFS attribute set equals HEAD's pointer set (2013); `check-ignore` matches all moved ignored paths. Two comment edits only | logs/R3-* |
| 2026-10-02 | S1 R2 report | Sweep roots deduplicated; `PROGRAM_RECORDS_PREFIX` removed with its uses (the archive rules cover the records); codegen byte-identical apart from the header paths; perturbation red on both exemption sites and restored. verification_core 187/187; xtask 1334 passed, 1 red on R5's untracked file (clears when staged); ticket_engine 234 passed, 1 red (F-023) | logs/R2-* |
| 2026-10-02 | S1 R4 report | `documentation/architecture/` born; CLAUDE.md atlas, `.cursor/rules`, the mirror rule, 30 archived status lines, 21 README indexes under the archive and ticket folders fixed by hand; documentation gates red only on untracked files (staged at the commit), R5's relocate README and F-023 | logs/R4-* |
| 2026-10-02 | S2 briefs drafted | Outside the tree from the snapshot `3ed754a64`: A1 brief with a 38-row manifest draft and run-time name table, A2–A5 bodies, OC-deploy steps, gate extras. Operator: D17 (fleet host agent renamed everywhere), D18 (A4 repairs the Dockerfile); the stale gitignored `target-container-api-v2/` under the frontend deleted | s2_drafts/ |
| 2026-10-02 | S1 R5 report | Escape-adjacent paths, `file://` URLs and `/../` macro pieces rewritten or reported; the manifests README is live; a file takes the treatment of where it lands; closed tickets' `owns` entries rewritten; the dry run verifies the planned tree; perturbation red on both new passes and restored. Orchestrator copied the tool's own clone output for the 35 files moved into the archive (R4's status lines kept, three README indexes kept at R4's version, F-029) and 33 ticket records (31 closed tickets' `owns`, T-1073, T-1112) | logs/R5-*, probes/R5/s1-clone |
| 2026-10-02 | S1 gate | fmt OK (four app files reformatted after the shorter paths); workspace clippy `-D warnings` (frontend native excluded, F-004) OK; frontend wasm32 clippy 116 + 137, equal to the baseline; per-package tests: xtask 1336, verification_core 187, ticket_engine 236, developer_tools 403 (4 ignored), and R3's app runs (map engine 1665 lib with `--all-features`, frontend 2005, API lib 566, 36 API integration binaries 262); `verify-workspace-laws` OK; file-length 4608 files, 0 violations; `relocate --verify` OK (28 checks); verify-documentation OK; `ticket check --strict` OK; `ci-local-schema` OK; route-tags OK; editorconfig OK; `ci-schema-parity` OK; `git lfs fsck --pointers` OK, 2013 LFS files; 16301 tracked files (16294 + 7 new); no retired spelling outside the archive and ticket records. A workspace-wide `cargo test` fails 24 developer_tools tests on reqwest feature unification (F-028, predates S1) | logs/gate-s1-* |
| 2026-10-02 | S1 commit | `64f16d1da` pushed to `main`; 9415 files changed; the two `.rdb` files left out | — |
| 2026-10-02 | S2 launch | Briefs reviewed and finalised: A1 checks that the library-name text rows shadow no binding before its apply; the staging host still runs the single instance (read-only check), so `--migrate-single-instance` stays and carries the D17 rename; A5 runs its own births manifest; A6 added for documentation. The briefs stay in the run folder until the S2 commit (amendment A6) | s2_drafts/ |
| 2026-10-02 | S2 A1 report | Dry run 1: 32 unresolved literals (synthetic manifest fixtures, relative quotes in `.ai/artifacts/`), fixed before the apply; apply exit 0, planned tree 26/26, verify 54/54; path 22 rows (3864 row moves, 10079 references), rust_path 4 (1648), text 16 (1377); ignored `.env`, `dist/`, `deploy.env` rode along; lock: the 865 packages unchanged apart from the six names; `cargo check --workspace --all-targets --locked`, fmt, crate-tiers, strangler, engine-layers, xtask 1337 ×2, verification_core 187 ×2 OK. The tool now excludes sqlx migrations (they are immutable). 518 leftover hits assigned by owner (394 Markdown) | logs/A1-* |
| 2026-10-02 | S2 wave 2 | A2–A6 launched from their bodies with amendments; A7 added for the relocation tool's false rewrites | prompt_A2..A7.md |
| 2026-10-02 | S2 A5 pause | A5 stopped before its apply as amendment A7 asks: three `.ai/tickets` files outside its ownership and six false slash rewrites (F-030). Amendment A8: the ticket files granted, the six literals restored by hand after the apply and every rewritten file diffed, crate anatomy and tiers hard on both new crates (`error.rs`, a serde-transparent `TerrainId`) | logs/A5-* |
| 2026-10-02 | S2 A7 report | Red first, then fixed: `"http://[::1]/"` and `"deploy/site.service"` no longer rewritten; 43 `relocate_*` tests twice; three perturbations red and restored; the migrations rule proven exact. Clone proof of the S2 manifest at `76aad29d2` without the extra row: one ambiguous literal reported, then clean | logs/A7-* |
| 2026-10-02 | S2 wave 2 reports | A2 (stopped at budget; A2b finished db compose, F-018), A3 (wave execution; four fail-open defects fixed), A4 (Dockerfile builds, `.dockerignore`, compose project names), A5 (two crates born under their own manifest; ticket files granted; no false slash rewrite occurred), A6 (documentation, D17 prose), A7 (tool false rewrites) | logs/A2..A7-* |
| 2026-10-02 | S2 closing batch | G-S2: documentation gates judge every tracked top-level folder; website checkout set gains the compile-time `contracts/` inputs. G2a: API test paths, build-lane runtime, citation roots over every member, CI and wave-gate tests derived from the workspace members. G2b: Caddyfile in `deploy/caddy/`, mounted alone. G2c: executed S1 and S2 briefs archived, documentation. Orchestrator: dev compose project `api` (the live volume `api_tbd_pgdata`), `wave.lock` T-251 row, the births manifest's never-retired README row dropped, the compiler identity pinned as the stored literal it was (F-031), the member-glob test, the testing runbook | logs/G*-* |
| 2026-10-02 | S2 gate (D19) | fmt OK; workspace clippy `-D warnings` (frontend native excluded, F-004) OK; frontend wasm32 116 + 137, equal to the baseline; `ci-local` first run red on one API test that the package rename had rewritten (the stored compiler identity), fixed; rerun green: 188 test binaries, 7715 passed, 0 failed, 9 ignored (every API integration binary, the trunk release build, the documentation gates, the laws, `relocate --verify`); Docker build of `deploy/Dockerfile`; `deploy website --dry-run` and `deploy staging --dry-run` (with and without `--migrate-single-instance`) OK; 16324 tracked and 2013 LFS files | logs/gs-s2-* |
| 2026-10-02 | S2 commit | `24ab925b5` pushed to `main`; 5174 files changed; the two `.rdb` files left out | — |
| 2026-10-02 | M launch | Research: M2 is four folder moves into `Engine/` plus split-literal fixes; the "seven pinned xtask paths" were a research miscount (six name other folders). M1: no reference path is centralised; `verify no-crf-leak` passes on a missing lane, the wave gate calls a nonexistent `make` target, and stale deploy excludes would ship the references. Operator: copy PlayableSelector into `References/`; the orchestrator moves the ignored lanes; the operator regenerates both `.rdb` files in Workbench and they join the M commit; stale worktrees audited. `mod compile` baseline clean (387 TBD files, 0 warnings). Worktrees: T-212, T-939.2, T-946.55, T-946.86 (clean, merged) removed with their branches, and the detached engine-reorg baseline; T-939.4 kept (6 unmerged commits, ticket T-1002); `tbd-s-baseline` kept (milestone S) | logs/m-preflight-* |
| 2026-10-02 | M2 report | Dry run clean; apply: 4 moves, 10 files rewritten; split tails in two schema tests and the destroy-target diagnostics README fixed by hand; `Engine/README.md`; destroy-target-diagnostics, enfusion-comments, file-length, schema and mod-script tests green | logs/M2-* |
| 2026-10-02 | M1 and M1b reports | Lane constants in both layout modules; `verify no-crf-leak` exits 2 on a missing lane (red first), no longer flags the Workshop mod name `@CRF_Framework`, and reads the vanilla paks once (8.3 s, was over 5 minutes); the wave gate runs no `make` step; one `apps/mod/References/` deploy exclude pinned by tests; `.gitignore` ignores the lanes and keeps the README; docs, runbooks and open tickets. Orchestrator moved `crf_framework` and `vanilla_reference` into `References/` and copied PlayableSelector there | logs/M1-*, logs/M1b-* |
| 2026-10-02 | M gate (before OC-mod) | fmt (two files reformatted), workspace clippy, xtask 1374, developer_tools 407, verification_core 188, workspace laws, file-length, enfusion-comments, destroy-target-diagnostics, `relocate --verify`, verify-documentation, `ticket check --strict`, editorconfig: all green. `verify no-crf-leak` runs all three lanes and reports four GUIDs (F-033) | logs/m-gate-* |
| 2026-10-02 | M gate and commit | Operator regenerated both `.rdb` files in Workbench; `mod compile` clean (387 TBD files, 0 warnings, 0.9 s). Decision D20: fast stage gates, the full set once at S12. Both `.rdb` files enter the M commit | logs/m-gate-05-* |
| 2026-10-02 | Parallel setup | M1/M2 committed (`ac4101cd3`). Decision D21: S3–S6 in parallel in detached worktrees, S5 and S6 gated on S4, per-stage logs under `stage_logs/`. Retired root build folders and the incremental cache removed | — |

## Amendments

| Date | Agent | Amendment |
|---|---|---|
| 2026-10-02 | R1 | A2: the tracked-file baseline is 16294 (16226 in the brief predates the S0 commit and the handoff commits); the LFS baseline stays 2013 |
| 2026-10-02 | all S1 and S2 agents | A3: `<scratch>` is `target/api-progress-checkpoint/2026-10-02-restructure-s1/` (gitignored, durable); its `env.sh` puts a cargo shim first on PATH that runs host cargo against the shared cache with git-lfs on PATH, and a podman shim; the two `resourceDatabase.rdb` files are in the foreign baseline |
| 2026-10-02 | R3, R5 | A4: R3 also owns the apps' run-time path pieces and the moved-path API integration binaries; R5 is added to fix the relocation tool's blind spots before S2 (F-021, F-022); the CI task id `developer-tools-test` and the `verification-core-*` temporary-folder prefixes stay (kebab-case ids, not package names) |
| 2026-10-02 | R5 | A5: closed tickets' `owns` entries take path rewrites like `spec` and `plan` (F-023); a file moved into a frozen area takes the frozen treatment (F-024); the clone proof also applies, so the orchestrator copies the archived files and the 10 ticket records from the tool's own output |
| 2026-10-02 | A1–A6 | A6: the S2 briefs live in `<scratch>/s2_drafts/` until the S2 commit, because their planned paths fail link-check before A1 and A1's apply would rewrite the manifest draft inside them; the S2 commit adds them under `agent_briefs/` with the draft replaced by a pointer to the committed manifest |
| 2026-10-02 | A2, A4, A5, A7 | A7: A2 renames the ci task id `website-api-test` to `api-test` and owns the workflows; A4 renames the `website-api-tests-` temporary prefix and the `engineering_laws_website_api_…` test, owns `.gitignore`, and settles the development upload default; A5 applies its births manifest only when every rewritten file is its own; A7 is added for the tool's false rewrites. The deployed unit `tbd-website-api.service` and `website_api_health_check` keep their names (the website's API service, not a package) |
| 2026-10-02 | A6 | A9: runbooks spell no retired path; the one-time OC-deploy moves live in the S2 brief, archived at the S2 commit under `documentation/archive/restructure_agent_briefs/` together with the S1 briefs, and the runbooks link to that section |

## Open findings

Each finding is triaged FIX, NOTE or CLOSE as the orchestration runbook defines; NOTE findings
become tickets at S12.

- **F-001 (FIX, S0 T5; operator: exempt the capitalised `Generated` too): red baseline in readme-coverage.** The 19 category folders under
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Gameplay/Policy/Generated/`
  have no README. The coverage exemption matches only a lowercase `generated` folder. The fix is
  to make the exemption match the capitalised spelling Enfusion uses, or to have the gameplay
  policy generator emit the READMEs. This predates the program. Fixed by T5 (exemption); closes with
  the S0 commit.
- **F-005 (FIX, closing batch): `documentation/standards/where_does_x_go.md:82` names only the
  lowercase `generated` exemption.** Reported by T5; the file was outside its ownership.
- **F-006 (FIX, closing batch): the map engine READMEs still list `earcutr`** (`legacy/map_engine/README.md:103`, `legacy/map_engine/src/README.md:86`). Reported by T2.
- **F-007 (decided, D14): `clippy::unwrap_used` in the workspace lint policy also fires in `#[cfg(test)]` code.** New crates' tests then use `expect`. The alternative is a root `clippy.toml` with `allow-unwrap-in-tests = true`. Reported by T2.
- **F-008 (NOTE, environment): one verification_core `file_length` test fails in this container because it runs as root, which can read a mode-000 file.** The test assumes a non-root user, as CI is. Gates run here record it as environmental. Reported by T2.
- **F-009 (FIX, closing batch): two tools still create root-level build folders.** `tools/xtask/src/commands/mcp/daemon.rs:96` (`target-dev-mcpd`) and `tools/xtask/src/commands/db/operations/selftest.rs:365` (`target-mk-db-selftest`) move under `target/`. Reported by T3.
- **F-010 (FIX, closing batch): a slice-agent brief still tells agents to use `target-<id>-api`** (`slice_agent_brief.md:72`). Reported by T3.
- **F-011 (NOTE, environment): eight developer_tools library tests read Git LFS objects this container has not pulled.** The S0 gate pulls the LFS objects those tests need before running them.
- **F-012 (decided, D13): the name of the parking folder.** The repository's prose rule (`tools/xtask/src/tests/tooling_prose_rules.rs:65`) bans the word "legacy" as history vocabulary, which is the planned folder name of decision D12. The laws T4 wrote name that folder, so one xtask prose test fails now. Either rename the parking folder to a present-tense name, or exempt the folder name from the rule. Reported by T4.
- **F-013 (NOTE, S2): a legacy map engine that re-exports the graphics engine would show about 28 shims.** The S2 and S4 prompts must cut or switch them in the stage that creates them. Reported by T4.
- **F-014 (FIX, before the S0 gate): the xtask prose-rule tests fail on T4's law files.** `tools/xtask/src/tests/tooling_prose_rules.rs` flags the word "legacy" (see F-012) and planned paths such as crates/map_rendering and `prelude.rs` written in prose. Reported by T1b and T4.
- **F-015 (NOTE): a relocation `path` row never rewrites `mod` declarations**; each stage's author edits them. Reported by T1b.
- **F-016 (FIX, done by G0b): `.ai/tickets/T-086.toml` was not in canonical form** (hand-edited in milestone S), so the ticket store's round-trip test failed.
- **F-017 (FIX, done by G0b): `ticket check --strict` rejected plan slice ids** in `documentation/mod/script_modularisation_progress_checkpoint.md` that match the retired priority-backlog id pattern.
- **F-018 (NOTE, environment): the `rust-test-it` task calls `podman` directly** (`tools/xtask/src/commands/ci/task_definitions.rs`), unlike `cargo xtask db test-it`, which resolves the runtime. This container uses a `podman`→`docker` shim. A code fix to resolve the runtime in that task belongs to S2 (A2 owns the db lane).
- **F-019 (NOTE, environment): the outliner-drag and perf browser smokes time out in `Runtime.evaluate` in this container.** Outliner-drag fails identically on the pre-S0 commit; perf shows the same signature. The container's Chromium is 141, while the gate pins 149.
- **F-020 (NOTE, S12 records batch): `.ai/tickets/T-1073.toml` is an open ticket whose bug R1 fixed in S1** (`cases_3.rs:434` climbed two folders from the map engine instead of three, so it missed the repository root). The fixed relocation tool rewrites the open ticket's text, so `relocate --verify` passes; `ticket ship` refuses an idea-tier ticket without its body, so the records batch fills the body and ships it.
- **F-021 (FIX, R5): the relocation tool's dry run reported 0 unresolved while the apply's verify found 5 leftovers.** Paths right after a `\n`/`\0` escape, `file://` URLs and `/../` pieces in macro arguments are not rewritten. Reported by R1.
- **F-022 (FIX, R5): the manifests-folder exclusion also covers its live README**, which is neither rewritten nor verified. Reported by R1.
- **F-023 (FIX, R5 and orchestrator): `.ai/tickets/wave.lock` was rewritten while the `owns` entries of 10 shipped tickets kept their old paths** (T-090.11, T-090.12, T-090.12.6, T-207, T-274, T-320, T-354, T-438, T-584, T-590), so the ticket engine's wave-lock test and `ticket check --strict` fail. Reported by R2 and R4.
- **F-024 (FIX, R5 and orchestrator): files moved into the archive were treated as live by their old path**, so their prose was rewritten and no longer quotes its own history. Reported by R4.
- **F-025 (FIX, S2 A4, D18): the deploy Dockerfile's trimmed workspace cannot build** (no root workspace tables, no copy of the verification core crate). Predates S1. Reported by R3 and the S2 drafter.
- **F-026 (NOTE, OC-deploy): the untracked development `.env` of the API still names the old equipment folder** (`EQUIPMENT_DATA_DIR`). Operator-local. Reported by R3.
- **F-027 (NOTE): the slice-execution stub test failed once with "Text file busy"**, a race that predates the program. Reported by R2.
- **F-028 (NOTE): a workspace-wide `cargo test` fails 24 developer_tools tests with "No rustls crypto provider"** because feature unification adds `rustls-no-provider` to reqwest (fleet host agent, API) while developer_tools installs no provider. CI and `ci-local` test per package, where they pass. Predates S1.
- **F-029 (NOTE, S12): README indexes under the archive and ticket-document folders are live documents** (`documentation/standards/readme_standard.md`), but the relocation tool gives them the frozen treatment, so their Contents blocks need hand fixes after a move (R4 fixed 21; three restored after the clone copy). Reported by R4 and R5.
- **F-030 (FIX, A7): the relocation tool makes two false rewrites its dry run cannot see**: a lone `"/"` Rust literal is read as a path, and a synthetic fixture path is re-anchored when only its leading folder is tracked. Reported by A1.
- **F-031 (FIX, done in S2): the package rename rewrote the stored compiler identity.** `COMPILER_PACKAGE_VERSION` was built from the package name, so the text row changed the value recorded with every artifact and hashed into its digest (seed digests, goldens, tests). It is now the literal `website-map-engine 0.1.0`, the value every stored artifact carries; later stages keep it when the compiler moves crates.
- **F-032 (NOTE): stage manifests are not the place for one-time operator moves.** Runbooks link the archived S2 brief's OC-deploy section for the server-side moves and the GitHub required checks, which stay the operator's until the next deploy.
- **F-033 (NOTE): `verify no-crf-leak` reports four GUIDs that belong to vanilla resources** (the robotomono font in `core/data.pak`, a vest and a radio-slot prefab in `data/data007.pak`, an equipment prefab): CRF references them too, and the vanilla pak probe does not find them. The gate's precision, not a leak; operator: no further work in pre-alpha.
- **F-034 (NOTE): `enf capability` and `enf citations` have no `cargo xtask` wrapper**; the wave gate runs them directly. T-939.4 keeps six unmerged commits (ticket T-1002).
- **F-002 (CLOSE, P0): shallow clone.** The container's clone was shallow, so 353 archive
  permalinks failed link-check as unknown objects. `git fetch --unshallow` in P0 fixed it, and
  link-check passes (2044 checks). A fresh container clones shallow again, so every new session
  unshallows before running the gates.
- **F-003 (CLOSE, P0): the blueprint draft cited planned paths.** Its four backticked planned
  paths broke link-check. Fixed by archiving it as
  [the blueprint draft](/documentation/archive/restructure_research/00_architecture_blueprint_draft.md).
- **F-004 (FIX, done in S3): the frontend was not clippy-clean.** Closed: the frontend is clean on native and wasm32 under `-D warnings`, CI and xtask lint both targets, and stage gates no longer exclude it.
  - The native build has 764 clippy errors under `-D warnings`, mostly imports and code used only by
    the wasm build.
  - The wasm32 build has 253 errors (116 bin, 137 test).
  - CI lints the frontend for wasm32 only, without `-D warnings`, so this predates the program.
  - Until S3, GS step 2 runs with `--exclude frontend`, and step 3 may not raise the wasm
    warning count above the baseline.
  - S3 makes the frontend clean on both targets, because it touches every frontend file. From S3
    the GS steps apply in full.

## Handoff

S3, S4, S5 and S6 run in parallel (decision D21), each in a detached worktree under its own
orchestrator session, following the [stage logs protocol](/documentation/restructure/stage_logs/README.md):

| Stage | Worktree | Launch prompt (in the worktree) |
|---|---|---|
| S3 Frontend in place | `/run/media/system/Disk_2/Projects/tbd-restructure-s3` | `target/restructure-s3/ORCHESTRATOR_PROMPT.md` |
| S4 Tier 0–1 | `/run/media/system/Disk_2/Projects/tbd-restructure-s4` | `target/restructure-s4/ORCHESTRATOR_PROMPT.md` |
| S5 Mission and ballistics | `/run/media/system/Disk_2/Projects/tbd-restructure-s5` | `target/restructure-s5/ORCHESTRATOR_PROMPT.md` |
| S6 World CPU | `/run/media/system/Disk_2/Projects/tbd-restructure-s6` | `target/restructure-s6/ORCHESTRATOR_PROMPT.md` |

S3 is on `main`: the frontend is `src/{foundation, features, pages, workspaces, shell}` with no `src/v2` layer, the frontend-layering law is hard at zero with the foundation sub-area order, and the frontend is clippy-clean on native and wasm32 under `-D warnings`, so every stage gate drops `--exclude frontend`. Its product changes (four inspector panels mounted in Mission Settings, the unmounted placed-vehicles panel deleted) are listed in [stage_logs/s3.md](/documentation/restructure/stage_logs/s3.md) for the S12 walkthrough.

S4 is on `main` in two commits, S4a (the tier 0–1 crates S5 and S6 need) and S4b (tool foundations
and contract crates). Duplicates S4 left in place, each with the stage that switches it, are in the
"Handoff duplicates" table of [stage_logs/s4.md](/documentation/restructure/stage_logs/s4.md).

Every remaining stage (S3–S12, M3) has its own worktree and orchestrator, managed by the
"Restructure coordinator" session (decision D22; inputs per stage in the stage logs protocol). The
launch prompts live in the gitignored run folders; if a worktree is lost, rebuild it with
`GIT_LFS_SKIP_SMUDGE=1 git worktree add --detach <path> main` and rewrite its prompt from this table
and the stage logs protocol.

Operator items left from S2: the server-side moves and `deploy staging --migrate-single-instance`
before the next deploy, and the GitHub required checks `api (Rust 1.95 + Postgres 18)` and
`frontend (Leptos SPA)`. M3 (objective behaviours) can run any time.

Machine notes (this workstation). Each worktree's `env.sh` puts two shims from
`~/.cache/tbd-bin/restructure-shims/` first on PATH: a `cargo` that runs host cargo with git-lfs on
the PATH, and a `podman` that reaches the host. Each worktree builds in its own folder
(`CARGO_TARGET_DIR=~/.cache/tbd-target-<stage>`), because a shared folder let `cargo xtask` run
the xtask binary another worktree linked last (finding F-S6-03). `git push` runs in the container. Prune
the build folder's `debug/incremental` and stale API test executables before a full
`db test-it`.
