**Status:** live — Documentation V2 move manifest summary: checkpoint 1 answered

# Questions for checkpoint CP1

Every open question from the manifest agents (P0-2 mechanical rows, P0-3 judgment rows, P0-4
ticket plan) and from the assembly (P0-5), merged where two agents asked the same thing. Each has
its options, a recommended answer and the operator's answer: the operator accepted every
recommendation at checkpoint CP1. The manifest, the rewrite TSVs and the pin catalogue follow all
21 answers; questions 17, 18 and 20 concern tooling and tickets that P1-2b changes and files. The
rules writers follow from these answers are in the writing brief's "Checkpoint 1 decisions"
section ([refactor_writing_brief.md](/documentation_v2/refactor_writing_brief.md)). The folder
index is [README.md](README.md).

## Names and placement

1. **Visual reference set names** (P0-2). Sets are named `<subject>_<kind>`, kind `blueprint`,
   `mockup` or `render` (`mortar_calculator_blueprint`, `mission_creator_shell_blueprint`,
   `satellite_backdrop_render`, `arsenal_mockup`); the mod Stitch sets keep their panel folder
   name without `tbd_reforger_` and `_standalone` (`scenario_browser_mockup`,
   `ultra_clear_2_second_end_screen_banner_mockup`). The full list is in
   [archive_and_reference_sets.md](archive_and_reference_sets.md).
   Options: (a) accept all; (b) accept all but rename `scenario_browser_mockup` to
   `mission_header_browser_mockup`, the program's term for Enfusion's world and game-mode
   config; (c) set a shorter naming rule for the mod panels.
   **Recommended: (b).**
   **Operator answer (2026-09-23): recommended option accepted.**
2. **Dashboard and server-intel sets** (P0-2). `command_dashboard_blueprint` and
   `server_intel_blueprint` sit in `pages/command_center/visual_references/`, while the code
   has `command_center/{dashboard,server_intel}/` and their page docs already sit in
   `pages/command_center/dashboard/` and `pages/command_center/server_intel/`.
   Options: (a) move each set into its page folder; (b) keep them at the area folder.
   **Recommended: (a)**, one feature in one folder.
   **Operator answer (2026-09-23): recommended option accepted.**
3. **Evidence folder name** (P0-2). The tbd-export evidence lands in
   `mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification/`, the mirror of a
   code folder whose child `Verification/` differs only in case, so a later mirror of that child
   collides on a case-insensitive filesystem.
   Options: (a) name the evidence folder `verification_evidence/`, here and in
   `website/api_v2/` (51 rows; the api-readiness constants P1-4 adds and P2-2 sets follow); (b)
   keep `verification/`.
   **Recommended: (a).**
   **Operator answer (2026-09-23): recommended option accepted.**
4. **Data files that change extension** (P0-2). The redirect stub `09_eden_wiki_manifest.yaml`
   holds a Markdown pointer, so it is archived as `…__09_eden_wiki_manifest.md`; the two size
   listings `docs/website/.doc-manifest.{before,after}` become
   `archive/monorepo_migration/doc_manifest_{before,after}.txt`.
   Options: (a) confirm; (b) keep the original extensions.
   **Recommended: (a).**
   **Operator answer (2026-09-23): recommended option accepted.**
5. **Eden wiki scrape manifest** (P0-3). `eden/wiki_manifest.yaml` (114 lines) records the 28
   scraped Eden wiki pages the catalogs cite, and the Mission Creator roadmap links it; the
   script that read it is gone.
   Options: (a) data beside the Eden catalogs,
   `apps/editor/eden_editor_reference/eden_wiki_scrape_manifest.yaml` (F08 indexes it); (b)
   archive it.
   **Recommended: (a).**
   **Operator answer (2026-09-23): recommended option accepted.**
6. **Vanilla source coverage** (P0-3). `docs/mod/vanilla_carve_coverage.md` holds the only full
   lane-4 procedure (`cargo xtask fetch vanilla-source`, `enf source`, `enf index vanilla`, the
   apidoc, the fallbacks, the politeness rule), and T-181.3.x cite it.
   Options: (a) live reference `mod/tbd-framework/vanilla_source_coverage.md` (F10 splits the
   dated research narrative into the archive); (b) a runbook; (c) `archive/audits/`.
   **Recommended: (a).**
   **Operator answer (2026-09-23): recommended option accepted.**
7. **Shipped-history log** (P0-3). `docs/platform/SHIPPED_HISTORY.md` is the relocated status
   log of every program.
   Options: (a) a new topic `archive/shipped_history/`; (b) `archive/handoffs_and_kickoffs/`.
   **Recommended: (a).**
   **Operator answer (2026-09-23): recommended option accepted.**
8. **macOS UX methodology** (P0-3). `docs/website/platform/macos_ux_architecture.md` audits
   React-era pages, but the methodology it names (context retention, progressive disclosure,
   frictionless action; split pane, create-over-list dialog, inline toggles, slide-over dossier)
   is cited as law by `TBD_UIInteractive.c:8` and TBD_MOD_DESIGN §2.
   Options: (a) archive it and have F17 write the methodology into `design_system/`; (b) keep
   the file live.
   **Recommended: (a).**
   **Operator answer (2026-09-23): recommended option accepted.**
9. **Frontend page template** (P0-3). `docs/website/frontend/_template.md` is a React Query page
   template; the plan lists it as an F04 source.
   Options: (a) archive (P3-1's feature-doc template replaces it); (b) `pending_merge/F04/`.
   **Recommended: (a).**
   **Operator answer (2026-09-23): recommended option accepted.**
10. **Mod agent start file** (P0-3). `docs/mod/CLAUDE-CODE-START.md` mixes MCP setup with a
    dated T-068/T-090/T-091 status half.
    Options: (a) merge into `runbooks/mod_slice_workflow.md` (F15; F11 reads the MCP part first,
    F15 archives the dated half); (b) archive it whole (MCP_TOOLING carries most live facts).
    **Recommended: (a)**, as planned.
    **Operator answer (2026-09-23): recommended option accepted.**
11. **Resolved known bug** (P0-3). KB-002 is resolved (T-177).
    Options: (a) keep it in the live `known_bugs/` registry with status resolved; (b) archive it.
    **Recommended: (a).**
    **Operator answer (2026-09-23): recommended option accepted.**
12. **Referee ticket panel** (P0-3). The Stitch `admin_tickets_panel` is the referee side of the
    help-ticket feature.
    Options: (a) `UI/admin_help_ticket/`; (b) `UI/in_game_menu/` with the other admin panels.
    **Recommended: (a).**
    **Operator answer (2026-09-23): recommended option accepted.**
13. **Feature doc names** (P0-3). One file per feature beside the folder README:
    `<page component>_page.md` in each page folder (`personnel_roster_page.md`),
    `account_pages.md`, `app_layout_and_navigation.md`, and `<screen>_specification.md` for mod
    screens.
    Options: (a) separate files as listed; (b) a README.md that is both index and feature doc.
    **Recommended: (a).**
    **Operator answer (2026-09-23): recommended option accepted.**
14. **Frontend README primary** (P0-3). The frontend folder README takes one primary source.
    Options: (a) `docs/website/frontend/ROADMAP.md`, the richest (route table); (b)
    `docs/website/frontend/README.md`, the literal hub.
    **Recommended: (a).**
    **Operator answer (2026-09-23): recommended option accepted.**
15. **pending_merge names** (P0-3). Secondary sources keep their original names under
    `pending_merge/<writer>/` (uppercase and hyphens stay), which the snake_case rule must exempt.
    Options: (a) exempt `pending_merge/`, which is empty by the end of Phase 5; (b) snake_case
    the names now.
    **Recommended: (a)**; the manifest applies it.
    **Operator answer (2026-09-23): recommended option accepted.**

## Tickets

16. **Citation value format** (P0-4). Fifteen orphan specs join a ticket's `citations`.
    Options: (a) `Design: <path>.`, plain text in ticketboard, like the 339 existing
    `Source: … .` entries; (b) the bare `<path>`, which ticketboard opens.
    **Recommended: (a)**; [refactor_orphan_spec_links.tsv](/documentation_v2/refactor_orphan_spec_links.tsv)
    uses it.
    **Operator answer (2026-09-23): recommended option accepted.**
17. **Top-level ticket ids run out at T-999** (P0-4). `.ai/tickets/schema.json:18`
    (`^T-[0-9]{3}(\.[0-9]+)*$`) and `tools_v2/ticket-engine/src/sync/gap_analysis.rs:12` allow
    three digits; T-999 exists, so the next `ticket add` mints T-1000 and every later
    `ticket check` fails.
    Options: (a) widen both patterns to `[0-9]{3,}` in Phase 1 (P1-2) and file the three
    follow-up tickets top-level; (b) file them as children of existing programs.
    **Recommended: (a).**
    **Operator answer (2026-09-23): recommended option accepted.**
18. **Status of the scenario-to-mission rename ticket** (P0-4). The operator deferred the rename
    program until this one ends; `queued` makes a ticket dispatchable to the wave packer.
    Options: (a) `deferred`; (b) `queued`.
    **Recommended: (a).**
    **Operator answer (2026-09-23): recommended option accepted.**
19. **Bare file names in tickets** (P0-4). Options: (a) rewrite a bare name only where the
    manifest changes the basename (the new string is then the full final path); (b) rewrite
    every bare name.
    **Recommended: (a)**; the rewrite TSV applies it (333 `keep` rows).
    **Operator answer (2026-09-23): recommended option accepted.**
20. **Stale branches, worktrees and tickets** (P0-4). Besides `main` there are 44 local
    branches: 42 merged with nothing ahead, `slice/T-939.4` 6 commits ahead and
    `scratch/track-b-draft` 1 ahead. Five slice worktrees sit under `.ai/artifacts/worktrees/`
    and a sixth outside the repository (`$HOME/.cache/tbd-engine-reorg/baseline`, detached).
    T-212, T-939.2, T-946.55 and T-946.86 are still `ready` although their slice branches are
    merged.
    Options: (a) the cleanup ticket (executor human) also reviews the two unmerged branches,
    prunes the outside worktree and reconciles the four ticket statuses; (b) a separate ticket
    for the statuses.
    **Recommended: (a).**
    **Operator answer (2026-09-23): recommended option accepted.**
21. **Tickets that name a redirect stub** (P0-5). Six bare-name rows name a stub (for example
    `04_eden_editor_ux_spec.md`).
    Options: (a) point them at the stub's real target
    (`documentation_v2/website/frontend/apps/editor/ux_spec.md`); (b) point them at the archived
    stub.
    **Recommended: (a)**; the rewrite TSV applies it.
    **Operator answer (2026-09-23): recommended option accepted.**

## Settled before CP1

- The five near-duplicate collapses are accepted by a plan amendment: against the kept file,
  three DESIGN.md copies differ by a final newline, one by a two-line header and one by both;
  each manifest note gives the exact difference.
- Child-ticket spec checks: P1-2 extends validation to every ticket file and first repoints
  T-068.10.5 and T-159.15.0 at their `.ai/artifacts/` files; the rewrite TSV carries a row for
  both spellings of each.

## Findings for the orchestrator

- `documentation_v2/refactor_program_plan.md:346-347` spells the wave-plan fossil needles, so the
  fossil-path guard fails `ticket check --strict` once xtask compiles again. Rewording those two
  lines clears it.
- `tools_v2/README.md:13-18` links the tools_v2 program records that P1-1 moves, but P1-1's
  YOU OWN names only `tools_v2/ticket-engine/README.md:23`.
- `.editorconfig-checker.json` needs an entry for `documentation_v2/design_system/token_exports/`:
  `aegis_design_tokens.md` leaves an excluded folder and has no final newline.
- `ticket sync` skips the ROADMAP marker and the gap-analysis column silently when their files
  are missing, and the fossil guard needs `ARCHIVED_WAVE_PLAN_READERS` to follow the moved
  files: P2-2's constants must land in commit 2a with P2-1's moves.
- The `refactor_*` program files spell old and future paths by design; the link checker P1-5
  builds should treat them as records until H2 archives them.
