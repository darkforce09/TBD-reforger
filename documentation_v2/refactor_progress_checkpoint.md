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
| P0-2 | 0 | Manifest, mechanical rows | done | — (scratchpad outputs; committed by P0-5) | CP1: set names, dashboard/server_intel set placement, tbd-export evidence folder case clash with code `Verification/`, near-duplicate collapses (4 DESIGN.md trailing newline, Arsenal header) |
| P0-3 | 0 | Manifest, judgment rows | done | — (scratchpad outputs; committed by P0-5) | CP1: 11 questions in judgment_notes.md (vanilla_carve_coverage, SHIPPED_HISTORY topic, macos_ux_architecture, frontend _template, CLAUDE-CODE-START, eden wiki_manifest, KB-002, admin_tickets panel, feature doc file naming, frontend README primary, pending_merge naming) |
| P0-4 | 0 | Ticket plan and follow-up tickets | done (step 3 blocked) | — (scratchpad outputs; committed by P0-5) | CP1: top-level ticket ids exhausted at T-999 (schema.json:18 and sync/gap_analysis.rs:12 allow 3 digits) — widen or file as children; citation value format; rename ticket status deferred vs queued; rewrite bare-name mentions only where the basename changes |
| P0-5 | 0 | Manifest assembly and pin catalogue | done (≈645k tokens — over the 500k cap) | 660b70833 | 21 CP1 questions in refactor_move_manifest/cp1_questions.md |
| P0-5f | 0 | Apply the checkpoint-1 answers to the manifest (fix run) | done (≈256k tokens) | f79a84a42 | — |
| P1-1 | 1 | Archive the tools_v2 program records | done (≈145k tokens) | this commit | — (`every_rust_file_named_in_prose_exists` passes; the two remaining `tooling_prose_rules` failures are unchanged from the baseline; pin catalogue rows 77–82 applied, the `tools_v2/README.md` links now sit on lines 14, 16 and 19–23) |
| P1-2 | 1 | Retire the generated ticket views | done (≈415k tokens) | this commit | 3 questions answered (see amendments). Pin catalogue drift: row 102 gives the old watch list as TICKETS_DIR, TREE_DIR, ROADMAP — the optional list was repo root, docs tree, roadmap directory (ticket dir armed separately); P2-2 rows 193–194 (ROADMAP paths in the `file_watch` tests) now sit at :115/:119 |
| P1-2f | 1 | Fix run: lines that still describe the retired views (committed with P1-2) | done (≈135k tokens) | this commit | Test renamed `ship_regenerates_queue_from_post_state_reload_pin`. For G2: `.ai/tickets/AI_PLAYBOOK.md:9` names the marker `<!-- ticket-sync:next -->`; the real one is `<!-- ticket-sync:next:start -->` (`validation/constants.rs:16`) |
| P1-2c | 1 | Retire the `ticket milestone` verb (operator decision on a P1-2 question) | done (≈116k tokens) | this commit | Kept the `milestone` key: it belongs to `FROZEN_27` (`key_contract.rs:5-34`), the frozen record of the untyped ticket-file keys; the live allow-list (`ENCODING_C_KEYS` ∪ `ALLOWED_NEW`) already rejects it. Not ours: `mod_ops::equipment_gameplay::tests::gameplay_preserves_numeric_tokens_and_rejects_broken_publication` fails in the other session's untracked module; `fmt --check` diffs in its `equipment_vehicle_export/publication.rs:98` and `mod_ops/mod.rs:1` |
| P1-2b | 1 | Widen the ticket id pattern to 3+ digits; file the three follow-up tickets | done (≈423k tokens) | this commit | Filed T-1000 (scenario-to-mission rename, deferred, `order = 9811` hand-set), T-1001 (mod wave gate Makefile, idea), T-1002 (branch/worktree/status cleanup, idea, executor human), T-1003 (retire the untyped ticket-tree load path, idea). One shared id-order key, `ticket_engine::store::ticket_id_order_key`; `wave.lock` wave 0 gains T-1000 and T-1002. Still in plain string order, cosmetic only: `Corpus.tickets` key order, the `wave.lock` `[owns]`/`[depends_on]` tables, ticketboard `measured/aggregation.rs:139` |
| P1-2d | 1 | Fix run: `ticket set-status deferred|cancelled` on a ticket without an `order` leaves the registry red (tooling gap found by P1-2b) | done (≈221k tokens) | this commit | Mint: deferred, cancelled, shipped (also `ship`/`done`) get `append_order` (`ops/ordering.rs:87`); refuse: queued, ready, running, review; children stay order-less; a pre-write rule (`ops/validation.rs:115`) refuses a changed parent left without an order. Polish for the P1-6 fix run: garbled phrase in the `ship` doc comment in `ops/transitions.rs` ("the full-corpus map is the a dotted child id resolves here"); `cli/shipping.rs:48` comment does not mention the minting |
| P1-3 | 1 | Retire the prose gates | done (≈268k tokens) | this commit | Also edited `ticket-engine/src/sync/tests/gap_analysis_tests.rs:11` (it builds `CorpusPins`; kept). `schema-validate` now runs six sub-gates (~5 s warm). Pin catalogue rows 125–140 applied. Launch additions: checks `ticket check --strict`, the empty-grep proof, file sizes; deletions by plain `rm` |
| P1-3f | 1 | Fix run: comments made stale by P1-3 and P1-2d (committed with P1-3) | done (≈107k tokens) | this commit | Polish for the P1-6 fix run: the `ship` doc (`ops/transitions.rs`) and the `cli/shipping.rs` comment say the SHA "stays hand-edited", but `ticket stamp-sha` writes `shipped_at` |
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
| 2026-09-23 | The orphan-spec decisions are applied by P2-3 (tickets), not P2-2; the P0-4 section of the plan names P2-2 by mistake | P0-4 finding |
| 2026-09-23 | P2-4's `docs/` grep also excludes `.ai/tickets/` (14 retired wave-plan mentions and other kept strings stay by design) | P0-4 finding |
| 2026-09-23 | P2-1 accepts 5 near-duplicate collapses (4 DESIGN.md copies differing by a trailing newline, Arsenal/DESIGN.md differing by a 2-line header); the manifest note records each difference | P0-2 finding |
| 2026-09-23 | Follow-up tickets are filed after xtask compiles again and after the operator rules on the three-digit id limit (CP1) | P0-4 finding |
| 2026-09-23 | P1-1 also owns tools_v2/README.md (lines 13-18 link the records it moves) | P0-5 finding |
| 2026-09-23 | P1-5's link and backticked-path checks treat `documentation_v2/refactor_*` program files as records (they spell old and future paths by design); once archived, the archive rules apply | P0-5 finding |
| 2026-09-23 | P2-2 moves the `.editorconfig-checker.json` excludes from the two docs/specs mockup folders to the new `visual_references/` and `design_system/token_exports/` locations (generated exports, byte-pinned) | P0-5 finding |
| 2026-09-23 | The committed plan copy spelled the retired wave-plan fossil strings; reworded (the fossil guard in `ticket check` would fail on them) | P0-5 finding |
| 2026-09-23 | Size agents tighter: P0-5 finished at ≈645k tokens. Scripted agents get narrower scopes; the orchestrator stops any agent nearing 450k | orchestrator |
| 2026-09-23 | CP1 answered: the operator accepted the recommended answer to all 21 questions; P0-5f applies 1–3 (set rename, per-page dashboard and server-intel sets, `verification_evidence/` folders) and records the decisions in the writing brief | operator decision |
| 2026-09-23 | New agent P1-2b (78 agents in total): after P1-2, widen the ticket id pattern in `.ai/tickets/schema.json:18` and `tools_v2/ticket-engine/src/sync/gap_analysis.rs:12` to three or more digits (tests included), then file the three follow-up tickets from `documentation_v2/refactor_followup_tickets.md` top-level: the scenario-to-mission rename (status deferred), the mod wave gate's missing Makefile, and one human cleanup ticket for the 44 stale branches, 6 worktrees and the 4 merged-but-ready tickets | operator decision (CP1 question 17) |
| 2026-09-23 | Launch prompts: each agent gets its plan prompt (role, grant, ownership, checks) with the placeholders replaced by concrete values and the base brief given by pointer, plus the amendments in this table, plus any ownership or check that the orchestrator's pre-launch scoping of the code finds missing. Every such addition is logged in this table with the agent's id before or with its commit | operator decision |
| 2026-09-23 | P1-1 launch additions: exact targets from manifest rows 1066–1072; the moved files stay byte-identical (no status line, no link edits; P2-5 does both in commit 2b) so git records pure renames; the two README edits use repo-root links to the archived files and must not present the archived plan as live authority; extra checks `hcargo xtask ticket check --strict`, `git status --porcelain -- tools_v2 documentation_v2/archive` and `git diff -M --cached --stat` (7 renames, no content change); if the other session breaks the xtask build, record the compiler error and report without touching their code | orchestrator pre-launch scoping |
| 2026-09-23 | New fix run P1-2f (orchestrator review of P1-2): lines that still describe the retired views — ticket-engine `Cargo.toml:8` (description), `src/lib.rs:1`, `src/cli/tests/mod.rs:240`, `src/cli/mutation_support.rs:36-38`, `src/cli/status.rs:92-95`, `src/cli/shipping.rs:84` (comments; present tense, no ticket references); `.ai/tickets/AI_PLAYBOOK.md:9`, `.ai/tickets/README.md:3`, `.ai/tickets/CLAUDE_CODE_PROMPT.md:74`, `.ai/tickets/SPEC_TEMPLATE.md:113`, `.cursor/rules/tbd-platform.mdc:15`, and `CLAUDE.md:178,238` (dirty file: those two lines only, staged hunk-wise). Ahead of G1/G2's full refresh, because live instructions otherwise name deleted files. Committed together with P1-2 | orchestrator review |
| 2026-09-23 | P1-2 questions answered: (1) ticketboard's file watch stays as it is — it never watched the gap-analysis file, and its git-status panel now lists that file's changes; (2) retire `cargo xtask ticket milestone` now — new agent P1-2c after P1-2f removes the verb (`ticket-engine/src/cli/queries.rs` `cmd_milestone`, its re-export at `cli/mod.rs:27`, xtask `commands/ticket/{cli.rs, dispatch.rs}`), its tests and every helper only it uses, and the `milestone` key in `registry/ticket_file_storage/key_contract.rs` if nothing else reads it (0 of 1,458 ticket files carry it); own commit; (3) the spec and plan existence check keeps its reach: every status except idea and cancelled | operator decision |
| 2026-09-23 | P1-2c launch additions: ownership of xtask `commands/ticket/{cli.rs, dispatch.rs, README.md, tests/**}` and any ticket-engine test that names the verb; the `milestone` key in `registry/ticket_file_storage/key_contract.rs` sits in a list marked FROZEN, so the agent removes it only if the list is the allow-list of current keys, and keeps it (reporting why) if the list is a compatibility contract for parsing existing files; plus `ticket-engine/src/cli/mutation_support.rs:33-34`, which cites a design document in a source comment (law 8), reworded in the same pass | orchestrator pre-launch scoping |
| 2026-09-23 | **Operator delegation (overnight run).** The operator is unavailable and has told the orchestrator to keep going until the program is done. From here the orchestrator: answers every agent question itself (the recommended option, backed by code evidence); sends every problem to a fix agent without waiting; reviews and decides CP2, CP3a and CP3b itself, with a verifier agent's verdict; decides CP4 non-destructively (items that look unplanned move to an "unscheduled" section of `product_roadmap.md`, never deleted); keeps the live server-address check of CP2 open for the operator (no remote commands); before P2-1 replaces the operator's confirmation with checks (`git status -- docs` clean, other local sessions listed and told to hold `docs/` writes); keeps one commit per agent and the phase handoff blocks (auto-compaction replaces the manual compaction). Every decision taken this way is logged in this table as "orchestrator decision (delegated)" for the operator's review | operator decision |
| 2026-09-23 | P1-2b launch additions: besides the two patterns, ids of four or more digits must sort after T-999. Six places break ties by comparing id strings, which puts T-1000 between T-100 and T-101: `registry/mod.rs:222` (`ticket_sort_key`), `sync/markers.rs:53`, `wave_lock/packing.rs:86`, `wave_lock/collisions.rs:189`, `metrics/summary.rs:29`, `registry/ticket_file_storage/storage.rs:147`. P1-2b owns them and their tests and gives them one shared id-order key (numeric parent id, then the full id string), which keeps every existing id in its current order, so queue.json, the roadmap block and wave packing stay byte-identical for today's corpus. It audits the numeric id parsers (`store.rs:40,142`, `ticket_file_storage/encoding.rs:32`, xtask `wave_execution/gate/gate_dispatch.rs:57`, `wave_execution/test_cmd.rs:71,242`, ticketboard `projection.rs:43`) and changes them only if one assumes three digits. It also owns the new ticket files `ticket add` writes and what `ticket sync` then rewrites | orchestrator pre-launch scoping |
| 2026-09-23 | P1-2c question (retire `FROZEN_27`, `frozen_27_matches_live_corpus` and `union_ticket_keys`, whose test always returns early on today's all-typed corpus?): keep them while `load_registry` (`registry/mod.rs:17-31`) still has its untyped path; P1-2b files a fourth follow-up ticket, status idea, to retire the untyped ticket-tree load path together with those three | orchestrator decision (delegated) |
| 2026-09-23 | P1-2b questions: (1) keep T-1000's hand-set `order = 9811`, the value `ticket reorder T-1000 T-981` mints (no verb could run while `set-status deferred` had left the registry red); (2) keep the three extra ordering sites (`registry/typed_projection.rs:184`, `wave_lock/parking.rs` `pack_last` sort and `wave_zero` list), the same fault as the six named ones. P1-2b's tooling gap is fixed now by a new fix run, P1-2d: `set-status` to `deferred` or `cancelled` mints the append order (`reorder` after the highest-ordered ticket) when the ticket has none, because order carries no dispatch meaning there; `queued` keeps its refusal (order is dispatch priority); no ticket verb may leave `ticket check` red | orchestrator decision (delegated) |
| 2026-09-23 | P1-2d decisions: keep the `ship`/`done` fix (same missing-order bug as `set-status shipped`); keep the pre-write order rule, which now refuses the one edge where `reorder` anchored on a negative order would write order 0 (no current ticket is affected; orders run 10–9811) | orchestrator decision (delegated) |
| 2026-09-23 | P1-3 questions: (1) the retired `specification_consistency` gate 7 checked that cited commands exist; P1-5b's link checker takes that over for live docs — every backticked `cargo xtask <group> [<verb>]` must name an existing subcommand path in the xtask command tree (placeholders and flags ignored); (2) keep P1-3's one-line edit to `ticket-engine/src/sync/tests/gap_analysis_tests.rs`. New fix run P1-3f: `xtask/src/cli/mod.rs:88` (`schema --help` headline), `commands/ci/task_runner.rs:114` (sub-gate count), `commands/db/tests/operations/tests.rs:4-7` (`LANE_COMMANDS` reader), plus P1-2d's polish items (`ticket-engine/src/ops/transitions.rs` `ship` doc phrase, `cli/shipping.rs:48`) | orchestrator decision (delegated) |
| 2026-09-23 | P2-5 and P2-6: `git mv` keeps link text, so a relative link inside a moved file is resolved against the file's pre-move path (the manifest's source column) before it is rewritten. P1-1's archived records hold three such links: `archive/tools_v2_refactor/analysis_and_inventory.md:5` and `architecture_plan.md:5` (sibling names that are now snake_case) and `phase_one_handoff.md:12` (`../docs/tools/editor_capture.md`, manifest row 816) | P1-1 finding |
| 2026-09-23 | P1-2 launch additions: ownership of `tools_v2/ticket-engine/src/metrics/estimates/git_changes.rs` (the only other user of `GENERATED_QUEUE_VIEW_PREFIX`, which the plan renames `RETIRED_QUEUE_VIEW_PREFIX`); `tools_v2/ticket-engine/src/validation/**` and its tests (the child-ticket spec and plan check), with additive-only changes in `tools_v2/ticket-engine/src/registry/ticket_file_storage/` if validation needs a helper that lists child ticket files; the `spec` field of `.ai/tickets/T-068.10.5.toml` and `.ai/tickets/T-159.15.0.toml` (through a `ticket` verb if one sets it); the view claims at `tools_v2/README.md:6` and `tools_v2/ticket-engine/README.md:3,12` (the plan's `src/README.md` does not exist; the crate README is meant); one extra check, `hcargo xtask ticket sync`, which must write no view file, with any queue.json, ROADMAP-marker or gap-analysis diff listed as sync output. Added at launch: a `git rm` grant for exactly the eight deletions (`sync/queue_views.rs`, `sync/registry_views.rs`, the five `docs/TICKET_*.md` views, `docs/MILESTONES.md`); ownership of `tools_v2/ticket-engine/src/sync/tests/**` in case a test names a removed item; two more checks, `hcargo build -p xtask` (downstream crate) and `hcargo test -p xtask tooling_prose_rules` (P1-2 edits tools_v2 Markdown; only the two baseline failures may remain) | orchestrator pre-launch scoping |

## Phase handoffs

Written by the orchestrator at each phase (and Phase 4–5 wave) boundary, before the main session is compacted. Each block names: commits landed, decisions and amendments, environment state (other sessions, xtask build status, dirty files), open questions, and the exact next agents to launch with any prompt adjustments. A fresh session resumes from the newest block.

### Phase 0 handoff (2026-09-23) — Phase 0 closed, CP1 answered

**Commits:** f8511271c (program plan, writing brief, this file) · 10f15eb5f (P0-1 row, first amendments) · d2ce2160b (phase handoff protocol) · 660b70833 (move manifest, ticket rewrites, orphan-spec links, pin catalogue, CP1 questions) · f79a84a42 (CP1 answers 1–3 applied by P0-5f, `refactor_followup_tickets.md`, plan evidence-folder wording).

**Decisions:** CP1 answered — the operator accepted all 21 recommendations (`refactor_move_manifest/cp1_questions.md`; rules in the writing brief's "Checkpoint 1 decisions"). Every amendment above applies from Phase 1 on.

**Environment at close:**
- Another session holds ~161 uncommitted entries (equipment data viewer, equipment export, equipment gameplay) across api_v2, frontend, tools_v2, apps/mod and contracts_v2. Shared dirty files: `CLAUDE.md`, `.gitignore`, `apps/website/api_v2/.env.example`, `apps/mod/tbd-export/resourceDatabase.rdb` (never stage the last one). Stage any shared file hunk-wise.
- `hcargo build -p xtask` exits 0 (log `<scratchpad>/logs/orchestrator/xtask_build_phase0_end.log`); it was broken earlier the same day by the other session's in-flight `equipment_gameplay` module, so re-check before each Phase 1 agent.
- Red, not ours: `verify-coding-standards` route-tag check (the other session's `equipment_data_viewer` routes); `tooling_prose_rules::nothing_narrates_its_own_history` (`equipment_vehicle_export/legacy_archive.rs`).
- Red, fixed by Phase 1: `only_a_layout_module_spells_a_repository_path` (P1-4), `every_rust_file_named_in_prose_exists` (P1-1).
- `docs/` has no uncommitted changes; the API v2 completion session writes `docs/verification/api_v2/` — pause it before Phase 2.
- Everything Phase 1+ needs is committed under `documentation_v2/refactor_*`; the scratchpad intermediates are not needed.

**Next — Phase 1, sequential, one green commit per agent:** P1-1 → P1-2 → P1-2b → P1-3 → P1-4 → P1-5 → P1-6. Prompts are in the plan's Phase 1 section, adjusted by the amendments:
- Before each agent: `hcargo build -p xtask` exits 0; if the other session breaks it again, stop and ask the operator.
- P1-1 also owns `tools_v2/README.md` (lines 13-18).
- P1-2 also extends spec/plan existence validation to every ticket file (parents and children) and first repoints T-068.10.5 → `.ai/artifacts/t068_10_5_weapon_families.md` and T-159.15.0 → `.ai/artifacts/t159_15_render_loop_handoff.md`.
- P1-2b (new): widen `.ai/tickets/schema.json:18` and `tools_v2/ticket-engine/src/sync/gap_analysis.rs:12` to `[0-9]{3,}` with tests; then file the three tickets in `documentation_v2/refactor_followup_tickets.md` top-level (rename program → deferred; mod wave gate Makefile → idea; human cleanup of 44 branches, 6 worktrees and the 4 merged-but-ready tickets → idea).
- P1-4: `API_READINESS_EVIDENCE_PREFIX` keeps the value `docs/verification/api_v2/` until P2-2 sets `documentation_v2/website/api_v2/verification_evidence/`.
- P1-5: treat `documentation_v2/refactor_*` as records; evidence folders are named `verification_evidence/`.
- Budget: stop agents near 450k tokens (P0-5 overran at ≈645k); keep scopes narrow.

## Checkpoints

| Checkpoint | Follows | The operator reviews | Status |
|---|---|---|---|
| CP1 manifest | P0-5 | `refactor_move_manifest.md` and its questions; P0-5 applies the answers in a fix-agent run | answered 2026-09-23 — all 21 recommendations accepted; P0-5f applies 1–3 |
| CP2 cutover | P2-7 | commits 2a and 2b; ticketboard's spec reader; the server address, identified by its SSH host key | pending |
| CP3a standard | P3-1 | the README standard and its samples; changes go through a P3-1 fix run | pending |
| CP3b pilot slices | P3-3 | the R01 and F01 pilot output; the style locks | pending |
| CP4 roadmap | V5.5 | `product_roadmap.md`; the operator strikes unplanned items and F19b applies the strikes | pending |
