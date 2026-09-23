**Status:** live — Documentation V2 pin catalogue

# Documentation V2 pin catalogue

Every place outside the documentation trees that spells a documentation path, a document name or
a document section that the move changes, with the new spelling and the agent that changes it.
The moves themselves are in [refactor_move_manifest.tsv](/documentation_v2/refactor_move_manifest.tsv);
ticket fields are in [refactor_ticket_rewrites.tsv](/documentation_v2/refactor_ticket_rewrites.tsv).

## How to read it

- **Sources.** The pin audit of the program's explorer pass, re-verified line by line with
  `git grep -n`; the section and bare-name pins P0-3 recorded; and a scripted
  `git grep -n -F` of every manifest source path, every retired `docs/` token, every basename the
  manifest changes and every extension-less document name (`TBD_MOD_DESIGN §2`) across
  `tools_v2/`, `apps/`, `.github/`, `.editorconfig-checker.json`, `.editorconfig`, `.gitignore`,
  `CLAUDE.md`, `README.md`, `.cursor/`, `contracts_v2/`, `assets_v2/` and `.ai/tickets/` (ticket
  TOML files excluded: the rewrite TSV covers them).
- **Rows.** 342: P1-1 6, P1-2 42, P1-3 16, P1-4 2, P2-2 156, P2-3 2, P2-6 44, G1 2, G2 30, H1 21,
  none 21. Kinds: code 84, comment 98, config 9, prose 94, test 57.
- **Owners.** P1-1 (tools_v2 program records), P1-2 (generated views), P1-3 (prose-gate
  constants), P1-4 (api-readiness constants), P2-2 (code, tests, config and comments, including
  the xtask message texts and the EnfScript comments), P2-3 (files its ticket commands
  regenerate), P2-6 (markdown prose in live files), G1 (`CLAUDE.md`), G2 (`.cursor/`,
  `apps/mod/.cursor/` and the `.ai/tickets` markdown), H1 (CI wiring and the doc-layout
  plumbing), none (must not change, or synthetic test data).
- **G1 and G2 rows** sit in files P2-6 also owns in commit 2b: P2-6 rewrites the link to the new
  string then, and G1 or G2 refresh the prose around it in Phase 6.
- **New string.** Markdown files take the repo-root link form (`/documentation_v2/…`); code,
  config and comments take the plain repo path. A value in parentheses is a structural change,
  not a string swap. Section signs stay as written except in EnfScript, where every added line
  must be ASCII (`section 2`, `section "Sources"`).
- **Not rows.** 105 hits sit inside 9 files the manifest itself moves (the tools_v2 and
  api_v2 program records, `apps/website/frontend/src/v2/MIGRATION.md` and
  `contracts_v2/MIGRATION_HANDOFF.md`): P2-5 rewrites the links in frozen and archived files and
  P2-6 those in live ones, driven by the manifest. `.ai/artifacts/` is out of scope and never
  rewritten.

## Must-not-change spellings

- `ARCHIVED_WAVE_PLANS` (`tools_v2/ticket-engine/src/repository.rs:233-234`): the two archived
  wave-plan paths are read at historical revisions through `git show`. No document of this
  program spells them, because the fossil-path guard reds any tracked file outside its allowlist
  that does; the same holds for the two wave environment variables the guard bans.
- The retired queue-view prefix `docs/TICKET_` (`repository.rs:200`, renamed
  `RETIRED_QUEUE_VIEW_PREFIX` by P1-2) and `NUMSTAT_EXCLUDED_PREFIXES` (`:225`): historical
  numstat still spells it.
- The token-factor document's quoted prefixes `.ai/`, `docs/TICKET_` and `Cargo.lock`:
  `estimate_provenance_tests.rs:5-22` requires the moved
  `documentation_v2/tools_v2/ticket-engine/token_estimate_factor.md` to quote them verbatim.
- Everything inside frozen specs, plans and archived files except their links.
- The ticket strings that name the archived wave plans (14, in five tickets): they stay as
  written and are absent from the rewrite TSV.

## Corrections to the plan's pin list

- The CRF-leak check is
  `tools_v2/xtask/src/verifications/licensing/upstream_code_leaks/verify_crf_leak.rs:39-41`. Its
  `§2` pin belongs to `mod_design.md` (F10); its `§Oracle lanes` pin belongs to the SLICE_WORKFLOW
  target `documentation_v2/runbooks/mod_slice_workflow.md` (F15), not to F10 as the plan's F10
  row says.
- `enf citations` reads `@idx` lines from files that land in several documentation_v2 folders
  (`mod/tbd-framework/mod_design.md`, `runbooks/mod_slice_workflow.md`), so `MOD_DOCS_DIR`
  becomes a walk over `documentation_v2/` (P2-2), not `documentation_v2/mod/`.
- `documentation_v2/standards/engine_boundary_rules.md` is written by F09 in Phase 5. The
  engine-layer messages P2-2 rewrites in commit 2a may point at it early; live markdown links
  point at the archived program until F09 re-points them.
- `tools_v2/README.md:13-18` links the tools_v2 program records P1-1 archives, but P1-1's YOU OWN
  names only `tools_v2/ticket-engine/README.md:23`; those rows are marked P1-1.
- `.editorconfig-checker.json` needs a `documentation_v2/design_system/token_exports/` entry as
  well: `aegis_design_tokens.md` leaves an excluded folder and ends without a final newline.

## Catalogue

| file:line | current string | new string | owner | kind | note |
|---|---|---|---|---|---|
| `tools_v2/README.md:13` | `ARCHITECTURE_PLAN.md` | `/documentation_v2/archive/tools_v2_refactor/architecture_plan.md` | P1-1 | prose | not in P1-1's YOU OWN (the plan names only ticket-engine/README.md:23); the links break when P1-1 moves the records, so P1-1 needs this file too |
| `tools_v2/README.md:14` | `ANALYSIS_AND_INVENTORY.md` | `/documentation_v2/archive/tools_v2_refactor/analysis_and_inventory.md` | P1-1 | prose | as line 13 |
| `tools_v2/README.md:16` | `PHASE_ONE_HANDOFF.md · PHASE_TWO_HANDOFF.md` | `/documentation_v2/archive/tools_v2_refactor/phase_one_handoff.md · /documentation_v2/archive/tools_v2_refactor/phase_two_handoff.md` | P1-1 | prose | as line 13 |
| `tools_v2/README.md:17` | `PHASE_FOUR_HANDOFF.md · PHASE_THREE_HANDOFF.md` | `/documentation_v2/archive/tools_v2_refactor/phase_four_handoff.md · /documentation_v2/archive/tools_v2_refactor/phase_three_handoff.md` | P1-1 | prose | as line 13 |
| `tools_v2/README.md:18` | `PHASE_FIVE_HANDOFF.md` | `/documentation_v2/archive/tools_v2_refactor/phase_five_handoff.md` | P1-1 | prose | as line 13 |
| `tools_v2/ticket-engine/README.md:23` | `../PHASE_THREE_HANDOFF.md` | `/documentation_v2/archive/tools_v2_refactor/phase_three_handoff.md` | P1-1 | prose | in P1-1's YOU OWN |
| `.ai/tickets/AI_PLAYBOOK.md:57` | `docs/TICKET_BRAINSTORM.md` | `one-line pointer to /apps/ticketboard/` | P1-2 | prose | in P1-2's YOU OWN |
| `.ai/tickets/AI_PLAYBOOK.md:74` | `docs/TICKET_MOD_QUEUE.md` | `one-line pointer to /apps/ticketboard/` | P1-2 | prose | in P1-2's YOU OWN |
| `.ai/tickets/AI_PLAYBOOK.md:98` | `docs/TICKET_REGISTRY.md` | (view table rows 98-103 become one pointer to /apps/ticketboard/) | P1-2 | prose | in P1-2's YOU OWN |
| `.ai/tickets/AI_PLAYBOOK.md:99` | `docs/TICKET_LEAD.md` | (as line 98) | P1-2 | prose | in P1-2's YOU OWN |
| `.ai/tickets/AI_PLAYBOOK.md:100` | `docs/TICKET_DEV_QUEUE.md` | (as line 98) | P1-2 | prose | in P1-2's YOU OWN |
| `.ai/tickets/AI_PLAYBOOK.md:101` | `docs/TICKET_MOD_QUEUE.md` | (as line 98) | P1-2 | prose | in P1-2's YOU OWN |
| `.ai/tickets/AI_PLAYBOOK.md:102` | `docs/MILESTONES.md` | (as line 98) | P1-2 | prose | in P1-2's YOU OWN |
| `.ai/tickets/AI_PLAYBOOK.md:103` | `docs/TICKET_BRAINSTORM.md` | (as line 98) | P1-2 | prose | in P1-2's YOU OWN |
| `.ai/tickets/README.md:57` | `docs/TICKET_LEAD.md · docs/platform/t161_ticket_xtask_program.md` | `one-line pointer to /apps/ticketboard/ · /documentation_v2/tickets/specs/t161_ticket_xtask_program.md` | P1-2 | prose | P1-2 replaces the view link (in its YOU OWN); the t161 hub link on the same line is rewritten by P2-6 in 2b and refreshed by G2 |
| `.ai/tickets/SPEC_TEMPLATE.md:6` | `docs/TICKET_LEAD.md` | `one-line pointer to /apps/ticketboard/` | P1-2 | prose | in P1-2's YOU OWN |
| `README.md:34` | `docs/TICKET_LEAD.md` | `one-line pointer to /apps/ticketboard/` | P1-2 | prose | in P1-2's YOU OWN |
| `apps/ticketboard/src/repository_status/models/git_status.rs:6-13` | `GIT_ARGS pathspec TREE_DIR` | `ROADMAP + GAP_ANALYSIS` | P1-2 | code | git status stays scoped to the files sync still writes |
| `apps/ticketboard/src/repository_status/models/tests/git_status.rs:8` | `docs/TICKET_LEAD.md` | (fixture follows the new GIT_ARGS pathspec) | P1-2 | test | GIT_ARGS swaps TREE_DIR for ROADMAP + GAP_ANALYSIS |
| `apps/ticketboard/src/repository_status/models/tests/git_status.rs:10` | `docs/TICKET_NEW.md · docs/old.md` | (fixture follows the new GIT_ARGS pathspec) | P1-2 | test | as line 8 |
| `apps/ticketboard/src/repository_status/models/tests/git_status.rs:24` | `docs/TICKET_LEAD.md` | (fixture follows the new GIT_ARGS pathspec) | P1-2 | test | as line 8 |
| `apps/ticketboard/src/repository_status/models/tests/git_status.rs:26` | `docs/TICKET_NEW.md · docs/old.md` | (fixture follows the new GIT_ARGS pathspec) | P1-2 | test | as line 8 |
| `apps/ticketboard/src/repository_status/services/file_watch.rs:13` | `use ...documentation::{ROADMAP, TREE_DIR}` | `use ...documentation::ROADMAP` | P1-2 | code | the view filter goes |
| `apps/ticketboard/src/repository_status/services/file_watch.rs:109-113` | `ticket_doc_name (TICKET_*.md, MILESTONES.md)` | (deleted) | P1-2 | code | view filter |
| `apps/ticketboard/src/repository_status/services/file_watch.rs:137-139` | `TREE_DIR parent rule` | (deleted) | P1-2 | code | view filter |
| `apps/ticketboard/src/repository_status/services/file_watch.rs:177-182` | `watch list: TICKETS_DIR, TREE_DIR, ROADMAP` | `TICKETS_DIR, ROADMAP` | P1-2 | code | 3 entries to 2; :117 and :140 read ROADMAP and follow the constant |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:100` | `TICKET_LEAD.md` | (assertion removed with the view filter) | P1-2 | test | ticket_doc_name goes with the view filter |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:101` | `TICKET_REGISTRY.md` | (assertion removed with the view filter) | P1-2 | test | as line 100 |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:122` | `docs/` | (case removed: the tree root is no longer watched) | P1-2 | test | the watch list drops TREE_DIR (3 entries to 2) |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:123` | `docs/TICKET_LEAD.md` | (case removed) | P1-2 | test | as line 122 |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:124` | `docs/MILESTONES.md` | (case removed) | P1-2 | test | as line 122 |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:125` | `docs/README.md` | (case removed) | P1-2 | test | as line 122 |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:126` | `docs/` | (case removed) | P1-2 | test | as line 122 |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:129` | `docs/platform/TICKET_X.md` | (case removed) | P1-2 | test | as line 122 |
| `apps/website/README.md:37` | `docs/TICKET_LEAD.md · docs/TICKET_REGISTRY.md` | `one-line pointer to /apps/ticketboard/` | P1-2 | prose | in P1-2's YOU OWN |
| `tools_v2/ticket-engine/src/cli/tests/command_mutation_tests.rs:408-416` | `read of TICKET_REGISTRY_VIEW` | (assertion deleted) | P1-2 | test | in P1-2's YOU OWN |
| `tools_v2/ticket-engine/src/repository.rs:168` | `docs/TICKET_REGISTRY.md` | (constant deleted) | P1-2 | code | TICKET_REGISTRY_VIEW |
| `tools_v2/ticket-engine/src/repository.rs:171` | `docs/TICKET_LEAD.md` | (constant deleted) | P1-2 | code | TICKET_LEAD_VIEW |
| `tools_v2/ticket-engine/src/repository.rs:174` | `docs/TICKET_DEV_QUEUE.md` | (constant deleted) | P1-2 | code | TICKET_DEV_QUEUE_VIEW |
| `tools_v2/ticket-engine/src/repository.rs:177` | `docs/TICKET_BRAINSTORM.md` | (constant deleted) | P1-2 | code | TICKET_BRAINSTORM_VIEW |
| `tools_v2/ticket-engine/src/repository.rs:180` | `docs/TICKET_MOD_QUEUE.md` | (constant deleted) | P1-2 | code | TICKET_MOD_QUEUE_VIEW |
| `tools_v2/ticket-engine/src/repository.rs:183` | `docs/MILESTONES.md` | (constant deleted) | P1-2 | code | MILESTONES |
| `tools_v2/ticket-engine/src/repository.rs:186` | `docs/mod/MILESTONES.md` | (constant deleted) | P1-2 | code | MOD_MILESTONES; the file itself is archived at documentation_v2/archive/product_plans/mod_milestones.md by the manifest |
| `tools_v2/ticket-engine/src/repository.rs:190-197` | `GENERATED_QUEUE_VIEWS` | (constant deleted) | P1-2 | code | the list of the six views |
| `tools_v2/ticket-engine/src/repository.rs:200` | `docs/TICKET_` | `RETIRED_QUEUE_VIEW_PREFIX = "docs/TICKET_" (value unchanged)` | P1-2 | code | MUST NOT CHANGE the value: it matches historical numstat paths; only the name and a present-tense doc comment change |
| `tools_v2/ticket-engine/src/repository.rs:248-251` | `ARCHIVED_WAVE_PLAN_READERS row keyed on GENERATED_QUEUE_VIEW_PREFIX` | (row deleted) | P1-2 | code | the views that quoted ticket prose are gone |
| `tools_v2/ticket-engine/src/sync/queue_views.rs:91` | `mod/MILESTONES.md link text over MOD_MILESTONES` | (file deleted) | P1-2 | code | the whole view generator goes; so does sync/registry_views.rs |
| `tools_v2/ticket-engine/src/sync/runner.rs:17-42` | `create_dir_all(TREE_DIR) and the six view writes` | (deleted) | P1-2 | code | keep queue.json (:44-45), the ROADMAP marker (:47-53) and the gap-analysis column (:55-57); both keepers skip silently when their file is missing (is_file checks), so P2-2 must move the ROADMAP and GAP_ANALYSIS constants in the same commit as P2-1's moves |
| `.ai/tickets/corpus-pins.toml:(key)` | `map_terrain_programme_ticket` | (key removed) | P1-3 | config | with the corpus_pins.rs field |
| `.github/workflows/ci.yml:164-169` | `specification-consistency n6 n10 in the schema sub-gate list` | (the two retired gates leave the list) | P1-3 | comment | Class-R parity comment with ci-local-schema |
| `tools_v2/ticket-engine/src/corpus_pins.rs:26-27` | `map_terrain_programme_ticket` | (field retired with its gate) | P1-3 | code | with src/tests/corpus_pins_tests.rs:16,40,67 and the key in .ai/tickets/corpus-pins.toml (the plan's P1-3 list) |
| `tools_v2/xtask/src/commands/db/operations.rs:136` | `schema specification-consistency` in the LANE_COMMANDS doc comment | (reason reworded: no retired gate reads the list) | P1-3 | comment | the plan's :135-139 comment |
| `tools_v2/xtask/src/commands/platform/wave_execution/schema.rs:26` | `docs/specs/**` | (docs/specs/** leaves the list with the retired gates) | P1-3 | comment | the plan's :22-25 doc block |
| `tools_v2/xtask/src/core/repository_layout.rs:63` | `docs/specs/Mission_Creator_Architecture` | (constant deleted) | P1-3 | code | SPECIFICATION_DOCS_DIR; the plan's :62-75 block |
| `tools_v2/xtask/src/core/repository_layout.rs:66` | `docs/website/frontend/ROADMAP.md` | (constant deleted) | P1-3 | code | FRONTEND_ROADMAP |
| `tools_v2/xtask/src/core/repository_layout.rs:69` | `docs/website/frontend/INDEX.md` | (constant deleted) | P1-3 | code | FRONTEND_INDEX |
| `tools_v2/xtask/src/core/repository_layout.rs:72` | `docs/website/frontend/pages/mission-editor.md` | (constant deleted) | P1-3 | code | MISSION_EDITOR_SURFACE |
| `tools_v2/xtask/src/core/repository_layout.rs:75` | `docs/mod/CLAUDE-CODE-START.md` | (constant deleted) | P1-3 | code | MOD_AGENT_START |
| `tools_v2/xtask/src/verifications/schemas/checks.rs:122, 290-292, 300-301, 321` | `ARCHIVAL_MAKE_TARGETS, content_budgets and specification_consistency wiring` | (deleted) | P1-3 | code | the plan's P1-3 line list; content_budgets.rs is deleted too |
| `tools_v2/xtask/src/verifications/schemas/checks/read_json.rs:12-14` | `spec_dir() over SPECIFICATION_DOCS_DIR` | (deleted) | P1-3 | code | orphan once the gates go |
| `tools_v2/xtask/src/verifications/schemas/checks/specification_consistency.rs:144` | `engineering_plan.md` | (file deleted) | P1-3 | code | lines 144, 154, 290 and 291 name engineering_plan.md and agent_execution.md; the whole gate is retired |
| `tools_v2/xtask/src/verifications/schemas/checks/specification_consistency.rs:154` | `engineering_plan.md` | (file deleted) | P1-3 | code | as line 144 |
| `tools_v2/xtask/src/verifications/schemas/checks/specification_consistency.rs:290` | `agent_execution.md` | (file deleted) | P1-3 | code | as line 144 |
| `tools_v2/xtask/src/verifications/schemas/checks/specification_consistency.rs:291` | `engineering_plan.md` | (file deleted) | P1-3 | code | as line 144 |
| `tools_v2/xtask/src/verifications/api_readiness/fingerprint.rs:17` | `docs/verification/api_v2/` | `API_READINESS_EVIDENCE_PREFIX (xtask documentation module; value unchanged, trailing slash kept for the starts_with at :97)` | P1-4 | code | P2-2 then sets the constant's value (row under P2-2) |
| `tools_v2/xtask/src/verifications/api_readiness/register.rs:10` | `docs/verification/api_v2/requirements.json` | `API_READINESS_REGISTER (xtask documentation module; value unchanged)` | P1-4 | code | P1-4 adds a test that the register starts with the prefix; P2-2 then sets the value |
| `.ai/tickets/corpus-pins.toml:24` | `docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md` | `documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md` | P2-2 | comment | - |
| `.ai/tickets/estimates.schema.json:4` | `docs/platform/token_estimate_factor.md` | `documentation_v2/tools_v2/ticket-engine/token_estimate_factor.md` | P2-2 | config | - |
| `.ai/tickets/scope-vocab.toml:2` | `docs/platform/t917_ticket_schema_v2.md §Scope v2` | `documentation_v2/tickets/specs/t917_ticket_schema_v2.md §Scope v2` | P2-2 | comment | - |
| `.editorconfig:1` | `CODING_STANDARDS.md §7` | `documentation_v2/standards/coding_standards/README.md §7` | P2-2 | comment | section and rule ids are content pins: F17 keeps them in the new document |
| `.editorconfig-checker.json:13` | `docs/specs/macOS_Blueprints/` | `"documentation_v2/.*/visual_references/" and "documentation_v2/design_system/token_exports/"` | P2-2 | config | Exclude entries are regular expressions; the token export aegis_design_tokens.md (moved from an excluded folder) ends without a final newline, which [*] insert_final_newline fails, so its folder needs the entry too until F17 rewrites it |
| `.editorconfig-checker.json:14` | `docs/specs/Mission_Creator_Mock_Up/` | (covered by the line-13 entries) | P2-2 | config | as line 13 |
| `.github/workflows/ci.yml:3` | `CODING_STANDARDS.md §0.3` | `documentation_v2/standards/coding_standards/README.md §0.3` | P2-2 | comment | section and rule ids are content pins: F17 keeps them in the new document |
| `.github/workflows/ci.yml:251` | `ENGINE_SPLIT_PROGRAM §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | comment | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `.github/workflows/ci.yml:259` | `ENGINE_SPLIT_PROGRAM §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | config | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `.github/workflows/contracts.yml:3` | `DOCUMENTATION_STANDARDS.md §10` | `documentation_v2/standards/documentation_standards.md §10` | P2-2 | comment | section and rule ids are content pins: P3-2 keeps them in the new document |
| `.gitignore:3` | `docs/mod/SLICE_WORKFLOW.md` | `documentation_v2/runbooks/mod_slice_workflow.md` | P2-2 | comment | - |
| `.gitignore:24` | `docs/mod/CLAUDE-CODE-START.md` | `documentation_v2/runbooks/mod_slice_workflow.md` | P2-2 | comment | - |
| `apps/fleet_host_agent/src/ledger_client/mod.rs:3` | `docs/verification/api_v2/fleet_command_ledger.md` | `documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md` | P2-2 | comment | - |
| `apps/mod/.gitignore:4` | `CLAUDE-CODE-START.md` | `documentation_v2/runbooks/mod_slice_workflow.md` | P2-2 | comment | - |
| `apps/mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryScan.c:1464` | `.ai/artifacts/t068_10_5_weapon_families.md` | `documentation_v2/tickets/specs/t068_10_5_weapon_families.md` | P2-2 | comment | rewritten line ASCII-only |
| `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_ResultsReporter.c:4` | `TBD_MOD_DESIGN.md §6` | `documentation_v2/mod/tbd-framework/mod_design.md section 6` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Log.c:106` | `STAGING-SERVER.md` | `documentation_v2/runbooks/game_server_staging/README.md` | P2-2 | comment | rewritten line ASCII-only, and replace the line's other non-ASCII characters; F14 keeps the quoted [TBD] Stage Print text |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/TBD_FrameworkManager.c:970` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/TBD_FrameworkManager.c:1196` | `STAGING-SERVER.md` | `documentation_v2/runbooks/game_server_staging/README.md` | P2-2 | comment | rewritten line ASCII-only; F14 keeps the quoted [TBD] Stage Print text |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/TBD_FrameworkManager.c:1586` | `TBD_MOD_DESIGN.md §5` | `documentation_v2/mod/tbd-framework/mod_design.md section 5` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/TBD_SafestartManager.c:5` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/TBD_SafestartManager.c:1032` | `TBD_MOD_DESIGN.md §5` | `documentation_v2/mod/tbd-framework/mod_design.md section 5` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section', and replace the line's other non-ASCII characters; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/TBD_AdminAudit.c:123` | `TBD_MOD_DESIGN.md §5` | `documentation_v2/mod/tbd-framework/mod_design.md section 5` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/TBD_AdminAudit.c:249` | `SLICE_WORKFLOW.md §Sources` | `documentation_v2/runbooks/mod_slice_workflow.md section "Sources"` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F15 keeps rule 1, §Sources, §What agents cannot do and §Oracle lanes |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/TBD_AdminService.c:74` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/TBD_AdminService.c:407` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/TBD_AdminSnapshotService.c:30` | `SLICE_WORKFLOW.md §What agents cannot do` | `documentation_v2/runbooks/mod_slice_workflow.md section "What agents cannot do"` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F15 keeps rule 1, §Sources, §What agents cannot do and §Oracle lanes |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/TBD_AdminScreen.c:23` | `TBD_MOD_DESIGN.md §2, §6` | `documentation_v2/mod/tbd-framework/mod_design.md section 2, section 6` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section', and replace the line's other non-ASCII characters; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/TBD_BriefingController.c:117` | `TBD_MOD_DESIGN.md §5` | `documentation_v2/mod/tbd-framework/mod_design.md section 5` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/TBD_BriefingService.c:12` | `SLICE_WORKFLOW.md §What agents cannot do` | `documentation_v2/runbooks/mod_slice_workflow.md section "What agents cannot do"` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F15 keeps rule 1, §Sources, §What agents cannot do and §Oracle lanes |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/TBD_MissionBrowser.c:25` | `docs/mod/t181_event_mod_program.md` | `documentation_v2/tickets/specs/t181_event_mod_program.md` | P2-2 | comment | rewritten line ASCII-only |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/TBD_MissionSelectorData.c:4` | `UI_STRUCTURE.md` | `documentation_v2/mod/tbd-framework/UI/README.md` | P2-2 | comment | rewritten line ASCII-only; F10 keeps the module-role table |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Players/TBD_PlayersCatalog.c:3` | `UI_STRUCTURE.md` | `documentation_v2/mod/tbd-framework/UI/README.md` | P2-2 | comment | rewritten line ASCII-only; F10 keeps the module-role table |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/TBD_SpectatorComponent.c:17` | `docs/mod/t181_event_mod_program.md` | `documentation_v2/tickets/specs/t181_event_mod_program.md` | P2-2 | comment | rewritten line ASCII-only |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/TBD_LoadoutEquipHelper.c:1642` | `docs/platform/PLAYTEST_RUNBOOK.md` | `documentation_v2/runbooks/two_client_playtest/README.md` | P2-2 | comment | rewritten line ASCII-only; F15 keeps the quoted grep strings |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/TBD_MarkerData.c:309` | `docs/mod/t181_event_mod_program.md` | `documentation_v2/tickets/specs/t181_event_mod_program.md` | P2-2 | comment | rewritten line ASCII-only |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/TBD_MissionLoader.c:250` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/TBD_RadioBridgeStub.c:8` | `contracts_v2/definitions/bridge-messages.md` | `documentation_v2/contracts_v2/definitions/bridge_messages.md` | P2-2 | comment | rewritten line ASCII-only |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/TBD_RadioBridgeStub.c:9` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Radio/TBD_RadioTuner.c:4` | `TBD_MOD_DESIGN.md §6` | `documentation_v2/mod/tbd-framework/mod_design.md section 6` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/TBD_SpawnManager.c:60` | `TBD_MOD_DESIGN.md (its §2 opens line 61)` | `documentation_v2/mod/tbd-framework/mod_design.md (line 61 keeps its section reference, spelled section 2 if rewritten)` | P2-2 | comment | rewritten line ASCII-only; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/TBD_SpawnManager.c:140` | `docs/mod/TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section', and replace the line's other non-ASCII characters; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/TBD_SpawnManager.c:1764` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/TBD_SpawnManager.c:1781` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/TBD_PlayAreaComponent.c:29` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/TBD_Zone.c:23` | `TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_MenuStack.c:271` | `docs/mod/TBD_MOD_DESIGN.md §5` | `documentation_v2/mod/tbd-framework/mod_design.md section 5` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section', and replace the line's other non-ASCII characters; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UIInteractive.c:8` | `docs/mod/TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section', and replace the line's other non-ASCII characters; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c:9` | `docs/mod/ui/UI_STRUCTURE.md` | `documentation_v2/mod/tbd-framework/UI/README.md` | P2-2 | comment | rewritten line ASCII-only; F10 keeps the module-role table |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UITheme.c:31` | `docs/mod/TBD_MOD_DESIGN.md §2` | `documentation_v2/mod/tbd-framework/mod_design.md section 2` | P2-2 | comment | rewritten line ASCII-only: spell the section sign as 'section'; F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:134` | `docs/specs/Mission_Creator_Architecture/ROADMAP.md` | `/repo/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md` | P2-2 | test | the ROADMAP watch stays; the path follows the ROADMAP constant |
| `apps/ticketboard/src/repository_status/services/tests/file_watch.rs:138` | `docs/specs/Mission_Creator_Architecture/other.md` | `/repo/documentation_v2/website/frontend/apps/editor/other.md` | P2-2 | test | negative case beside the ROADMAP file; keep it a sibling of the new ROADMAP path |
| `apps/website/api_v2/.env.example:26` | `DEV_RUNBOOK.md §Discord OAuth2` | `documentation_v2/runbooks/local_development.md §Discord OAuth2` | P2-2 | comment | F13 keeps §Discord OAuth2 and §5 |
| `apps/website/api_v2/.env.example:85` | `docs/website/DEV_RUNBOOK.md §Discord OAuth2` | `documentation_v2/runbooks/local_development.md §Discord OAuth2` | P2-2 | comment | F13 keeps §Discord OAuth2 and §5 |
| `apps/website/api_v2/seeds/discord_roles.sql:20` | `docs/website/DEV_RUNBOOK.md §5` | `documentation_v2/runbooks/local_development.md §5` | P2-2 | comment | F13 keeps §Discord OAuth2 and §5 |
| `apps/website/api_v2/src/community_content/handlers/modpack_admin.rs:35` | `STAGING-SERVER.md` | `documentation_v2/runbooks/game_server_staging/README.md` | P2-2 | comment | F14 keeps the quoted [TBD] Stage Print text |
| `apps/website/api_v2/src/missions/services/mission_artifacts/mod.rs:2` | `docs/verification/api_v2/mission_artifacts.md` | `documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md` | P2-2 | comment | - |
| `apps/website/api_v2/src/server_infrastructure/services/fleet_commands/mod.rs:2` | `docs/verification/api_v2/fleet_command_ledger.md` | `documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md` | P2-2 | comment | - |
| `apps/website/docker-compose.staging.yml:4` | `HOME_SERVER` | `documentation_v2/runbooks/website_deployment.md` | P2-2 | comment | - |
| `apps/website/docker-compose.staging.yml:41` | `HOME_SERVER Phases D–E` | `documentation_v2/runbooks/website_deployment.md Phases D–E` | P2-2 | comment | F13 keeps the Phases D-E wording or the pin is reworded |
| `apps/website/frontend/src/v2/apps/editor/arsenal/tests/shell_wiring.rs:421-430, 588-600` | `gap_analysis in comments and assertion messages` | `eden_gap_analysis (optional wording)` | P2-2 | test | the include_str! row carries the real pin |
| `apps/website/frontend/src/v2/apps/editor/arsenal/tests/shell_wiring.rs:542-546` | `"/../../../docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md" (include_str! over CARGO_MANIFEST_DIR)` | `"/../../../documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md"` | P2-2 | test | resolved from CARGO_MANIFEST_DIR (apps/website/frontend) at compile time; a stale path fails the wasm build (mk ci-local-leptos) |
| `apps/website/frontend/src/v2/apps/editor/ui/inspector/env.rs:30` | `engineering_plan.md` | `documentation_v2/archive/go_and_react_era_design/mission_creator_engineering_plan.md` | P2-2 | comment | - |
| `apps/website/frontend/style/aegis.css:430` | `THEME.md` | `documentation_v2/design_system/design_tokens.md` | P2-2 | comment | - |
| `apps/website/map-engine/src/data/scenario/ast/entities.rs:272` | `contracts_v2/definitions/bridge-messages.md` | `documentation_v2/contracts_v2/definitions/bridge_messages.md` | P2-2 | comment | - |
| `apps/website/map-engine/src/frame/tests/damage_discipline.rs:12` | `ENGINE_SPLIT_PROGRAM.md §2C` | `documentation_v2/standards/engine_boundary_rules.md §2C` | P2-2 | test | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/developer-tools/gate-env.json:6` | `docs/website/EDITOR_GATE_RUNBOOK.md` | `documentation_v2/runbooks/editor_gates.md` | P2-2 | config | - |
| `tools_v2/developer-tools/src/browser_testing/capture_cli.rs:5` | `docs/tools/editor_capture.md` | `documentation_v2/runbooks/editor_capture.md` | P2-2 | comment | - |
| `tools_v2/developer-tools/src/browser_testing/cdp.rs:106` | `docs/tools/editor_capture.md §2` | `documentation_v2/runbooks/editor_capture.md §2` | P2-2 | comment | F14 keeps §2 |
| `tools_v2/developer-tools/src/browser_testing/diagnostics.rs:15` | `docs/website/EDITOR_GATE_RUNBOOK.md` | `documentation_v2/runbooks/editor_gates.md` | P2-2 | comment | - |
| `tools_v2/developer-tools/src/browser_testing/screen_capture.rs:15` | `docs/tools/editor_capture.md` | `documentation_v2/runbooks/editor_capture.md` | P2-2 | comment | - |
| `tools_v2/developer-tools/src/enfusion_tooling/capability.rs:14` | `docs/mod/capability_verdicts.tsv` | `documentation_v2/mod/tbd-framework/capability_verdicts.tsv` | P2-2 | comment | - |
| `tools_v2/developer-tools/src/enfusion_tooling/capability.rs:49` | `docs/mod/TBD_MOD_DESIGN.md §Deferrals` | `documentation_v2/mod/tbd-framework/mod_design.md §Deferrals` | P2-2 | code | F10 keeps §2, §5, §6 and §Deferrals (or lists the renames for P2-2) |
| `tools_v2/developer-tools/src/enfusion_tooling/citations.rs:3` | `docs/mod/**` | `documentation_v2/** (the enf citations walk root)` | P2-2 | comment | - |
| `tools_v2/developer-tools/src/enfusion_tooling/symbols.rs:11` | `docs/mod/oracle/**` | `documentation_v2/** (no oracle folder is tracked; the citations walk covers the tree)` | P2-2 | comment | - |
| `tools_v2/developer-tools/src/map_raster_pipeline/satellite_archive_container.rs:7` | `CODING_STANDARDS` | `documentation_v2/standards/coding_standards/README.md` | P2-2 | comment | section and rule ids are content pins: F17 keeps them in the new document |
| `tools_v2/developer-tools/src/repository_layout.rs:278` | `docs/mod` | `MOD_DOCS_DIR = "documentation_v2" (a walk over the whole tree)` | P2-2 | code | MOD_DOCS_DIR: the @idx citations land in mod/tbd-framework/mod_design.md and runbooks/mod_slice_workflow.md (the frozen t181 spec carries no lane#Symbol citation), so the enf citations walk covers documentation_v2/; a rename of the constant is P2-2's call; readers: enfusion_tooling/cli.rs:58 (enf citations --docs default) |
| `tools_v2/developer-tools/src/repository_layout.rs:282` | `docs/mod/capability_verdicts.tsv` | `documentation_v2/mod/tbd-framework/capability_verdicts.tsv` | P2-2 | code | CAPABILITY_VERDICTS; readers: enfusion_tooling/cli.rs:98 (enf capability --verdicts default) |
| `tools_v2/developer-tools/src/repository_layout.rs:286` | `docs/website/EDITOR_GATE_RUNBOOK.md` | `documentation_v2/runbooks/editor_gates.md` | P2-2 | code | EDITOR_GATE_RUNBOOK; readers: browser_testing/diagnostics/check_fonts.rs:298 |
| `tools_v2/developer-tools/src/world_export_pipeline/forest_smoothing.rs:54` | `docs/specs/ideas/t149_forest_smooth.md` | `documentation_v2/tickets/specs/t149_forest_smooth.md` | P2-2 | comment | - |
| `tools_v2/developer-tools/src/world_export_pipeline/tests/forest_smoothing/tests.rs:98` | `docs/specs/ideas/t149_forest_smooth.md` | `documentation_v2/tickets/specs/t149_forest_smooth.md` | P2-2 | test | - |
| `tools_v2/ticket-engine/src/cli/readiness.rs:23` | `docs/plans/` | `documentation_v2/tickets/plans/` | P2-2 | comment | - |
| `tools_v2/ticket-engine/src/ops/readiness.rs:31` | `docs/plans/TEMPLATE.md` | `.ai/tickets/plan_template.md` | P2-2 | comment | - |
| `tools_v2/ticket-engine/src/ops/tests/status_and_shipping_tests.rs:313-353` | `docs/spec.md · docs/plans/t-1_plan.md · docs/plans/t-2_plan.md` | `documentation_v2/tickets/plans/t-1_plan.md · t-2_plan.md (PLANS_DIR value); spec under a created folder` | P2-2 | test | mark_ready_backfills_and_gates; breaks when PLANS_DIR moves: create_dir_all(PLANS_DIR) stops creating docs/, so the literal fs::write paths under docs/ fail; move the plan literals under the new PLANS_DIR value and put the synthetic spec in a folder the test creates |
| `tools_v2/ticket-engine/src/ops/tests/status_and_shipping_tests.rs:375-414` | `docs/spec.md · docs/plans/t-3_plan.md` | `documentation_v2/tickets/plans/t-3_plan.md; spec under a created folder` | P2-2 | test | mark_ready_refuses_empty_ready_tier_fields; breaks when PLANS_DIR moves: create_dir_all(PLANS_DIR) stops creating docs/, so the literal fs::write paths under docs/ fail; move the plan literals under the new PLANS_DIR value and put the synthetic spec in a folder the test creates |
| `tools_v2/ticket-engine/src/ops/tests/status_and_shipping_tests.rs:559-602` | `docs/spec.md · docs/plans/t-9_1_plan.md · docs/plans/custom.md` | `documentation_v2/tickets/plans/t-9_1_plan.md (the default-path message at :570) · documentation_v2/tickets/plans/custom.md` | P2-2 | test | mark_ready_plan_gate_refuses_and_resolves; breaks when PLANS_DIR moves: create_dir_all(PLANS_DIR) stops creating docs/, so the literal fs::write paths under docs/ fail; move the plan literals under the new PLANS_DIR value and put the synthetic spec in a folder the test creates |
| `tools_v2/ticket-engine/src/ops/tests/status_and_shipping_tests.rs:605-606` | `docs/plans/t-917_6_plan.md · docs/plans/t-090_4_plan.md` | `documentation_v2/tickets/plans/t-917_6_plan.md · documentation_v2/tickets/plans/t-090_4_plan.md` | P2-2 | test | default_plan_path asserts; follow plan_path() |
| `tools_v2/ticket-engine/src/ops/tests/status_and_shipping_tests.rs:628-634` | `docs/spec.md · docs/plans/t-1_plan.md` | `documentation_v2/tickets/plans/t-1_plan.md; spec under a created folder` | P2-2 | test | mark_ready_without_order_refuses; breaks when PLANS_DIR moves: create_dir_all(PLANS_DIR) stops creating docs/, so the literal fs::write paths under docs/ fail; move the plan literals under the new PLANS_DIR value and put the synthetic spec in a folder the test creates |
| `tools_v2/ticket-engine/src/repository.rs:111-128` | `SPARSE_CHECKOUT_SETS: website = [apps/website], mod = [apps/mod], root has documentation::TREE_DIR (:120)` | `website adds documentation_v2/website; mod adds documentation_v2/mod; root keeps TREE_DIR (now documentation_v2)` | P2-2 | code | a website or mod slice checks out its own documentation mirror |
| `tools_v2/ticket-engine/src/repository.rs:136` | `TREE_DIR = "docs"` | `TREE_DIR = "documentation_v2"` | P2-2 | code | readers after P1-2: SPARSE root set (:120) and STALE_TICKET_ID_SCAN_ROOTS (:215); the sync create_dir_all and the ticketboard watch go with P1-2 |
| `tools_v2/ticket-engine/src/repository.rs:139` | `docs/plans` | `documentation_v2/tickets/plans` | P2-2 | code | PLANS_DIR; plan_path() derives every ticket plan path from it, and P2-3's plan rewrites land under it |
| `tools_v2/ticket-engine/src/repository.rs:142` | `docs/plans/TEMPLATE.md` | `.ai/tickets/plan_template.md` | P2-2 | code | PLAN_TEMPLATE; the manifest moves the file there (G2 rewrites it) |
| `tools_v2/ticket-engine/src/repository.rs:154` | `docs/specs` | `documentation_v2/tickets/specs/` | P2-2 | code | SPECS_DIR; a stale-id scan root |
| `tools_v2/ticket-engine/src/repository.rs:158` | `docs/specs/Mission_Creator_Architecture/ROADMAP.md` | `documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md` | P2-2 | code | ROADMAP; ticket sync skips the marker block silently when the file is missing (runner.rs:47-53), so the value moves in the same commit as the file; P1-4's existence test catches a wrong value |
| `tools_v2/ticket-engine/src/repository.rs:161` | `docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md` | `documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md` | P2-2 | code | GAP_ANALYSIS; ticket sync column, the round-trip test and the strict priority rule read it; skipped silently when missing (runner.rs:55-57) |
| `tools_v2/ticket-engine/src/repository.rs:165` | `docs/platform/token_estimate_factor.md` | `documentation_v2/tools_v2/ticket-engine/token_estimate_factor.md` | P2-2 | code | TOKEN_ESTIMATE_FACTOR_DOC; estimate_provenance_tests reads the moved doc, which must keep quoting the three prefixes (F12 pin) |
| `tools_v2/ticket-engine/src/repository.rs:205-211` | `SCAN_EXEMPT_PREFIXES` | `add documentation_v2/archive/ and documentation_v2/tickets/; replace the two design-corpus prefixes (rows :208-209) with the visual_references locations` | P2-2 | code | the stale-id scan walks all of documentation_v2 once TREE_DIR moves; frozen trees quote retired ids by design |
| `tools_v2/ticket-engine/src/repository.rs:214-220` | `STALE_TICKET_ID_SCAN_ROOTS [TREE_DIR, SPECS_DIR, QUEUE_JSON, CLAUDE.md, README.md]` | `unchanged names; TREE_DIR and SPECS_DIR follow their new values` | P2-2 | code | SPECS_DIR sits inside TREE_DIR after the move, so it is walked twice unless dropped |
| `tools_v2/ticket-engine/src/repository.rs:253` | `docs/platform/SHIPPED_HISTORY.md` | `documentation_v2/archive/shipped_history/shipped_history.md` | P2-2 | code | ARCHIVED_WAVE_PLAN_READERS entry: the fossil guard matches by path prefix and the moved file still spells the needles, so the entry names the new path in commit 2a or ticket check --strict reds; one prefix per frozen tree (documentation_v2/archive/, documentation_v2/tickets/specs/) is the alternative |
| `tools_v2/ticket-engine/src/repository.rs:257` | `docs/platform/t911_ticket_registry_redesign.md` | `documentation_v2/tickets/specs/t911_ticket_registry_redesign.md` | P2-2 | code | ARCHIVED_WAVE_PLAN_READERS entry: the fossil guard matches by path prefix and the moved file still spells the needles, so the entry names the new path in commit 2a or ticket check --strict reds; one prefix per frozen tree (documentation_v2/archive/, documentation_v2/tickets/specs/) is the alternative |
| `tools_v2/ticket-engine/src/repository.rs:261` | `docs/platform/t912_wave_lockfile.md` | `documentation_v2/tickets/specs/t912_wave_lockfile.md` | P2-2 | code | ARCHIVED_WAVE_PLAN_READERS entry: the fossil guard matches by path prefix and the moved file still spells the needles, so the entry names the new path in commit 2a or ticket check --strict reds; one prefix per frozen tree (documentation_v2/archive/, documentation_v2/tickets/specs/) is the alternative |
| `tools_v2/ticket-engine/src/repository.rs:265` | `docs/platform/GROK_WAVE_130_HANDOFF.md` | `documentation_v2/archive/factory_runs/grok_wave_130_handoff.md` | P2-2 | code | ARCHIVED_WAVE_PLAN_READERS entry: the fossil guard matches by path prefix and the moved file still spells the needles, so the entry names the new path in commit 2a or ticket check --strict reds; one prefix per frozen tree (documentation_v2/archive/, documentation_v2/tickets/specs/) is the alternative |
| `tools_v2/ticket-engine/src/repository.rs:269` | `docs/platform/WAVE209_GROK_KICKOFF.md` | `documentation_v2/archive/factory_runs/wave_209_grok_kickoff.md` | P2-2 | code | ARCHIVED_WAVE_PLAN_READERS entry: the fossil guard matches by path prefix and the moved file still spells the needles, so the entry names the new path in commit 2a or ticket check --strict reds; one prefix per frozen tree (documentation_v2/archive/, documentation_v2/tickets/specs/) is the alternative |
| `tools_v2/ticket-engine/src/store.rs:4` | `docs/platform/t915_ticketboard_design.md §Write path` | `documentation_v2/tickets/specs/t915_ticketboard_design.md §Write path` | P2-2 | comment | - |
| `tools_v2/ticket-engine/src/tests/repository_layout_tests.rs:31` | `docs/plans/t-917_6_plan.md` | `documentation_v2/tickets/plans/t-917_6_plan.md` | P2-2 | test | - |
| `tools_v2/ticket-engine/src/validation/references.rs:151` | `REORG_CHANGELOG.md` | `documentation_v2/archive/monorepo_migration/reorg_changelog.md` | P2-2 | code | - |
| `tools_v2/ticket-engine/src/validation/runner.rs:154` | `gap_analysis` | `eden_gap_analysis (optional wording in the message)` | P2-2 | code | names the document, not a path |
| `tools_v2/ticket-engine/src/validation/tests/schema_and_integrity_tests.rs:627-637` | `docs/plans/t-001_plan.md (TOML value, error text, file write)` | `documentation_v2/tickets/plans/t-001_plan.md` | P2-2 | test | plan-gate case: :636 creates PLANS_DIR, so :637's literal must sit under it |
| `tools_v2/ticket-engine/src/validation/vocabulary.rs:5` | `docs/platform/t917_ticket_schema_v2.md §Scope v2` | `documentation_v2/tickets/specs/t917_ticket_schema_v2.md §Scope v2` | P2-2 | comment | - |
| `tools_v2/xtask/deploy/Caddyfile.website:7` | `docs/website/HOME_SERVER.md` | `documentation_v2/runbooks/website_deployment.md` | P2-2 | comment | - |
| `tools_v2/xtask/deploy/systemd/tbd-website-api.service:35` | `docs/website/HOME_SERVER.md` | `documentation_v2/runbooks/website_deployment.md` | P2-2 | config | - |
| `tools_v2/xtask/src/commands/agent_context/guards.rs:27` | `PLATFORM_FACTORY.md` | `documentation_v2/runbooks/factory_waves/README.md` | P2-2 | comment | - |
| `tools_v2/xtask/src/commands/build/recipes/shell_word.rs:239` | `docs/platform/EDITOR_FACTORY_FOR_CURSOR.md §5` | `documentation_v2/runbooks/factory_waves/README.md §5` | P2-2 | comment | the file merges into factory_waves/README.md (F16), so §5 must survive as a named section there; cite the file now, the heading once F16 names it |
| `tools_v2/xtask/src/commands/ci/task_definitions.rs:44` | `ENGINE_SPLIT_PROGRAM §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | comment | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/commands/ci/task_definitions.rs:109` | `docs/ · DOCUMENTATION_STANDARDS §10` | `documentation_v2/ · documentation_v2/standards/documentation_standards.md §10` | P2-2 | code | section and rule ids are content pins: P3-2 keeps them in the new document |
| `tools_v2/xtask/src/commands/ci/task_definitions.rs:116` | `CODING_STANDARDS §11` | `documentation_v2/standards/coding_standards/README.md §11` | P2-2 | code | section and rule ids are content pins: F17 keeps them; H1 drops 'doc layout' from this help when verify-doc-layout goes |
| `tools_v2/xtask/src/commands/ci/task_definitions.rs:141` | `CODING_STANDARDS §7` | `documentation_v2/standards/coding_standards/README.md §7` | P2-2 | code | section and rule ids are content pins: F17 keeps them in the new document |
| `tools_v2/xtask/src/commands/ci/task_definitions.rs:300` | `ENGINE_SPLIT_PROGRAM §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | code | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:55` | `ENGINE_SPLIT_PROGRAM §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | test | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/commands/deploy/tests/website/tests.rs:108` | `HOME_SERVER` | `documentation_v2/runbooks/website_deployment.md` | P2-2 | test | - |
| `tools_v2/xtask/src/commands/deploy/website/asset_preflight.rs:81` | `HOME_SERVER` | `documentation_v2/runbooks/website_deployment.md` | P2-2 | comment | - |
| `tools_v2/xtask/src/commands/mod_ops/playtest_server.rs:31` | `docs/mod/STAGING-SERVER.md` | `documentation_v2/runbooks/game_server_staging/README.md` | P2-2 | comment | F14 keeps the quoted [TBD] Stage Print text |
| `tools_v2/xtask/src/commands/mod_ops/playtest_server.rs:47` | `docs/mod/STAGING-SERVER.md` | `documentation_v2/runbooks/game_server_staging/README.md` | P2-2 | comment | F14 keeps the quoted [TBD] Stage Print text |
| `tools_v2/xtask/src/commands/mod_ops/playtest_server/usage_fail.rs:192` | `docs/mod/STAGING-SERVER.md` | `documentation_v2/runbooks/game_server_staging/README.md` | P2-2 | comment | F14 keeps the quoted [TBD] Stage Print text |
| `tools_v2/xtask/src/commands/platform/slice_worktree/git_plain.rs:174` | `SLICE_WORKFLOW.md rule 1` | `documentation_v2/runbooks/mod_slice_workflow.md rule 1` | P2-2 | code | F15 keeps rule 1, §Sources, §What agents cannot do and §Oracle lanes |
| `tools_v2/xtask/src/commands/platform/slice_worktree/tests.rs:137` | `PLATFORM_FACTORY.md` | `documentation_v2/runbooks/factory_waves/README.md` | P2-2 | test | the :276 line reference dies with the move and F16's merge; cite the section (Known traps) instead |
| `tools_v2/xtask/src/commands/platform/slice_worktree/tests.rs:138` | `FACTORY_FOR_CURSOR.md` | `documentation_v2/runbooks/factory_waves/README.md` | P2-2 | test | FACTORY_FOR_CURSOR merges into factory_waves/README.md; the :469 line reference dies; cite the section instead |
| `tools_v2/xtask/src/commands/platform/wave_execution/changed/changed_rs.rs:424` | `PLATFORM_FACTORY.md Known traps` | `documentation_v2/runbooks/factory_waves/README.md Known traps` | P2-2 | comment | F16 keeps the Known traps section |
| `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs:8` | `docs/platform/EDITOR_FACTORY_FOR_CURSOR.md §5` | `documentation_v2/runbooks/factory_waves/README.md §5` | P2-2 | comment | the file merges into factory_waves/README.md (F16), so §5 must survive as a named section there; cite the file now, the heading once F16 names it |
| `tools_v2/xtask/src/commands/platform/wave_execution/reclaim/du_mb.rs:217` | `PLATFORM_FACTORY Known traps` | `documentation_v2/runbooks/factory_waves/README.md Known traps` | P2-2 | comment | F16 keeps the Known traps section |
| `tools_v2/xtask/src/commands/setup/staging_server.rs:4` | `docs/mod/STAGING-SERVER.md` | `documentation_v2/runbooks/game_server_staging/README.md` | P2-2 | comment | F14 keeps the quoted [TBD] Stage Print text |
| `tools_v2/xtask/src/commands/verify/cli.rs:78` | `ENGINE_SPLIT_PROGRAM §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | comment | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/core/repository_layout.rs:(new)` | `API_READINESS_EVIDENCE_PREFIX and API_READINESS_REGISTER values` | `documentation_v2/website/api_v2/verification_evidence/ and documentation_v2/website/api_v2/verification_evidence/requirements.json` | P2-2 | code | after P1-4 adds the constants; verify api-readiness must resolve the register |
| `tools_v2/xtask/src/core/repository_layout.rs:60` | `docs/platform/factory_pack_wave` | `.ai/factory_pack_wave` | P2-2 | code | FACTORY_PACK_WAVE (data file, moved by the manifest); readers: platform/wave_execution/db.rs:53 |
| `tools_v2/xtask/src/core/repository_layout.rs:82` | `docs/website/HOME_SERVER.md` | `documentation_v2/runbooks/website_deployment.md` | P2-2 | code | HOME_SERVER_RUNBOOK; readers: deploy/website.rs:232,426,442 |
| `tools_v2/xtask/src/core/repository_layout.rs:85` | `docs/mod/STAGING-SERVER.md` | `documentation_v2/runbooks/game_server_staging/README.md` | P2-2 | code | STAGING_SERVER_RUNBOOK; F14 keeps the quoted [TBD] Stage Print text; readers: mod_ops/development_server.rs:23, setup/staging_server.rs:124 |
| `tools_v2/xtask/src/core/repository_layout.rs:88` | `docs/mod/SLICE_WORKFLOW.md` | `documentation_v2/runbooks/mod_slice_workflow.md` | P2-2 | code | SLICE_WORKFLOW_RUNBOOK; readers: mod_ops/wave_execution.rs:39, platform/slice_worktree.rs:41, verify_crf_leak.rs:41 |
| `tools_v2/xtask/src/core/repository_layout.rs:91` | `docs/platform/PLATFORM_FACTORY.md` | `documentation_v2/runbooks/factory_waves/README.md` | P2-2 | code | PLATFORM_FACTORY_RUNBOOK; readers: platform/wave_execution/mod.rs:77 |
| `tools_v2/xtask/src/core/repository_layout.rs:94` | `docs/mod/TBD_MOD_DESIGN.md` | `documentation_v2/mod/tbd-framework/mod_design.md` | P2-2 | code | MOD_DESIGN; readers: verify_crf_leak.rs:40 |
| `tools_v2/xtask/src/core/repository_layout.rs:97` | `docs/mod/SPAWN_DETERMINISM.md` | `documentation_v2/runbooks/spawn_determinism.md` | P2-2 | code | SPAWN_DETERMINISM_RUNBOOK; readers: verifications/mod_scripts/spawn_determinism.rs:83 |
| `tools_v2/xtask/src/verifications/architecture/engine_layer_boundaries.rs:1` | `ENGINE_SPLIT_PROGRAM.md §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | comment | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs:194` | `ENGINE_SPLIT_PROGRAM.md §1` | `documentation_v2/standards/engine_boundary_rules.md §1` | P2-2 | code | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs:199` | `ENGINE_SPLIT_PROGRAM.md §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | code | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs:204` | `ENGINE_SPLIT_PROGRAM.md §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | code | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs:209` | `ENGINE_SPLIT_PROGRAM.md §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | code | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs:214` | `ENGINE_SPLIT_PROGRAM.md §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | code | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs:221` | `ENGINE_SPLIT_PROGRAM.md §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | code | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs:227` | `ENGINE_SPLIT_PROGRAM.md §5` | `documentation_v2/standards/engine_boundary_rules.md §5` | P2-2 | code | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/verifications/architecture/engine_layer_rules.rs:232` | `ENGINE_SPLIT_PROGRAM.md §2D` | `documentation_v2/standards/engine_boundary_rules.md §2D` | P2-2 | code | F09 writes standards/engine_boundary_rules.md in Phase 5 and keeps the §-rule numbering or tells P2-2 the new anchors; the message may point at it before it exists |
| `tools_v2/xtask/src/verifications/architecture/route_tags.rs:6` | `CODING_STANDARDS.md` | `documentation_v2/standards/coding_standards/README.md` | P2-2 | comment | section and rule ids are content pins: F17 keeps them in the new document |
| `tools_v2/xtask/src/verifications/architecture/route_tags.rs:27` | `DOCUMENTATION_STANDARDS.md §3.1` | `documentation_v2/standards/documentation_standards.md §3.1` | P2-2 | comment | section and rule ids are content pins: P3-2 keeps them in the new document |
| `tools_v2/xtask/src/verifications/language_bans/shell_scripts.rs:170` | `CODING_STANDARDS.md` | `documentation_v2/standards/coding_standards/README.md` | P2-2 | code | section and rule ids are content pins: F17 keeps them in the new document |
| `tools_v2/xtask/src/verifications/licensing/upstream_code_leaks.rs:35` | `SLICE_WORKFLOW.md` | `documentation_v2/runbooks/mod_slice_workflow.md` | P2-2 | comment | - |
| `tools_v2/xtask/src/verifications/licensing/upstream_code_leaks/verify_crf_leak.rs:39-41` | `"See {} §2 and {} §Oracle lanes." over MOD_DESIGN and SLICE_WORKFLOW_RUNBOOK` | (format unchanged; the constants carry the new paths) | P2-2 | code | §2 is mod_design.md's (F10 keeps it); §Oracle lanes is mod_slice_workflow.md's (F15 keeps it), not TBD_MOD_DESIGN's as the plan's F10 row says; F15 keeps rule 1, §Sources, §What agents cannot do and §Oracle lanes |
| `tools_v2/xtask/src/verifications/schemas/checks/contract_citations.rs:166` | `docs/` | `documentation_v2/` | P2-2 | code | - |
| `tools_v2/xtask/src/verifications/schemas/checks/contract_citations.rs:167` | `DOCUMENTATION_STANDARDS §10` | `documentation_v2/standards/documentation_standards.md §10` | P2-2 | code | section and rule ids are content pins: P3-2 keeps them in the new document |
| `.ai/tickets/queue.json:49 spec lines` | `"spec": "docs/..."` | regenerated by `cargo xtask ticket sync` after P2-3 rewrites the ticket spec fields | P2-3 | config | never hand-edited |
| `.ai/tickets/wave.lock:41 owns lines` | `docs/... owns entries copied from ticket owns` | regenerated by `cargo xtask wave repack` after P2-3 rewrites the ticket owns | P2-3 | config | never hand-edited; two tickets' owns lines naming the archived wave-plan TSVs stay as written (their ticket strings are excluded from the rewrite TSV) |
| `README.md:9` | `docs/platform/WHERE_DOES_X_GO.md` | `/documentation_v2/standards/where_does_x_go.md` | P2-6 | prose | - |
| `README.md:13` | `docs/specs/` | `/documentation_v2/tickets/specs/ (non-spec design docs sit in their feature folders)` | P2-6 | prose | - |
| `README.md:14` | `docs/platform/` | `/documentation_v2/runbooks/ and documentation_v2/standards/ (the folder splits)` | P2-6 | prose | - |
| `README.md:15` | `docs/mod/ · docs/website/` | `/documentation_v2/mod/ · /documentation_v2/website/` | P2-6 | prose | - |
| `README.md:18` | `docs/website/CURSOR_SETUP.md` | `/documentation_v2/runbooks/cursor_workspace_setup.md` | P2-6 | prose | - |
| `README.md:35` | `docs/platform/MONOREPO_MIGRATION.md` | `/documentation_v2/archive/monorepo_migration/monorepo_migration_runbook.md` | P2-6 | prose | - |
| `apps/fleet_host_agent/README.md:9` | `docs/verification/api_v2/fleet_command_ledger.md` | `/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md` | P2-6 | prose | - |
| `apps/mod/README.md:19` | `docs/mod/STAGING-SERVER.md` | `/documentation_v2/runbooks/game_server_staging/README.md` | P2-6 | prose | F14 keeps the quoted [TBD] Stage Print text |
| `apps/mod/README.md:20` | `CLAUDE-CONTINUATION.md §16` | `/documentation_v2/archive/handoffs_and_kickoffs/mod_claude_continuation.md §16` | P2-6 | prose | - |
| `apps/mod/README.md:21` | `docs/mod/MILESTONES.md` | `/documentation_v2/archive/product_plans/mod_milestones.md` | P2-6 | prose | - |
| `apps/mod/README.md:29` | `docs/mod/CLAUDE-CODE-START.md` | `/documentation_v2/runbooks/mod_slice_workflow.md` | P2-6 | prose | - |
| `apps/mod/README.md:56` | `docs/mod/STAGING-SERVER.md` | `/documentation_v2/runbooks/game_server_staging/README.md` | P2-6 | prose | F14 keeps the quoted [TBD] Stage Print text |
| `apps/mod/README.md:81` | `docs/mod/ · docs/mod/STAGING-SERVER.md` | `/documentation_v2/mod/ · /documentation_v2/runbooks/game_server_staging/README.md` | P2-6 | prose | F14 keeps the quoted [TBD] Stage Print text |
| `apps/mod/README.md:83` | `docs/mod/CLAUDE-CONTINUATION.md · docs/mod/MILESTONES.md · docs/mod/tbd-reforger-platform-build-plan.md` | `/documentation_v2/archive/handoffs_and_kickoffs/mod_claude_continuation.md · /documentation_v2/archive/product_plans/mod_milestones.md · /documentation_v2/archive/product_plans/tbd_reforger_platform_build_plan.md` | P2-6 | prose | - |
| `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md:51` | `docs/verification/equipment-vehicle-export/` | `/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/` | P2-6 | prose | - |
| `apps/mod/tbd-framework/README.md:111` | `docs/STAGING-SERVER.md · docs/mod/STAGING-SERVER.md` | `/documentation_v2/runbooks/game_server_staging/README.md · /documentation_v2/runbooks/game_server_staging/README.md` | P2-6 | prose | F14 keeps the quoted [TBD] Stage Print text |
| `apps/website/README.md:24` | `docs/website/DEV_RUNBOOK.md` | `/documentation_v2/runbooks/local_development.md` | P2-6 | prose | - |
| `apps/website/README.md:29` | `docs/platform/WHERE_DOES_X_GO.md · docs/website/DEV_RUNBOOK.md` | `/documentation_v2/standards/where_does_x_go.md · /documentation_v2/runbooks/local_development.md` | P2-6 | prose | - |
| `apps/website/README.md:33` | `docs/website/README.md` | `/documentation_v2/README.md` | P2-6 | prose | - |
| `apps/website/README.md:38` | `docs/website/frontend/ROADMAP.md` | `/documentation_v2/website/frontend/README.md` | P2-6 | prose | - |
| `apps/website/README.md:39` | `docs/website/backend/ROADMAP.md` | `/documentation_v2/website/api_v2/api_overview.md` | P2-6 | prose | - |
| `apps/website/README.md:40` | `docs/specs/Mission_Creator_Architecture/ROADMAP.md` | `/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md` | P2-6 | prose | - |
| `apps/website/README.md:42` | `docs/website/archive/README.md` | `/documentation_v2/archive/monorepo_migration/docs_website_archive_readme.md` | P2-6 | prose | - |
| `apps/website/api_v2/README.md:9` | `ARCHITECTURE_PLAN.md` | `/documentation_v2/archive/api_v2_refactor/architecture_plan.md` | P2-6 | prose | - |
| `apps/website/api_v2/README.md:10` | `ANALYSIS_AND_INVENTORY.md` | `/documentation_v2/archive/api_v2_refactor/analysis_and_inventory.md` | P2-6 | prose | - |
| `apps/website/api_v2/README.md:154` | `ARCHITECTURE_PLAN.md` | `/documentation_v2/archive/api_v2_refactor/architecture_plan.md` | P2-6 | prose | - |
| `apps/website/api_v2/README.md:156` | `ANALYSIS_AND_INVENTORY.md` | `/documentation_v2/archive/api_v2_refactor/analysis_and_inventory.md` | P2-6 | prose | - |
| `apps/website/api_v2/README.md:158` | `PHASE_1_HANDOFF.md · PHASE_7_HANDOFF.md` | `/documentation_v2/archive/api_v2_refactor/phase_1_handoff.md · /documentation_v2/archive/api_v2_refactor/phase_7_handoff.md` | P2-6 | prose | - |
| `apps/website/api_v2/src/server_infrastructure/README.md:34` | `docs/verification/api_v2/fleet_command_ledger.md` | `/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md` | P2-6 | prose | - |
| `apps/website/frontend/src/v2/README.md:58` | `docs/platform/ENGINE_SPLIT_PROGRAM.md` | `/documentation_v2/archive/engine_split/engine_split_program.md` | P2-6 | prose | points at the archived program until F09 writes standards/engine_boundary_rules.md (Phase 5); F09 re-points |
| `apps/website/frontend/src/v2/pages/navigation/README.md:9` | `nav_config.md` | `/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md` | P2-6 | prose | - |
| `apps/website/map-engine/src/frame/tests/README.md:8` | `ENGINE_SPLIT_PROGRAM.md §2C` | `/documentation_v2/archive/engine_split/engine_split_program.md §2C` | P2-6 | prose | points at the archived program until F09 writes standards/engine_boundary_rules.md (Phase 5); F09 re-points |
| `assets_v2/README.md:12` | `ARCHITECTURE_PLAN.md` | `/documentation_v2/archive/assets_v2_relocation/architecture_plan.md` | P2-6 | prose | - |
| `assets_v2/README.md:13` | `ANALYSIS_AND_INVENTORY.md` | `/documentation_v2/archive/assets_v2_relocation/analysis_and_inventory.md` | P2-6 | prose | - |
| `assets_v2/README.md:70` | `ARCHITECTURE_PLAN.md` | `/documentation_v2/archive/assets_v2_relocation/architecture_plan.md` | P2-6 | prose | - |
| `assets_v2/README.md:71` | `ANALYSIS_AND_INVENTORY.md` | `/documentation_v2/archive/assets_v2_relocation/analysis_and_inventory.md` | P2-6 | prose | - |
| `assets_v2/README.md:75` | `MIGRATION_HANDOFF.md` | `/documentation_v2/archive/assets_v2_relocation/migration_handoff.md` | P2-6 | prose | - |
| `contracts_v2/README.md:12` | `ARCHITECTURE_PLAN.md` | `/documentation_v2/archive/contracts_v2_relocation/architecture_plan.md` | P2-6 | prose | - |
| `contracts_v2/README.md:13` | `ANALYSIS_AND_INVENTORY.md` | `/documentation_v2/archive/contracts_v2_relocation/analysis_and_inventory.md` | P2-6 | prose | - |
| `contracts_v2/README.md:53` | `ARCHITECTURE_PLAN.md` | `/documentation_v2/archive/contracts_v2_relocation/architecture_plan.md` | P2-6 | prose | - |
| `contracts_v2/README.md:73` | `ARCHITECTURE_PLAN.md` | `/documentation_v2/archive/contracts_v2_relocation/architecture_plan.md` | P2-6 | prose | - |
| `contracts_v2/README.md:74` | `ANALYSIS_AND_INVENTORY.md` | `/documentation_v2/archive/contracts_v2_relocation/analysis_and_inventory.md` | P2-6 | prose | - |
| `contracts_v2/README.md:75` | `MIGRATION_HANDOFF.md` | `/documentation_v2/archive/contracts_v2_relocation/migration_handoff.md` | P2-6 | prose | - |
| `contracts_v2/definitions/README.md:69` | `bridge-messages.md` | `/documentation_v2/contracts_v2/definitions/bridge_messages.md` | P2-6 | prose | - |
| `CLAUDE.md:207` | `docs/` | `/documentation_v2/` | G1 | prose | - |
| `CLAUDE.md:208` | `docs/` | `/documentation_v2/` | G1 | prose | - |
| `.ai/tickets/AI_PLAYBOOK.md:9` | `docs/TICKET_*.md · docs/specs/Mission_Creator_Architecture/ROADMAP.md · gap_analysis` | (retired view: drop the mention or point at apps/ticketboard, which reads the ticket files) · /documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md · /documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md | G2 | prose | - |
| `.ai/tickets/AI_PLAYBOOK.md:38` | `docs/website/AGENT_COMMIT_CHECKLIST.md` | `/documentation_v2/standards/commit_checklist.md` | G2 | prose | - |
| `.ai/tickets/AI_PLAYBOOK.md:43` | `docs/specs/Mission_Creator_Architecture/t068_asset_registry.md` | `/documentation_v2/tickets/specs/t068_asset_registry.md` | G2 | prose | - |
| `.ai/tickets/CLAUDE_CODE_PROMPT.md:3` | `docs/handoffs` | (no such folder: G2 rewords the audience line) | G2 | prose | - |
| `.ai/tickets/CLAUDE_CODE_PROMPT.md:13` | `docs/specs/**/t0xx_*.md` | `/documentation_v2/tickets/specs/t0xx_*.md` | G2 | prose | - |
| `.ai/tickets/CLAUDE_CODE_PROMPT.md:44` | `docs/specs/.../t0xx_slice_spec.md` | `/documentation_v2/tickets/specs/t0xx_slice_spec.md` | G2 | prose | - |
| `.ai/tickets/CLAUDE_CODE_PROMPT.md:74` | `docs/** · docs/TICKET_*.md` | `/documentation_v2/** · (retired view: drop the mention or point at apps/ticketboard, which reads the ticket files)` | G2 | prose | - |
| `.ai/tickets/CLAUDE_CODE_PROMPT.md:177` | `docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md` | `/documentation_v2/tickets/specs/t090_1_2_2_sap_cell_seam_repair.md` | G2 | prose | - |
| `.ai/tickets/HANDOFF_TEMPLATE.md:16` | `docs/specs/.../t0xx_slice.md` | `/documentation_v2/tickets/specs/t0xx_slice.md` | G2 | prose | - |
| `.ai/tickets/README.md:3` | `docs/TICKET_*.md` | (retired view: drop the mention or point at apps/ticketboard, which reads the ticket files) | G2 | prose | - |
| `.ai/tickets/SPEC_TEMPLATE.md:65` | `docs/website/AGENT_COMMIT_CHECKLIST.md` | `/documentation_v2/standards/commit_checklist.md` | G2 | prose | - |
| `.ai/tickets/SPEC_TEMPLATE.md:98` | `docs/specs/.../t0xx_{slug}.md` | `/documentation_v2/tickets/specs/t0xx_{slug}.md` | G2 | prose | - |
| `.ai/tickets/SPEC_TEMPLATE.md:113` | `docs/** · docs/TICKET_*.md` | `/documentation_v2/** · (retired view: drop the mention or point at apps/ticketboard, which reads the ticket files)` | G2 | prose | - |
| `.cursor/rules/acceptance-gates-reproducible.mdc:31` | `docs/platform/known-bugs/` | `/documentation_v2/known_bugs/` | G2 | prose | - |
| `.cursor/rules/acceptance-gates-reproducible.mdc:32` | `docs/website/EDITOR_GATE_RUNBOOK.md` | `/documentation_v2/runbooks/editor_gates.md` | G2 | prose | - |
| `.cursor/rules/application-code-forbidden.mdc:38` | `docs/` | `/documentation_v2/` | G2 | prose | - |
| `.cursor/rules/claude-prompt-delivery.mdc:14` | `.ai/tickets/CLAUDE_CODE_PROMPT.md` | `.ai/tickets/implementation_prompt.md` | G2 | prose | G2 renames the file (plan Phase 6); not a manifest row |
| `.cursor/rules/cursor-agent-workflow.mdc:20-21` | `Cursor owns docs; Claude Code must not write documentation` | (the split goes, as in tbd-platform.mdc) | G2 | prose | doc-ownership decision |
| `.cursor/rules/cursor-agent-workflow.mdc:75` | `docs/platform/FACTORY_FOR_CURSOR.md` | `/documentation_v2/runbooks/factory_waves/README.md` | G2 | prose | - |
| `.cursor/rules/cursor-agent-workflow.mdc:114` | `docs/platform/CODEBASE_AUDIT_2026.md` | `/documentation_v2/archive/audits/codebase_audit_2026.md` | G2 | prose | - |
| `.cursor/rules/platform-factory-mode.mdc:12` | `docs/platform/FACTORY_FOR_CURSOR.md` | `/documentation_v2/runbooks/factory_waves/README.md` | G2 | prose | - |
| `.cursor/rules/platform-factory-mode.mdc:14` | `docs/platform/PLATFORM_FACTORY.md` | `/documentation_v2/runbooks/factory_waves/README.md` | G2 | prose | - |
| `.cursor/rules/platform-factory-mode.mdc:22` | `docs/registry/plan-review` | (no such folder: G2 rewords) | G2 | prose | - |
| `.cursor/rules/tbd-platform.mdc:2, 16-17` | `Cursor owns docs, Claude Code owns code slices` | (the split goes: documentation ships in the same commit as the code it describes) | G2 | prose | doc-ownership decision; the writing brief's contradiction list names these lines |
| `.cursor/rules/tbd-platform.mdc:11` | `docs/specs/Mission_Creator_Architecture/` | `/documentation_v2/website/frontend/apps/editor/` | G2 | prose | - |
| `.cursor/rules/tbd-platform.mdc:12` | `docs/platform/WHERE_DOES_X_GO.md` | `/documentation_v2/standards/where_does_x_go.md` | G2 | prose | - |
| `.cursor/rules/tbd-platform.mdc:13` | `docs/website/frontend/` | `/documentation_v2/website/frontend/` | G2 | prose | - |
| `.cursor/rules/tbd-platform.mdc:15` | `docs/TICKET_*.md` | (retired view: drop the mention or point at apps/ticketboard, which reads the ticket files) | G2 | prose | - |
| `.cursor/rules/tbd-platform.mdc:25` | `docs/platform/FACTORY_FOR_CURSOR.md` | `/documentation_v2/runbooks/factory_waves/README.md` | G2 | prose | - |
| `apps/mod/.cursor/rules/single-branch-main.mdc:10` | `git checkout -b ban (Law 2 wording)` | `adds the slice/<id> tooling exception` | G2 | prose | no documentation path; listed because the explorer pin audit names it |
| `tools_v2/xtask/src/commands/ci/task_definitions.rs:120, 130-138` | `verify-doc-layout step and task row` | (deleted; one verify-documentation row added) | H1 | code | H1 also updates ci_local_step_set_is_frozen |
| `tools_v2/xtask/src/commands/ci/task_definitions.rs:132` | `DOCUMENTATION_STANDARDS §8.2` | (row deleted) | H1 | code | verify-doc-layout task row :130-138; DOCUMENTATION_STANDARDS §8.2; section and rule ids are content pins: P3-2 keeps them in the new document |
| `tools_v2/xtask/src/commands/ci/task_runner.rs:5, 16, 56, 120, 213, 216` | `verify-doc-layout in docs and imports` | (deleted) | H1 | comment | with the task |
| `tools_v2/xtask/src/commands/ci/task_runner.rs:144` | `docs/` | (deleted) | H1 | comment | doc_layout_refusal :142-153; LAYOUT_TARGET_DIR read at :151 |
| `tools_v2/xtask/src/commands/ci/task_runner.rs:149` | `docs/` | (deleted) | H1 | code | as line 144 |
| `tools_v2/xtask/src/commands/ci/task_runner/split_cmd.rs:239` | `DOCUMENTATION_STANDARDS §8.2` | (deleted) | H1 | comment | verify-doc-layout predicate and runner :239-285; section and rule ids are content pins: P3-2 keeps them in the new document |
| `tools_v2/xtask/src/commands/ci/task_runner/split_cmd.rs:240` | `docs/*.md` | (deleted) | H1 | comment | as line 239 |
| `tools_v2/xtask/src/commands/ci/task_runner/split_cmd.rs:246` | `docs/*.md` | (deleted) | H1 | comment | as line 239 |
| `tools_v2/xtask/src/commands/ci/task_runner/split_cmd.rs:248` | `docs/*.md` | (deleted) | H1 | comment | as line 239 |
| `tools_v2/xtask/src/commands/ci/task_runner/split_cmd.rs:249` | `docs/` | (deleted) | H1 | comment | as line 239 |
| `tools_v2/xtask/src/commands/ci/task_runner/split_cmd.rs:252` | `docs/` | (deleted) | H1 | code | as line 239 |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:9` | `doc_layout_predicate_reproduces_finds_globs in the module doc` | (deleted) | H1 | comment | with the test |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:268` | `docs/*.md` | (test deleted) | H1 | test | doc_layout_predicate_reproduces_finds_globs :264-280 |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:269` | `docs/spec.md` | (test deleted) | H1 | test | as line 268 |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:270` | `docs/a/b/c.md` | (test deleted) | H1 | test | as line 268 |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:271` | `docs/y.md` | (test deleted) | H1 | test | as line 268 |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:273` | `docs/readme.md` | (test deleted) | H1 | test | as line 268 |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:274` | `docs/` | (test deleted) | H1 | test | as line 268 |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:275` | `docs/spec.txt` | (test deleted) | H1 | test | as line 268 |
| `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:277` | `docs/` | (test deleted) | H1 | test | as line 268 |
| `tools_v2/xtask/src/core/repository_layout.rs:79` | `LAYOUT_TARGET_DIR = "docs/website/"` | (deleted) | H1 | code | stale between commit 2a and H1 (the refusal text names a gone folder); P2-2 may point it at documentation_v2/ in 2a |
| `tools_v2/ticket-engine/src/metrics/estimates/tests/estimate_provenance_tests.rs:5-22` | `factor doc must quote ".ai/", "docs/TICKET_", "Cargo.lock"` | (unchanged) | none | test | MUST NOT CHANGE: the moved token_estimate_factor.md keeps quoting the three prefixes verbatim (F12 pin); the test reads TOKEN_ESTIMATE_FACTOR_DOC |
| `tools_v2/ticket-engine/src/metrics/estimates/tests/estimate_provenance_tests.rs:29-30` | `is_excluded_path(docs/TICKET_LEAD.md), (docs/TICKET_REGISTRY.md)` | (unchanged) | none | test | MUST NOT CHANGE: the retired prefix still excludes historical view paths |
| `tools_v2/ticket-engine/src/repository.rs:225` | `NUMSTAT_EXCLUDED_PREFIXES [.ai/, GENERATED_QUEUE_VIEW_PREFIX]` | (unchanged) | none | code | MUST NOT CHANGE: historical numstat still spells the retired view prefix |
| `tools_v2/ticket-engine/src/repository.rs:233-234` | `ARCHIVED_WAVE_PLANS (the two archived wave-plan TSV paths)` | (unchanged) | none | code | MUST NOT CHANGE: read at historical revisions through git show; this catalogue never spells the two values |
| `tools_v2/xtask/src/tests/tooling_prose_rules.rs:91` | `REPOSITORY_PATH_LITERAL = r#""(scripts\|docs\|\.ai\|documentation_v2)/"#` | (unchanged) | none | test | keeps docs so a retired-root literal outside a layout module still fails; documentation_v2 is already in it |
| `apps/ticketboard/src/application/tests/rendering.rs:148,151,155,161` | `docs/... strings` | (unchanged) | none | test | synthetic: rendering fixture paths; P2-4's grep excludes it |
| `apps/ticketboard/src/application/tests/window_layout.rs:43` | `docs/... strings` | (unchanged) | none | test | synthetic: viewer open() argument, never read; P2-4's grep excludes it |
| `apps/ticketboard/src/document_viewer/services/tests/document_loading.rs:12,13,15,17,31,32,54,58,62,66,74,80,81,85,111,112,113,117,124,129,132,142,143,147,148,151,152,163,164,171,176,266,268,278,282,316,317` | `docs/... strings` | (unchanged) | none | test | synthetic: viewer path-resolution cases over a scratch root; P2-4's grep excludes it |
| `apps/ticketboard/src/ticket_actions/services/commands/tests/commands.rs:37,40,44` | `docs/... strings` | (unchanged) | none | test | synthetic: command-line text cases; P2-4's grep excludes it |
| `apps/ticketboard/src/ticket_browser/models/tests/detail_sections.rs:185` | `docs/... strings` | (unchanged) | none | test | synthetic: TOML fixture strings; P2-4's grep excludes it |
| `apps/ticketboard/src/ticket_browser/models/tests/status_board.rs:402,405,412,421` | `docs/... strings` | (unchanged) | none | test | synthetic: TOML fixture strings; P2-4's grep excludes it |
| `apps/ticketboard/src/wave_plan/models/tests/wave_projection.rs:34,88` | `docs/... strings` | (unchanged) | none | test | synthetic: TOML fixture strings; P2-4's grep excludes it |
| `tools_v2/ticket-engine/src/cli/tests/command_mutation_tests.rs:436,467,471` | `docs/... strings` | (unchanged) | none | test | synthetic: synthetic spec paths inside the scratch fixture; P2-4's grep excludes it |
| `tools_v2/ticket-engine/src/cli/tests/mod.rs:93,94,190,193,218,219,226,227` | `docs/... strings` | (unchanged) | none | test | synthetic: fixture writes its own docs/ folder (line 65); P2-4's grep excludes it |
| `tools_v2/ticket-engine/src/metrics/estimates/tests/estimate_provenance_tests.rs:33,35,455` | `docs/... strings` | (unchanged) | none | test | synthetic: asserts a path is not excluded; stays true at any path; owns string; suffix-rule case; P2-4's grep excludes it |
| `tools_v2/ticket-engine/src/ops/tests/status_and_shipping_tests.rs:620,622` | `docs/... strings` | (unchanged) | none | test | synthetic: missing-file case; P2-4's grep excludes it |
| `tools_v2/ticket-engine/src/registry/ticket_file_storage/tests/ticket_file_storage_tests.rs:98,99,115,122` | `docs/... strings` | (unchanged) | none | test | synthetic: TOML round-trip strings; P2-4's grep excludes it |
| `tools_v2/ticket-engine/src/tests/encoding/encoding_roundtrip_tests.rs:189,190,207,372,399` | `docs/... strings` | (unchanged) | none | test | synthetic: encoding round-trip strings; P2-4's grep excludes it |
| `tools_v2/ticket-engine/src/validation/tests/readiness_and_accounting_tests.rs:20,81,291` | `docs/... strings` | (unchanged) | none | test | synthetic: synthetic spec and owns strings in scratch TOML; P2-4's grep excludes it |
| `tools_v2/ticket-engine/src/validation/tests/schema_and_integrity_tests.rs:105,322,609` | `docs/... strings` | (unchanged) | none | test | synthetic: owns string; spec string in a plan-gate case; P2-4's grep excludes it |
| `tools_v2/xtask/src/tests/tooling_prose_rules.rs:399,447` | `docs/... strings` | (unchanged) | none | test | synthetic: synthetic offender the rule must catch; P2-4's grep excludes it |
