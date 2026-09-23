**Status:** live — resume file for the Documentation V2 program

# Documentation V2 progress checkpoint

The resume file for the Documentation V2 program: the baseline taken before any move, one roster
row per planned agent, and the operator checkpoints. The orchestrator updates a roster row when it
commits that agent's work; each verifier updates the rows of the phase or wave it verifies. The plan
is [refactor_program_plan.md](/documentation_v2/refactor_program_plan.md); the rules every agent
follows are in [refactor_writing_brief.md](/documentation_v2/refactor_writing_brief.md).

## Resume instructions

Read this file, then the program plan, then the writing brief. Continue the roster in order from the
first row that is not `done`: rows run in the order listed, and rows the plan runs in parallel (P0-2
to P0-4, P2-5 with P2-6, R01 with F01, the writers of one wave, G1 with G2) may run together, at
most four writers at a time. Stop at each checkpoint in the Checkpoints table and wait for the
operator's sign-off before starting the row after it. Before P2-1, get the operator's confirmation
that no other session writes under `docs/` and that `git status -- docs` is clean.

Roster statuses: `pending`, `running`, `done — awaiting commit`, `fix-list` (a verifier's FIX-LIST
is open), `done` (the commit column holds the commit).

## Baseline (2026-09-23)

P0-1 ran each command once from the repository root through `hcargo`, the host cargo wrapper the
base brief names, one command per call. The logs sit under `logs/P0-1/` in the program scratchpad
the base brief names, outside the repository.

| Command | Exit | Log | Failure excerpt |
|---|---|---|---|
| `hcargo xtask ticket check --strict` | 0 | `logs/P0-1/a_ticket_check_strict.log` | none: `check OK`, with 10 warnings that an `acceptance` item is command-shaped (T-853, T-912.1, T-912.2, T-913.1, T-915.1, T-916.2, T-917.1, T-919, T-920.1, T-921) |
| `hcargo test -p xtask` | 101 | `logs/P0-1/b_test_xtask.log` | 773 passed, 3 failed: the three expected `tooling_prose_rules` failures, quoted below |
| `hcargo test -p ticket-engine` | 0 | `logs/P0-1/c_test_ticket_engine.log` | none: 206 unit tests and 1 trybuild test passed |
| `hcargo test -p developer-tools` | 0 | `logs/P0-1/d_test_developer_tools.log` | none: 263 passed, 4 ignored |
| `hcargo test -p ticketboard` | 0 | `logs/P0-1/e_test_ticketboard.log` | none: 180 passed, 3 ignored |
| `hcargo test -p verification-core` | 0 | `logs/P0-1/f_test_verification_core.log` | none: 68 unit tests and 1 doc-test passed |
| `hcargo xtask ci verify-coding-standards` | 1 | `logs/P0-1/g_verify_coding_standards.log` | `ROUTE-TAG CHECK: FAIL — 11 unwired tag(s), 0 undocumented route(s)`; doc layout, `file-length` (3271 `.rs` files, 0 violations) and `no-select-star` pass |
| `hcargo xtask mod compile` | 0 | `logs/P0-1/h_mod_compile.log` | none: `OK: compiled clean`, 0 warnings in TBD sources |

### Baseline failures and findings

- `tooling_prose_rules::every_rust_file_named_in_prose_exists`, expected; P1-1 clears it by
  archiving the tools_v2 program records. First line:
  `prose names a Rust file that is nowhere in the workspace:` then
  `tools_v2/PHASE_FIVE_HANDOFF.md:2093: agent.rs`. All six hits are `agent.rs` in that file (lines
  2093, 2823, 3000, 3039, 3183 and 3188).
- `tooling_prose_rules::nothing_narrates_its_own_history`, expected; it belongs to the other
  session's equipment export code, committed in `ba275f244`. First line:
  `prose narrates a past state; describe the present one:` then
  `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/legacy_archive.rs:10: const JOURNAL: &str = ".legacy-archive.json";`.
  The hits are 8 lines in `legacy_archive.rs` and 8 in the same folder's `tests/validation.rs`.
- `tooling_prose_rules::only_a_layout_module_spells_a_repository_path`, expected; P1-4 clears it.
  First line: `a production source spells a repository path; put it in its crate's layout module:`
  then `tools_v2/xtask/src/verifications/api_readiness/fingerprint.rs:17: "docs/verification/api_v2/",`
  and `tools_v2/xtask/src/verifications/api_readiness/register.rs:10`.
- `verify-coding-standards` route tags, not expected; it comes from another session's uncommitted
  work. The 11 unwired tags sit on the untracked handlers under
  `apps/website/api_v2/src/community_content/handlers/equipment_data_viewer/`; the first is
  `source_inspection.rs:10  @route GET /api/v1/debug/equipment-data/containers  ->  handler
  containers is NOT registered in the api_v2 domain route tables on that method+path`. The handlers
  are mounted by an uncommitted `.merge(handlers::equipment_data_viewer::routes())` line
  (`apps/website/api_v2/src/community_content/routes.rs:18`): a sub-router the route-tag check does
  not read, since it reads only the eight domain `routes.rs` files.
- `ticket check --strict` passes although two child tickets cite `spec` files that do not exist:
  `T-068.10.5` (`docs/specs/Mission_Creator_Architecture/t068_10_5_weapon_variants.md`) and
  `T-159.15.0` (`docs/platform/t159_15_render_loop_handoff.md`). Both registry loaders admit only
  parent-id files as tickets (`tools_v2/ticket-engine/src/registry/typed_projection.rs:167`,
  `tools_v2/ticket-engine/src/registry/ticket_file_storage/storage.rs:114`) and the "spec missing on
  disk" rule walks registry tickets (`tools_v2/ticket-engine/src/validation/runner.rs:116`), so no
  child file's `spec` is ever checked. They are the only 2 of the 488 child files with a missing
  spec. Their `citations` name `t068_10_5_weapon_families.md` and `t159_15_render_loop_handoff.md`,
  which exist under `.ai/artifacts/`.
- A re-run of `hcargo xtask ticket check --strict` after the program files were written
  (`logs/P0-1/a2_ticket_check_strict_after_files.log`) exits 101 at compile time: another session is
  writing `tools_v2/xtask/src/commands/mod_ops/equipment_gameplay/` (untracked), whose `mod.rs`
  declares `policy_codegen`, `projection` and `resource_projection` before their files exist. All
  eight baseline runs finished before that edit began. None of the eight baseline commands reads
  `documentation_v2/`, so the program files cannot change a baseline result.

## Roster

| id | phase | role | status | commit | open questions |
|---|---|---|---|---|---|
| P0-1 | 0 | Program files and baseline | done | f8511271c | Personal paths in the program files replaced with `<repo>`, `<scratchpad>` and `$HOME` placeholders before the commit; `ticket check --strict` never checks child-ticket `spec` paths (see the baseline findings) — operator question pending |
| P0-2 | 0 | Manifest, mechanical rows | pending | — | — |
| P0-3 | 0 | Manifest, judgment rows | pending | — | — |
| P0-4 | 0 | Ticket plan and follow-up tickets | pending | — | — |
| P0-5 | 0 | Manifest assembly and pin catalogue | pending | — | — |
| P1-1 | 1 | Archive the tools_v2 program records | pending | — | — |
| P1-2 | 1 | Retire the generated ticket views | pending | — | — |
| P1-3 | 1 | Retire the prose gates | pending | — | — |
| P1-4 | 1 | Api-readiness constants and layout existence tests | pending | — | — |
| P1-5 | 1 | Documentation gate modules and verbs | pending | — | — |
| P1-6 | 1 | Phase 1 verifier | pending | — | — |
| P2-1 | 2 · commit 2a | Mover | pending | — | — |
| P2-2 | 2 · commit 2a | Code and config pins | pending | — | — |
| P2-3 | 2 · commit 2a | Tickets | pending | — | — |
| P2-4 | 2 · commit 2a | Verifier for 2a | pending | — | — |
| P2-5 | 2 · commit 2b | Link rewriter, frozen corpus | pending | — | — |
| P2-6 | 2 · commit 2b | Link rewriter, live corpus | pending | — | — |
| P2-7 | 2 · commit 2b | Verifier for 2b | pending | — | — |
| P3-1 | 3 | README standard and templates | pending | — | — |
| P3-2 | 3 | Documentation standards, glossary skeleton and entry README | pending | — | — |
| R01 | 3 · pilot | README writer: api_v2 crate root, migrations, seeds, `src/` root, `src/bin` and every domain except missions and operations | pending | — | — |
| F01 | 3 · pilot | Doc writer: `website/frontend/pages/administration/**` | pending | — | — |
| P3-3 | 3 · pilot | Pilot verifier over R01 and F01 | pending | — | — |
| R02 | 4 · W4.1 | README writer: api_v2 `src/{missions, operations}` | pending | — | — |
| R03 | 4 · W4.1 | README writer: frontend crate root and non-src folders, `src/` root, `src/v2/` root, `src/v2/core/**` | pending | — | — |
| R04 | 4 · W4.1 | README writer: `src/v2/pages/` root, `pages/{command_center, operations, mission_hub}/**` | pending | — | — |
| R05 | 4 · W4.1 | README writer: `pages/{administration, doctrine_and_info, field_tools, account, navigation}/**` | pending | — | — |
| V4.1 | 4 · W4.1 | Wave verifier over R02–R05 | pending | — | — |
| R06 | 4 · W4.2 | README writer: `src/v2/apps/editor/ui/**` | pending | — | — |
| R07 | 4 · W4.2 | README writer: `src/v2/apps/` root, `editor/` root and its children except `ui/`, `apps/{debug, planner, aar}/**` | pending | — | — |
| R08 | 4 · W4.2 | README writer: map-engine `src/data/**` | pending | — | — |
| R09 | 4 · W4.2 | README writer: map-engine `src/{world, streaming, spatial}/**` | pending | — | — |
| V4.2 | 4 · W4.2 | Wave verifier over R06–R09 | pending | — | — |
| R10 | 4 · W4.3 | README writer: map-engine crate root, `src/` root and every other `src/` folder | pending | — | — |
| R11 | 4 · W4.3 | README writer: `apps/README.md`, `apps/website/` root, `graphics-engine/**`, `apps/website/shared`, `contracts_v2/**`, `assets_v2/**` | pending | — | — |
| R12 | 4 · W4.3 | README writer: tbd-framework `Scripts/Game/TBD/Systems/**` | pending | — | — |
| R13 | 4 · W4.3 | README writer: tbd-framework `Scripts/Game/TBD/{Session, API, Core}/**` | pending | — | — |
| V4.3 | 4 · W4.3 | Wave verifier over R10–R13 | pending | — | — |
| R14 | 4 · W4.4 | README writer: `apps/mod/` root, tbd-framework root and script root levels, `{Gamemode, UI}` scripts, asset folders, `tbd-emcp/**` | pending | — | — |
| R15 | 4 · W4.4 | README writer: `apps/mod/tbd-export/**` | pending | — | — |
| R16 | 4 · W4.4 | README writer: xtask `src/commands/{platform, mod_ops}/**` | pending | — | — |
| R17 | 4 · W4.4 | README writer: xtask crate root, `deploy/**`, `dedicated_server_profiles/**`, `fixtures/**`, `src/` root, `src/{core, cli}/**`, the other command groups | pending | — | — |
| V4.4 | 4 · W4.4 | Wave verifier over R14–R17 | pending | — | — |
| R18 | 4 · W4.5 | README writer: xtask `src/verifications/**` | pending | — | — |
| R19 | 4 · W4.5 | README writer: developer-tools `src/{browser_testing, blueprint}/**` | pending | — | — |
| R20 | 4 · W4.5 | README writer: developer-tools crate root, the remaining `src/**`, `src/bin`, `fixtures/**`, `test_fixtures/**` | pending | — | — |
| R21 | 4 · W4.5 | README writer: `tools_v2/` root, `ticket-engine/**`, `verification-core/**`, `enfusion_mcp_node_package/`, `apps/ticketboard/**`, `apps/fleet_host_agent/**` | pending | — | — |
| V4.5 | 4 · W4.5 | Wave verifier over R18–R21 | pending | — | — |
| F02 | 5 · W5.1 | Doc writer: `pages/{command_center, operations}/**` | pending | — | — |
| F03 | 5 · W5.1 | Doc writer: `pages/{mission_hub, doctrine_and_info, field_tools}/**` | pending | — | — |
| F04 | 5 · W5.1 | Doc writer: `pages/{account, navigation}/**`, `website/frontend/README.md`, frontend core docs | pending | — | — |
| F09 | 5 · W5.1 | Doc writer: `website/README.md`, `website/api_v2/**`, `website/map-engine/**`, `website/graphics-engine/**`, `standards/engine_boundary_rules.md` | pending | — | — |
| V5.1 | 5 · W5.1 | Wave verifier over F02, F03, F04 and F09 | pending | — | — |
| F05 | 5 · W5.2 | Doc writer: Mission Creator feature inventory, part 1 (areas A–M) and its folder README | pending | — | — |
| F06 | 5 · W5.2 | Doc writer: Mission Creator feature inventory, part 2 (areas N–Z) | pending | — | — |
| F07 | 5 · W5.2 | Doc writer: Mission Creator roadmap, `ux_spec.md`, `decisions.md`, Eden gap analysis | pending | — | — |
| F08 | 5 · W5.2 | Doc writer: Eden reference catalogs, arsenal, editor visual references, `apps/{planner, aar}/`, editor folder README | pending | — | — |
| V5.2 | 5 · W5.2 | Wave verifier over F05–F08 | pending | — | — |
| F10 | 5 · W5.3 | Doc writer: `mod/tbd-framework/**` (mod design, capability verdicts index, `UI/**`) | pending | — | — |
| F11 | 5 · W5.3 | Doc writer: `mod/README.md`, `mod/tbd-export/**`, `mod/tbd-emcp/**` | pending | — | — |
| F12 | 5 · W5.3 | Doc writer: `tools_v2/**`, `ticketboard/**`, `fleet_host_agent/**`, `contracts_v2/**`, `assets_v2/**` docs | pending | — | — |
| F13 | 5 · W5.3 | Doc writer: runbooks for local development, website deployment, database operations, testing and CI | pending | — | — |
| V5.3 | 5 · W5.3 | Wave verifier over F10–F13 | pending | — | — |
| F14 | 5 · W5.4 | Doc writer: runbooks for editor gates, editor capture, Enfusion MCP tooling, spawn determinism, game server staging | pending | — | — |
| F15 | 5 · W5.4 | Doc writer: runbooks for the two-client playtest and the mod slice workflow | pending | — | — |
| F16 | 5 · W5.4 | Doc writer: runbooks for factory waves, the ticket run pipeline, Cursor workspace setup | pending | — | — |
| F17 | 5 · W5.4 | Doc writer: coding standards, where-does-x-go, commit checklist, ticket identifiers, `known_bugs/**`, `design_system/**` | pending | — | — |
| V5.4 | 5 · W5.4 | Wave verifier over F14–F17 | pending | — | — |
| F18 | 5 · W5.5 | Doc writer: folder-index READMEs for every documentation_v2 folder no other writer owns | pending | — | — |
| F19 | 5 · W5.5 | Doc writer: `glossary.md`, `product_roadmap.md`, final pass on `documentation_v2/README.md` | pending | — | — |
| V5.5 | 5 · W5.5 | Wave verifier over F18 and F19 | pending | — | — |
| F19b | 5 · after CP4 | Apply the operator's CP4 strikes to `product_roadmap.md` | pending | — | — |
| G1 | 6 | `CLAUDE.md` full refresh | pending | — | — |
| G2 | 6 | Cursor rules and `.ai/tickets` instruction files refresh | pending | — | — |
| G3 | 6 | Verifier over G1 and G2 | pending | — | — |
| H1 | 7 | Documentation gates into `ci-local` and `ci.yml` | pending | — | — |
| H2 | 7 | Full verification matrix, program files archived, close-out | pending | — | — |

## Plan amendments

| date | amendment | source |
|---|---|---|
| 2026-09-23 | P1-2 also extends ticket validation so every ticket file (parent and child) gets the spec/plan existence check, and first repoints T-068.10.5 and T-159.15.0 at their real `.ai/artifacts/` files so the check stays green until the cutover moves them | operator decision |
| 2026-09-23 | Personal absolute paths in the program files were replaced by `<repo>`, `<scratchpad>` and `$HOME` placeholders; launch prompts carry the concrete values | orchestrator |
| 2026-09-23 | Mod UI screenshots and Stitch sets are mapped to per-screen folders by P0-3 (judgment rows), not by P0-2 | orchestrator |
| 2026-09-23 | The operator compacts the main session at every phase boundary (and at wave boundaries in Phases 4–5). Before each boundary the orchestrator writes a "Phase handoff" block below, then signals "safe to compact"; compaction happens only when no agent is running | operator decision |

## Phase handoffs

Written by the orchestrator at each phase (and Phase 4–5 wave) boundary, before the main session is compacted. Each block names: commits landed, decisions and amendments, environment state (other sessions, xtask build status, dirty files), open questions, and the exact next agents to launch with any prompt adjustments. A fresh session resumes from the newest block.

(none yet — Phase 0 in progress)

## Checkpoints

| Checkpoint | Follows | The operator reviews | Status |
|---|---|---|---|
| CP1 manifest | P0-5 | `refactor_move_manifest.md` and its questions; P0-5 applies the answers in a fix-agent run | pending |
| CP2 cutover | P2-7 | commits 2a and 2b; ticketboard's spec reader; the server address, identified by its SSH host key | pending |
| CP3a standard | P3-1 | the README standard and its samples; changes go through a P3-1 fix run | pending |
| CP3b pilot slices | P3-3 | the R01 and F01 pilot output; the style locks | pending |
| CP4 roadmap | V5.5 | `product_roadmap.md`; the operator strikes unplanned items and F19b applies the strikes | pending |
