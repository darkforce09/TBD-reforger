**Status:** live — Documentation V2 program plan (approved 2026-09-23)
# Documentation V2 — execution plan (sub-agent orchestrated)

## Context

`docs/` (875 tracked files) is still the authoritative documentation tree. `documentation_v2/`
(148 files) is a draft from 2026-09-16: 59 of its 101 Markdown files are stubs, and all 46 of its
visual-reference files are copies of files in `docs/`. The draft's plan and inventory miss a lot:

- `docs/verification/`
- 1,096 Markdown files outside `docs/`
- about 3.3k internal links, 506 of which are already broken
- 249 path mentions inside ticket bodies
- many code pins (include_str!, api-readiness fingerprint, prose gates, `wave.lock`, EnfScript
  comments, ticketboard's README test)
- 3 README collisions and 22 redirect stubs

Twenty cross-document contradictions exist, and 364 of about 700 code folders lack a README. The
corpus holds about 88k lines of prose (~1.35M tokens). About 500k lines of source code must be
read to check claims.

**Outcome.** `documentation_v2/` is the only documentation tree. It mirrors the code, is verified
against the running code, and follows one README standard plus a template for each document kind.
Every code folder has a README. `docs/` is deleted, and CI gates keep the tree correct.

**Execution model.** All substantive work is done by sub-agents: 77 are planned, plus fix agents as
verifiers require. The main session only orchestrates (details under "Orchestrator protocol").

## Operator decisions (2026-09-23)

| Topic | Decision |
|---|---|
| Scope | `docs/` plus the in-code docs: every README, `page.md`, and program records. Also create the missing READMEs. Agent instruction files get a full refresh. `.ai/artifacts/` is out of scope, except for the 3 spec files cited by 7 tickets, which move. |
| Root name | `documentation_v2/` stays |
| Refresh depth | Every live doc is verified against the code and rewritten |
| History | Archived and frozen under `archive/<topic>/`, with a status line; never reworded |
| Specs and plans | All frozen; only moved-doc links are fixed. `tickets/specs/` is flat and holds only ticket specs (`t<id>_*.md`); `tickets/plans/` is flat. Ticketboard is the reader. |
| New specs | Live while the ticket is idea/queued/ready; frozen once shipped or cancelled. Lasting knowledge then moves to the feature doc. |
| Doc layers | The in-code README describes its own folder at a high level. `documentation_v2` holds the deeper docs, and the README links to them. |
| README rule | Every folder in `apps/`, `tools_v2/`, `contracts_v2/`, `assets_v2/` gets one, mod asset folders included. `tests/`, `generated/` and hidden tool-config folders are exempt. Every existing README is rewritten to the standard. |
| Tree layout | A hybrid code mirror: code folder spellings, minus `apps/`, `src/`, `src/v2/` and `Scripts/Game/TBD/`. Top-level `runbooks/`, `standards/`, `design_system/`, `known_bugs/`, `tickets/`, `archive/`, plus `glossary.md` and `product_roadmap.md`. |
| Mirror granularity | Case by case. For example, one `pages/account/` covers login, the auth callback and settings; administration splits per sub-page. |
| Feature grouping | Everything about one feature lives together: behaviour, UX, design target, `visual_references/`, roadmap, research, evidence. All Mission Creator material sits under `website/frontend/apps/editor/`. |
| Page specs | `page.md`, the legacy page specs and the draft hubs merge into one feature doc per page area; `page.md` files go |
| Names | The editor is "Mission Creator". **Mission** everywhere; Enfusion's world + game-mode config is the **mission header**. **Event** means a session record; **operations** is the domain. Code identifiers are quoted as spelled. |
| Scenario→mission code rename | A separate program after this one, deferred by the operator. P0-4 files its ticket. |
| Processes | Factory waves, the mod slice workflow, the `ticket run` pipeline and Cursor all stay live. The factory docs become agent-neutral. |
| Law 2 | Gains one exception: `slice/<id>` branches that xtask slice and wave tooling creates and deletes itself |
| Doc ownership | The Cursor-only rule is retired. Docs ship in the same commit as the code they describe. |
| `ticket sync` | Drop its 6 generated views. Keep `queue.json` and the ROADMAP/gap-analysis marker injection. |
| Prose gates | Retire `specification_consistency` and `content_budgets` |
| New gates | README coverage with a "Contents matches folder" check, markdown placement with the size check, and a link checker. The Contents check is the only structural rule enforced. |
| Verification evidence | Moves now; the api-readiness register is re-pointed; one `--execute` run happens at the end |
| Build plan | Items are extracted into `product_roadmap.md`, and the operator strikes the unplanned ones at checkpoint 4. The original is archived. |
| Orphan specs | Linked to their tickets. Dead files and program records are archived. |
| Known bugs | A live registry at `known_bugs/` |
| Templates | Ticket templates live in `.ai/tickets/`, snake_case: `spec_template.md`, `plan_template.md`, `handoff_template.md`. Document templates live in `standards/templates/`. |
| Links | Repo-root style (`/documentation_v2/…`). Dead code links in frozen and archived docs become GitHub permalinks (`darkforce09/TBD-reforger`). |
| File names | snake_case, with these exceptions: `README.md`, `t-<id>_plan.md`, and the hyphenated evidence JSON |
| Header | Status line only (`live` / `frozen record` / `archived`), on documentation_v2 docs. Code READMEs carry none. |
| Diagrams | ASCII, in `text` blocks |
| Open work | Feature docs link open tickets in `## Open work`. READMEs never cite tickets. |
| Doc size | Live docs stay at or under 500 lines and are split by topic when longer. Frozen and archived docs are exempt. Sync-managed tables stay in one file. |
| Mission Creator corpus | The feature inventory (split), roadmap, UX decisions log (as `decisions.md`) and Eden gap analysis all stay live |
| Glossary | A top-level `glossary.md`; each doc links a term's first use |
| Audience | Developers and AI agents |
| Runbook checks | A static check against the xtask source, plus safe runs only (`--help`, `--dry-run`, local db and gate doctor) |
| Screenshots | None. Design references are the only images. |
| Server IP | Unknown. The docs name `TBD_SSH_HOST` and are checked live at checkpoint 2. |
| In-flight work | Never stage a foreign hunk; stage by hunk with `git apply --cached`. `apps/mod/tbd-export/resourceDatabase.rdb` is never staged. |
| Checkpoints | CP1 manifest, CP2 cutover, CP3a standard, CP3b pilot slices, CP4 roadmap |

## Target tree

```text
documentation_v2/
├── README.md              entry: map, authority ladder, where to find what
├── glossary.md            project terms and abbreviations
├── product_roadmap.md     operator-curated plan (checkpoint 4)
├── website/               mirrors apps/website/: api_v2/ (incl. verification/), frontend/ (pages/<area>/…,
│                          apps/editor/ = Mission Creator, apps/planner, apps/aar), map-engine/, graphics-engine/
├── mod/                   mirrors apps/mod/: tbd-framework/ (mod design, UI/<screen>/), tbd-export/ (+ evidence), tbd-emcp/
├── ticketboard/  fleet_host_agent/  tools_v2/<crate>/  contracts_v2/  assets_v2/
├── design_system/         tokens, typography, colour, symbology, token exports
├── runbooks/              every operator procedure
├── standards/             readme_standard, templates/, documentation, coding, where_does_x_go,
│                          commit_checklist, ticket_identifiers, engine_boundary_rules
├── known_bugs/            live registry
├── tickets/specs/  tickets/plans/   flat, frozen
└── archive/<topic>/       frozen history
```

## README and document standard (P3-1 designs it; CP3a reviews it)

**README core.** The sections come in this order:

1. H1: a human-readable name, with no path and no backticks.
2. A 1–3 sentence purpose.
3. `## Contents`: a `text` tree whose root line is the folder's repo path with a trailing `/`. It
   lists every tracked direct child (README.md excluded; `tests/` and `generated/` as one line each)
   with a one-line role. A homogeneous collection is one glob line.
4. `## How it works` (a leaf folder with ≤3 files may skip it).
5. The kind sections.
6. `## Boundaries`, with three bullets: Depends on / Used by / Rules.
7. `## Related documentation`: repo-root links, omitted when there are none.

Kind sections: area root → Getting started · crate/package/addon root → Getting started,
Configuration, Public surface · domain/subsystem → Public surface · leaf → none · page → Routes, Data,
States · app → Routes, Public surface · command-line → Commands · data (contracts, assets,
fixtures, migrations, seeds) → Format, Producers and consumers · mod scripts → Authority
(server/client/replicated) · mod assets → Format, Referenced by · deploy/config → Configuration,
Installed by · documentation_v2 folder → Code.

Headings are unnumbered with exact names. Soft length guide: leaf 10–60 lines, subsystem 40–200,
root ≤400; the hard cap is 500.

**Gate.** Every tracked child matches exactly one Contents line, by name or glob; every line matches
a child; every role is non-empty; the root line equals the folder path. No other structural checks.

**Other templates**, each in `standards/templates/`:

- **Feature doc:** Where it lives → Behaviour → Data → Design → Open work → Decisions.
- **Runbook:** Prerequisites → numbered Steps (one command per block, `Expected:`) → Verify →
  Troubleshooting (symptom | cause | fix) → Related.
- **`decisions.md` entry:** `### YYYY-MM-DD — decision`, then Context / Decision / Consequences /
  Supersedes.
- **Known bug:** `# KB-NNN — title`, then Status / Symptom / Cause / Workaround / Fix / Related tickets.
- **Glossary entry:** term, definition, `In code:`, `See:`.
- **Folder index:** the README core.

## Orchestrator protocol (main session)

- **Launch.** Every agent is started with `Agent(subagent_type="general-purpose", model="opus",
  run_in_background=true, description="<ID> <title>")`. Its prompt is the BASE BRIEF (inline until
  P0-1 commits `refactor_writing_brief.md`; after that, a pointer to the file), then the role
  template, then the slice parameters.
- **On completion:**
  - Read the report and skim the diff (`git diff --stat` and spot checks).
  - Stage only the agent's files. Dirty foreign files are staged hunk-wise.
  - Commit to `main`: Conventional Commits, `docs(<area>): …`, `refactor(<crate>): …` or
    `chore(docs): …`, with the attribution trailer.
- **Reviews and fixes.** The wave verifier's verdict is either PASS or a FIX-LIST. A FIX-LIST is
  handed to a fix agent using the same template, scoped to that list.
- **Concurrency and budgets.**
  - At most **4** writers run in parallel, always on disjoint files; drop to 3 if usage limits bite.
  - Each agent stops at about 450k tokens. An agent near its cap is stopped with TaskStop and its
    remainder relaunched narrower.
- **Owned by the main session:**
  - memory updates at phase ends and after 2a
  - relaying checkpoint questions to the operator
  - pausing the other session's `docs/` writes before Phase 2 (operator confirmation plus
    `git status -- docs` clean)
  - copying CLAUDE.md to AGENTS.md
- **Resume.** Any session resumes from `documentation_v2/refactor_progress_checkpoint.md`.

## BASE BRIEF (every agent reads this first; P0-1 stores it as `documentation_v2/refactor_writing_brief.md`)

```text
BASE BRIEF — Documentation V2 program.
Repo: <repo> = the checkout root, `git rev-parse --show-toplevel` (git, main). You run in a Debian container
(glibc 2.36); the host is Fedora (glibc 2.43). Scratchpad: <scratchpad> = the session scratchpad named in your launch prompt
(logs under <scratchpad>/logs/<your-id>/).
Read first: CLAUDE.md; documentation_v2/refactor_writing_brief.md (decisions, rules, contradiction
resolutions, terminology); from Phase 2 on documentation_v2/refactor_move_manifest.tsv; from Phase 3 on
documentation_v2/standards/readme_standard.md, standards/templates/, glossary.md.
RULES
1 Ownership: edit ONLY files under "YOU OWN". Anything else that needs changing goes in your report
  under "Outside my scope".
2 Git: read-only git only (log, show, ls-files, grep, diff, blame, cat-file, check-ignore) unless your
  task grants a verb (e.g. git mv). Never add/commit/checkout/reset/stash/restore/clean/branch/merge.
3 Foreign changes: the tree carries another session's uncommitted work. Before editing a file run
  `git diff --stat -- <file>`; if dirty, touch only your lines and list the file in your report. Never
  touch apps/mod/tbd-export/resourceDatabase.rdb.
4 Truth: verify every claim against code — paths via `git ls-files`; commands via the xtask CLI source
  (tools_v2/xtask/src/cli/, each group's cli.rs) plus safe `--help`/`--dry-run` runs; routes via
  apps/website/frontend/src/app_routes.rs and api_v2 src/<domain>/routes.rs; env vars via
  apps/website/api_v2/.env.example; symbols by reading the code; callers via `git grep`. Code beats docs;
  record each doc-vs-code contradiction (file:line both sides).
5 Style: present tense; no history words (formerly, previously, legacy, migrated, renamed from), no dates
  (except decisions entries and archive headers), no ticket IDs in READMEs; repo-root links
  ([x](/documentation_v2/…), [y](/apps/…)); ASCII diagrams in ```text; snake_case names (exceptions:
  README.md, t-<id>_plan.md, hyphenated evidence JSON); status line on documentation_v2 docs only;
  ≤500 lines per live doc (split into a folder with a README index); terminology: Mission Creator,
  mission, mission header, event, operations (glossary links on first use); never hard-code IPs (name
  TBD_SSH_HOST from tools_v2/xtask/deploy/deploy.env); no personal absolute paths.
6 Preservation: before merging, replacing or dropping text, list every fact of the source and give each
  a disposition — carried to <target>, archived, or dropped as wrong (cite the proving code). Frozen
  files (documentation_v2/tickets/**, documentation_v2/archive/**) are never reworded; only links change.
7 Commands: one command per Bash call; >30 s runs go to background with their own log, polled every 2 s;
  always `cd <dir> && <cmd>`; never pipe a test/gate through head/tail. Cargo: use
  $HOME/.cache/tbd-bin/hcargo for every cargo and xtask call (host toolchain, warm cache); if a
  run refuses on ABI grounds use CARGO_TARGET_DIR=<repo>/
  target-container-api-v2 for `xtask mk …` recipes. Database: `distrobox-host-exec podman start
  tbd_reforger_db` (compose is unavailable); db/ci-local runs need <scratchpad>/bin/podman containing
  `exec distrobox-host-exec podman "$@"` (chmod +x) first on PATH. EnfScript edits: added lines ASCII
  only (check `git diff -U0 -- <file> | grep -P '^\+.*[^\x00-\x7F]'` is empty); compile with
  `hcargo xtask mod compile`. tools_v2 Markdown: no T-### IDs, no history words (incl. "legacy"), no
  retired spellings (t159, tbd_tools), no .sh/.py/.mjs/.cjs names, every x.rs named must exist.
8 Never run remote, deploy, restore, drop or destructive commands; never print secrets.
9 Stop at ~450k tokens; report done/remaining.
10 Final message = REPORT (≤80 lines): Done · Checks run (command, exit) · Claims verified (count,
  notable) · Contradictions (file:line) · Dispositions (merges/drops) · Outside my scope · Questions
  for the operator.
```

## Roster summary

| Phase | Agents | Order |
|---|---|---|
| 0 Setup and manifest | 5 | P0-1 → (P0-2 ∥ P0-3 ∥ P0-4) → P0-5 → **CP1** |
| 1 Tooling before cutover | 6 | P1-1 → P1-2 → P1-3 → P1-4 → P1-5 → P1-6, one commit per agent |
| 2 Cutover | 7 | P2-1 → P2-2 → P2-3 → P2-4 → commit 2a → (P2-5 ∥ P2-6) → P2-7 → commit 2b → **CP2** |
| 3 Standards and pilot | 5 | P3-1 → **CP3a** → P3-2 → (R01 ∥ F01) → P3-3 → **CP3b** |
| 4 README rollout | 25 | 5 waves × (4 writers in parallel) + 1 verifier per wave |
| 5 Feature docs, runbooks, standards | 24 | 5 waves (4, 4, 4, 4, 2 writers) + 1 verifier per wave; **CP4**; F19b |
| 6 Agent instruction files | 3 | (G1 ∥ G2) → G3 |
| 7 Gates on, close-out | 2 | H1 → H2 |
| **Total** | **77** | fix agents are extra, as verifiers require |

## Phase 0: Setup and manifest (no moves)

**P0-1 Program files and baseline** (~150k tokens)

```text
[BASE BRIEF inline]
ROLE P0-1. YOU OWN: documentation_v2/refactor_program_plan.md, refactor_writing_brief.md,
refactor_progress_checkpoint.md (new).
1 Copy the approved plan ($HOME/.claude/plans/i-want-you-to-cosmic-blanket.md) verbatim into refactor_program_plan.md
  (status line "live — program plan").
2 refactor_writing_brief.md: the BASE BRIEF text, the operator-decisions table, the README/document
  standard section, and the contradiction resolutions: /healthz; SPA :3000, API :8080, Postgres :5434
  local; Law 2 + tooling exception; allowlist retired; `cargo xtask ticket …` (no ./scripts); ticket
  statuses incl. running/review; T-092 shipped; engine split done; CLAUDE.md has no §Status
  (SHIPPED_HISTORY archived); frontend code under src/v2; deploy files under tools_v2/xtask/deploy/;
  host = TBD_SSH_HOST. Split into a folder if >500 lines.
3 Baseline, one command per call, logged: hcargo xtask ticket check --strict; hcargo test -p xtask;
  -p ticket-engine; -p developer-tools; -p ticketboard; -p verification-core; hcargo xtask ci
  verify-coding-standards; hcargo xtask mod compile. Expected red: tooling_prose_rules —
  only_a_layout_module_spells_a_repository_path (api_readiness/{fingerprint,register}.rs literals),
  nothing_narrates_its_own_history (equipment_vehicle_export/legacy_archive.rs — other session),
  every_rust_file_named_in_prose_exists (agent.rs in tools_v2/PHASE_FIVE_HANDOFF.md). Quote each
  failure; flag anything else red.
4 refactor_progress_checkpoint.md: baseline table (command, exit, log, excerpt) + roster table (id,
  phase, status, commit, open questions) prefilled with all 77 agent ids.
```

**P0-2 Manifest, mechanical rows** (~250k, parallel with P0-3 and P0-4)

```text
[BASE BRIEF] ROLE P0-2. YOU OWN: <scratchpad>/manifest/build_mechanical.py, mechanical.tsv.
Emit TSV rows `source<TAB>target<TAB>action<TAB>class<TAB>writer<TAB>note` (empty note = "-"; no
trailing whitespace; final newline) — action ∈ move|move+rename|merge-into|archive|collapse-duplicate|
delete; class ∈ frozen|live-rewrite|archive|evidence|data; writer = the Phase-4/5 id that rewrites it.
Rules:
- Ticket specs docs/specs/**/t<digit>*.md, docs/platform/t<digit>*.md, docs/platform/audit/t*.md,
  docs/mod/t181_event_mod_program.md → documentation_v2/tickets/specs/<basename> (move, frozen). Re-run
  the basename collision audit; fail loudly on duplicates.
- Cited non-ticket specs: docs/specs/website_reorg/plan.md → tickets/specs/t934_website_reorganisation_
  program.md; docs/platform/tbd_north_star_backlog.md → t131_north_star_backlog.md; .ai/artifacts/
  missed_items_handoff.md → t166_missed_items_handoff.md; .ai/artifacts/t068_10_5_weapon_families.md and
  t159_15_render_loop_handoff.md → tickets/specs/ same name (move+rename / move, frozen).
- docs/plans/t-*_plan.md → documentation_v2/tickets/plans/ (frozen); docs/plans/TEMPLATE.md →
  .ai/tickets/plan_template.md (live-rewrite, writer G2).
- Visual references (docs/specs/macOS_Blueprints/**, docs/specs/Mission_Creator_Mock_Up/**,
  docs/mod/ui/** png/html and ui_stitch_mockup/**, documentation_v2/**/visual_reference/**) → the
  feature folder's visual_references/<set_name>/ with files renamed <set_name>.html / .png (sha256
  duplicates → collapse-duplicate keeping the docs/ copy); the 33 boilerplate visual_reference READMEs
  (identical sha 8f633b19…) → delete. Use documentation_v2/ANALYSIS_AND_INVENTORY.md §3 for website sets;
  mod UI sets → mod/tbd-framework/UI/<screen>/visual_references/<set>/.
- Program records → archive (frozen): apps/website/api_v2/{ARCHITECTURE_PLAN,ANALYSIS_AND_INVENTORY,
  PHASE_1..7_HANDOFF}.md → archive/api_v2_refactor/<snake>; contracts_v2/{3} →
  archive/contracts_v2_relocation/; assets_v2/{3} → archive/assets_v2_relocation/;
  apps/website/frontend/src/v2/MIGRATION.md → archive/frontend_v2_migration/; the 7 tools_v2 records are
  archived by P1-1 (row: action "done-by-P1-1").
- 22 redirect stubs, docs/specs/Mission_Creator_Mock_Up/Mission ccreator/README.md, the
  09_eden_wiki_manifest.yaml stub → archive/redirect_stubs/ (note = real target, used to retarget links);
  docs/website/.doc-manifest.{before,after}, docs/website/REORG_CHANGELOG.md → archive/monorepo_migration/;
  eden/wiki_manifest.yaml → archive/redirect_stubs/.
- docs/verification/api_v2/** → documentation_v2/website/api_v2/verification/ (evidence);
  docs/verification/equipment-vehicle-export/** → documentation_v2/mod/tbd-export/<mirror of the
  exporter's code folder>/verification/ (evidence).
- docs/platform/factory_pack_wave → .ai/factory_pack_wave (data); docs/mod/capability_verdicts.tsv →
  documentation_v2/mod/tbd-framework/capability_verdicts.tsv (data).
- docs/TICKET_*.md, docs/MILESTONES.md → "done-by-P1-2".
Report counts per action/class and every file under docs/, documentation_v2/, or non-README .md in code
trees that no rule covered (P0-3 takes those).
```

**P0-3 Manifest, judgment rows** (~300k, parallel)

```text
[BASE BRIEF] ROLE P0-3. YOU OWN: <scratchpad>/manifest/judgment.tsv, judgment_notes.md.
INPUT: <scratchpad>/manifest/explorer_classification.md (per-file class table from the audit).
One row (same columns as P0-2) for every file the mechanical rules do not cover (≈250): docs/website/**,
docs/mod/*.md + ui/**/*.md, docs/platform non-ticket docs + known-bugs/, docs/tools/, docs/specs
non-ticket docs (Mission_Creator_Architecture ROADMAP/agent_execution/engineering_plan/feature_inventory/
problem_statement/ux_spec/reference/eden/*, audit_2026_09 README + false_claims, website_reorg audit +
mission-core blueprint, macOS_UX stub), documentation_v2 hub .md, the 19 page.md,
pages/navigation/{nav_config,layout}.md, contracts_v2/definitions/bridge-messages.md.
Targets follow the target tree and decisions: live docs → their mirror/runbooks/standards/design_system/
known_bugs/product_roadmap home (snake_case); history → archive/<topic> (api_v2_refactor,
contracts_v2_relocation, assets_v2_relocation, frontend_v2_migration, tools_v2_refactor, engine_split,
monorepo_migration, audits, factory_runs, handoffs_and_kickoffs, go_and_react_era_design, product_plans,
redirect_stubs, documentation_v2_refactor). Merges: the primary source moves to the target path; every
secondary source moves to documentation_v2/pending_merge/<writer-id>/<original name> (action
merge-into, note = final target). Writer ids come from the plan's P3-1/P3-2, R01–R21, F01–F19 and
G1/G2 tables (DOCUMENTATION_STANDARDS and _doc_header → P3-2; TAGS → F17). Mission
Creator material all under website/frontend/apps/editor/; factory docs → runbooks/factory_waves/ (dated
parts archived); ENGINE_SPLIT_PROGRAM → archive/engine_split/ with its rules extracted by F09 into
standards/engine_boundary_rules.md. judgment_notes.md: one line per non-obvious call; uncertain calls as
questions for CP1.
```

**P0-4 Ticket plan and follow-up tickets** (~200k, parallel)

```text
[BASE BRIEF] ROLE P0-4. YOU OWN: <scratchpad>/manifest/ticket_rewrites.tsv; three new ticket files
created ONLY via `hcargo xtask ticket` verbs (find the verb with `hcargo xtask ticket --help`; git
grants: none); the files those verbs regenerate through `ticket sync` — docs/TICKET_*.md,
docs/MILESTONES.md, .ai/tickets/queue.json, the ROADMAP marker block and the gap-analysis ticket
column — and nothing else in them.
1 Every string in .ai/tickets/*.toml naming docs/…, the 3 cited .ai/artifacts spec files, or a program
  record: rows ticket, field (spec|plan|owns|citations|other field name), old string. Owns: map only
  docs/ prefixes; directory owns (docs/mod T-274, docs/plans T-917.6, docs/specs/Mission_Creator_
  Architecture T-612) get a proposed new value and an overlap note.
2 Orphan specs: set `spec` on T-068.0.1, .1, .2, .3, .4, .5, .5.1, .6, .7, .8, .13, .14 (they have
  none); for t068_asset_registry, t090_1_2_satellite_backlog, the 8 t090_* design specs and
  t180_class_r_pins, add the spec to `citations` of the best-matching ticket (child by topic, else the
  program ticket); t060_1/t062_1/t062_1_1/t062_2 → citations on T-060 / T-062. Record decisions only;
  P2-2 applies them.
3 File (status queued, one-paragraph summary each): scenario→mission code rename program (identifiers,
  UI strings, /fleet/scenarios routes, contract fields, a DB rename migration, mod classes, fleet agent;
  engine-owned names excepted: server-config scenarioId, SCR_EScenario*); mod wave gate calls four
  `make` targets with no tracked Makefile (tools_v2/xtask/src/commands/mod_ops/wave_execution/
  execution.rs:354-363); remove 45 stale local branches and 5 worktrees under .ai/artifacts/worktrees/.
```

**P0-5 Manifest assembly and pin catalogue** (~250k, after P0-2 through P0-4)

```text
[BASE BRIEF] ROLE P0-5. YOU OWN: documentation_v2/refactor_move_manifest.tsv, refactor_move_manifest.md,
refactor_ticket_rewrites.tsv, refactor_pin_catalogue.md.
1 Merge mechanical.tsv + judgment.tsv. Assert: every tracked file under docs/ and documentation_v2/
  (refactor_* excepted), every non-README .md in code trees, and the 3 artifact specs appear exactly
  once; no duplicate targets except merge-into; snake_case targets (listed exceptions); no target under
  docs/; no trailing whitespace; the file never spells the retired wave-plan fossil strings (the
  needles ticket-engine's fossil guard greps for; see ARCHIVED_WAVE_PLANS). Add rows moving documentation_v2/{ARCHITECTURE_PLAN,
  ANALYSIS_AND_INVENTORY}.md → archive/documentation_v2_refactor/.
2 Fill ticket_rewrites targets from the manifest → refactor_ticket_rewrites.tsv.
3 Pin catalogue: `git grep -n` every old path of the manifest across tools_v2/, apps/, .github/,
  .editorconfig-checker.json, .gitignore, CLAUDE.md, .cursor/, apps/mod/.cursor/, .ai/tickets/,
  contracts_v2/, assets_v2/, seeds, .env.example — plus the plan's Code-pins list. Table: file:line,
  current, new, owner agent (P1-x/P2-x/G1/G2/H1), kind (code|test|config|comment|prose). Mark
  must-not-change historical spellings (ARCHIVED_WAVE_PLANS, the retired queue-view numstat prefix).
4 refactor_move_manifest.md (≤500 lines, split if needed): counts by action/class/target folder; each
  live target with its sources and writer; archive topics with file lists; every P0-3 question.
```

**CP1.** The operator reviews `refactor_move_manifest.md` and its questions. P0-5 applies the answers
through a fix-agent run.

## Phase 1: Tooling before the cutover (sequential; one green commit per agent)

**P1-1 Archive the tools_v2 program records** (~60k)

```text
[BASE BRIEF] ROLE P1-1. GRANT: git mv. YOU OWN: tools_v2/{ARCHITECTURE_PLAN,ANALYSIS_AND_INVENTORY,
PHASE_ONE..FIVE_HANDOFF}.md → documentation_v2/archive/tools_v2_refactor/<snake_case>.md;
tools_v2/ticket-engine/README.md (line 23 link only).
git mv each file; fix the inbound link. Run hcargo test -p xtask tooling_prose_rules and report which of
the three baseline failures remain (every_rust_file_named_in_prose_exists must now pass).
```

**P1-2 Retire the generated ticket views** (~200k)

```text
[BASE BRIEF] ROLE P1-2. YOU OWN: tools_v2/ticket-engine/src/{sync/runner.rs, sync/mod.rs,
sync/queue_views.rs (delete), sync/registry_views.rs (delete), repository.rs, sync/README.md, README.md,
cli/tests/command_mutation_tests.rs}; apps/ticketboard/src/repository_status/{services/file_watch.rs,
services/tests/file_watch.rs, models/git_status.rs, models/tests/git_status.rs}; docs/TICKET_*.md and
docs/MILESTONES.md (delete); link lines at README.md:34, apps/website/README.md:37,
.ai/tickets/AI_PLAYBOOK.md:57,74,98-103, .ai/tickets/README.md:57, .ai/tickets/SPEC_TEMPLATE.md:6.
- runner.rs: drop create_dir_all(TREE_DIR) and the 6 view writes (:17-42); keep queue.json (:44-45),
  the ROADMAP marker (:47-53) and the gap-analysis column (:55-57).
- repository.rs: delete the 5 TICKET_*_VIEW constants, MILESTONES, MOD_MILESTONES (:186),
  GENERATED_QUEUE_VIEWS (:190) and the ARCHIVED_WAVE_PLAN_READERS row keyed on the view prefix
  (:248-251). Keep the "docs/TICKET_" prefix (it matches historical numstat) renamed to
  RETIRED_QUEUE_VIEW_PREFIX with a present-tense doc comment; NUMSTAT_EXCLUDED_PREFIXES keeps it.
- Delete the assertion at command_mutation_tests.rs:408-416.
- ticketboard: remove the view filter (file_watch.rs :13, :111-113, :137-139; the watch list at
  :177-182 goes from 3 to 2); git_status.rs GIT_ARGS swaps TREE_DIR for ROADMAP + GAP_ANALYSIS;
  update both test files.
- Inbound links: replace view links with a one-line pointer to apps/ticketboard.
Checks: hcargo test -p ticket-engine; hcargo test -p ticketboard; hcargo clippy -p ticket-engine -p
ticketboard --all-targets -- -D warnings; hcargo xtask ticket check --strict.
```

**P1-3 Retire the prose gates** (~200k)

```text
[BASE BRIEF] ROLE P1-3. YOU OWN (tools_v2/xtask/src/ unless stated): verifications/schemas/checks/
{specification_consistency.rs, content_budgets.rs} (delete); verifications/schemas/checks.rs (:1-3,
:70, :115-127 incl. ARCHIVAL_MAKE_TARGETS, :290-292, :300-301, :321); checks/read_json.rs (:12-14
spec_dir); commands/schema/{cli.rs :18-24, dispatch.rs :17-21}; commands/ci/task_definitions.rs
(:17-20, :73, :91-97); commands/platform/wave_execution/schema.rs (:22-25 doc, :36-46
VALIDATE_GATES); commands/ci/tests/task_runner.rs (:111-140); commands/platform/wave_execution/tests/
schema/tests.rs (:43-47); core/repository_layout.rs (:62-75: SPECIFICATION_DOCS_DIR, FRONTEND_ROADMAP,
FRONTEND_INDEX, MISSION_EDITOR_SURFACE, MOD_AGENT_START); commands/db/operations.rs (:135-139 comment);
verifications/schemas/README.md:3; tools_v2/xtask/README.md:12; .github/workflows/ci.yml:164-169
(prose); tools_v2/ticket-engine/src/corpus_pins.rs (:26-27 map_terrain_programme_ticket) +
src/tests/corpus_pins_tests.rs (:16,:40,:67) + .ai/tickets/corpus-pins.toml (its key).
Remove the two gates and every orphan (clippy -D warnings treats dead code as an error). Both
tripwire lists change in the same edit.
Checks: hcargo test -p xtask; hcargo test -p ticket-engine; hcargo clippy -p xtask -p developer-tools
--all-targets -- -D warnings; hcargo xtask ci ci-local-schema (expect green).
```

**P1-4 Api-readiness constants and layout existence tests** (~150k)

```text
[BASE BRIEF] ROLE P1-4. YOU OWN: tools_v2/xtask/src/core/repository_layout.rs (+ its sibling tests
file), tools_v2/xtask/src/verifications/api_readiness/{fingerprint.rs, register.rs},
tools_v2/ticket-engine/src/repository.rs (tests only) + tools_v2/ticket-engine/src/tests/
repository_layout_tests.rs, tools_v2/developer-tools/src/repository_layout.rs tests +
src/tests/repository_layout.rs.
- Add API_READINESS_EVIDENCE_PREFIX = "docs/verification/api_v2/" (trailing slash; starts_with
  matching at fingerprint.rs:97) and API_READINESS_REGISTER to the xtask documentation module; use
  them in fingerprint.rs:17 and register.rs:10; add a test that the register starts with the prefix.
- Add existence tests for every documentation constant in the three layout modules (each named file
  or dir exists), so the cutover cannot silently skip a file.
Checks: hcargo test -p xtask (only_a_layout_module_spells_a_repository_path must pass); -p
ticket-engine; -p developer-tools; hcargo xtask verify api-readiness (without --execute: reports
stale/unavailable evidence, never a register-read error).
```

**P1-5 Documentation gate modules and verbs** (~400k)

```text
[BASE BRIEF] ROLE P1-5. YOU OWN: tools_v2/xtask/src/verifications/documentation/** (new, incl.
README.md files), tools_v2/xtask/src/verifications/mod.rs (registration line),
tools_v2/xtask/src/commands/verify/{cli.rs, dispatch.rs}, tools_v2/xtask/src/core/repository_layout.rs
(new constants + tests).
Build (no new crates; every file <500 LOC; tests in sibling tests/ files <1000 LOC):
  tracked_tree.rs (git ls-files -z via verification_core::proc::Run → files + dirs; git missing or
  failing → NotRun), readme_coverage.rs, markdown_placement.rs, link_check.rs +
  link_check/{markdown_scan.rs, heading_anchors.rs (GitHub slug rules, duplicates -1/-2, setext,
  <a id|name>), target_resolution.rs (repo-root "/" or relative, percent-decoding, <…> destinations,
  folder targets, #L10-L20 on code files, own-repo permalinks checked with git cat-file -e sha:path),
  backticked_paths.rs (live docs only; first segment must be an existing top-level folder; skip globs,
  <…>, {…}, $VAR, trailing :line, commands; gitignored runtime paths accepted via git check-ignore
  --stdin; exemption list constant for historical spellings)}.
Behaviour:
- readme-coverage: every tracked dir under CODE_TREES has README.md, except dirs under tests/,
  generated/ or a dot-dir; every README (code trees + documentation_v2) passes "Contents matches
  folder": the first ```text block under "## Contents" has root line == the folder's repo path +
  "/", every tracked direct child except README.md matches exactly one line (name or glob), every line
  matches ≥1 child, every role is non-empty.
- markdown-placement: code trees hold only README.md; the retired docs root does not exist; live
  documentation_v2 docs are ≤500 lines (tickets/, archive/ exempt).
- link-check: every markdown/.mdc file in documentation_v2/, all README.md, root README.md,
  CLAUDE.md, .ai/tickets/*.md, .cursor/**/*.mdc, apps/mod/.cursor/**; frozen areas (tickets/,
  archive/) are checked for links only; `--report` prints every break with file:line.
- Every verb takes an optional repeatable `--path <dir>` scope.
Paths come ONLY from layout constants: add CODE_TREES = [apps, tools_v2, contracts_v2, assets_v2],
ARCHIVE_DIR, CURSOR_RULE_DIRS, RETIRED_DOCS_ROOT, PERMALINK_BASE = https://github.com/darkforce09/
TBD-reforger/blob/; re-export ticket_engine::repository::documentation::{TREE_DIR, SPECS_DIR,
PLANS_DIR} and TICKETS_DIR. Verbs: `cargo xtask verify readme-coverage | markdown-placement |
link-check [--report]`, following the EngineLayers pattern (dispatch.rs:90-94). NOT wired into ci-local
or ci.yml.
Checks: hcargo test -p xtask; clippy -D warnings; run each verb once against the tree and put the
totals in the report (failures are expected before the cutover).
```

**P1-6 Phase 1 verifier** (~200k)

```text
[BASE BRIEF] ROLE P1-6 (read-only except documentation_v2/refactor_progress_checkpoint.md).
Run: hcargo test -p ticket-engine / xtask / developer-tools / ticketboard / verification-core; hcargo
clippy -p xtask -p developer-tools -p ticket-engine -p ticketboard --all-targets -- -D warnings; hcargo
xtask ticket check --strict; hcargo xtask ci verify-coding-standards; hcargo xtask ci ci-local-schema.
Audit the Phase 1 diffs: Law 7 (sizes, sibling tests, no inline test modules), present-tense comments,
no string-literal paths outside the layout modules, nothing unused. Update the checkpoint rows.
VERDICT: PASS or a FIX-LIST. Name any remaining red test and whose it is (the history-word test belongs
to the other session's equipment export code).
```

## Phase 2: Cutover (2a is pure moves plus pins; 2b is link rewrites)

Before starting, the orchestrator gets operator confirmation that no other session is writing under
`docs/` (the api_v2 completion session is paused or committed) and that `git status -- docs` is clean.

**P2-1 Mover (commit 2a)** (~200k)

```text
[BASE BRIEF] ROLE P2-1. GRANT: git mv, git rm (only for manifest rows with action delete or
collapse-duplicate). YOU OWN: every source and target path in refactor_move_manifest.tsv.
Apply every row EXACTLY: git mv for move/move+rename/archive/merge-into (secondary sources →
documentation_v2/pending_merge/<writer>/), git rm for delete/collapse-duplicate (verify sha256 equality
before each collapse). No content edits at all (keeps rename detection exact). Afterwards: no file left
under docs/ (remove the empty dir); list any row that failed.
Check: every manifest target exists; `git status --porcelain` shows only renames/deletes (R/D).
```

**P2-2 Code and config pins (commit 2a)** (~350k, after P2-1)

```text
[BASE BRIEF] ROLE P2-2. YOU OWN: every row of refactor_pin_catalogue.md whose owner is P2-2 (code,
tests, config, comments; not .md/.mdc prose). Includes: the three `documentation` modules (final
paths; ARCHIVED_WAVE_PLANS and RETIRED_QUEUE_VIEW_PREFIX unchanged; ARCHIVED_WAVE_PLAN_READERS
re-pointed; stale-id scan roots and exemptions gain archive/ and tickets/ and the new visual_references
locations; the REORG_CHANGELOG exemption becomes its archived path; PLAN_TEMPLATE →
.ai/tickets/plan_template.md; SPARSE_CHECKOUT_SETS website/mod sets gain their documentation_v2
mirror folders; API_READINESS_* → documentation_v2/website/api_v2/verification/; FACTORY_PACK_WAVE →
.ai/factory_pack_wave; enf MOD_DOCS_DIR/capability verdicts/editor-gate runbook → new paths);
apps/website/frontend/src/v2/apps/editor/arsenal/tests/shell_wiring.rs:542 include_str!; ticketboard
file_watch ROADMAP path + tests (:134,:138); ticket-engine/xtask/developer-tools/ticketboard fixtures
listed in the catalogue; tools_v2/developer-tools/gate-env.json:6;
tools_v2/xtask/deploy/systemd/tbd-website-api.service:35; tools_v2/xtask/deploy/Caddyfile.website:7;
.editorconfig-checker.json:13-14; .gitignore:3,24 (dirty: hunk only); .github/workflows/{ci.yml:3,
contracts.yml:3} comments; contract_citations.rs:165-168 text; .ai/tickets/estimates.schema.json:4;
.ai/tickets/{corpus-pins.toml:24, scope-vocab.toml:2} comments; apps/website/api_v2/.env.example:26,85
(dirty: hunk only); apps/website/api_v2/seeds/discord_roles.sql:20; api_v2 + fleet agent comments
naming docs/verification; developer-tools comments; xtask printed texts (runbook paths; engine-layer
messages at task_definitions.rs:44,300, commands/verify/cli.rs, engine_layer_boundaries.rs,
engine_layer_rules.rs, ci.yml:259 → /documentation_v2/standards/engine_boundary_rules.md;
verify_crf_leak.rs:38-42 section names); the EnfScript comments (added lines ASCII only).
Checks, one per call: hcargo build -p xtask; hcargo test -p ticket-engine / xtask / developer-tools /
ticketboard / verification-core; clippy -D warnings (xtask, developer-tools); hcargo xtask mk
ci-local-leptos; hcargo xtask mod compile; hcargo xtask verify api-readiness (register resolves).
```

**P2-3 Tickets (commit 2a)** (~250k, after P2-2)

```text
[BASE BRIEF] ROLE P2-3. YOU OWN: .ai/tickets/*.toml, .ai/tickets/wave.lock, .ai/tickets/queue.json.
Apply refactor_ticket_rewrites.tsv (spec, plan, owns docs/-prefixes only, citations, other fields) and
the P0-4 orphan-spec decisions. Then, one per call: hcargo build -p xtask; save `[[waves]]` of wave.lock;
hcargo xtask wave repack; diff the waves (report any regrouping of open waves); hcargo xtask ticket
sync (must write no views; queue.json + ROADMAP marker + gap-analysis column at the new paths); hcargo
xtask ticket check --strict (zero missing spec/plan).
```

**P2-4 Verifier for 2a** (~200k)

```text
[BASE BRIEF] ROLE P2-4 (read-only + checkpoint file). Run all P2-2/P2-3 checks again plus hcargo xtask
verify markdown-placement. `git grep -nE '(^|[^_a-z])docs/'` excluding documentation_v2/tickets/,
documentation_v2/archive/, .ai/artifacts/, and the synthetic test fixtures (ticketboard
document_loading tests, ticket-engine cli/tests + status_and_shipping_tests, tooling_prose_rules.rs)
→ only historical constants remain. Confirm rename detection: `git diff --cached -M --stat` shows
renames, not delete+add. VERDICT: PASS or a FIX-LIST.
```

After a PASS, the orchestrator commits 2a, updates memory (`MEMORY.md:14`,
`api-v2-completion-program.md`, `feedback-api-v2-completion-subagent-slices.md`) and tells the other
session its new resume path.

**P2-5 Link rewriter, frozen corpus (commit 2b)** (~350k, parallel with P2-6)

```text
[BASE BRIEF] ROLE P2-5. YOU OWN: documentation_v2/tickets/**, documentation_v2/archive/**.
Script (scratchpad) driven by the manifest: rewrite every markdown link whose target was moved →
repo-root form of the final target (merge sources → their merge target); drop anchors that no longer
exist; dead code links → PERMALINK_BASE/<commit>/<path> where <commit> = parent of the commit that
deleted the path (`git log -1 --diff-filter=D --format=%H -- <path>` then ^); add the first-line status
line ("frozen record" for tickets/**, "archived — see <live replacement>" for archive/**). No other text
changes. Check: hcargo xtask verify link-check --path documentation_v2/tickets --path
documentation_v2/archive → zero breaks.
```

**P2-6 Link rewriter, live corpus (commit 2b)** (~350k, parallel)

```text
[BASE BRIEF] ROLE P2-6. YOU OWN: every .md/.mdc outside documentation_v2/{tickets,archive}/: the rest
of documentation_v2/** (incl. pending_merge/), every in-code README.md, root README.md, CLAUDE.md
(dirty: your lines only), .ai/tickets/*.md, .cursor/**, apps/mod/.cursor/**.
Rewrite moved-doc links and backticked docs/ path mentions to repo-root form of the final target;
status line "live" on live documentation_v2 docs. Leave apps/ticketboard module README `## Files`
relative links untouched (its architecture test requires them; R21 migrates them). Do not rewrite
anything else (content is Phase 3–6 work). Check: link-check --report over your scope — every
remaining break is content-level; list them by writer id for Phases 4–6.
```

**P2-7 Verifier for 2b** (~200k)

```text
[BASE BRIEF] ROLE P2-7. Frozen corpus: link-check zero; diff shows only link/status-line changes in
frozen files (sample 50). Live corpus: the break list is attached per writer. hcargo xtask ticket check
--strict; hcargo test -p ticketboard. Update the checkpoint. VERDICT: PASS or a FIX-LIST.
```

**CP2.** The operator reviews both commits, opens ticketboard to check the spec reader, and confirms
the server address (both recorded LAN addresses carry the same SSH host key, so the host is identified by key).

## Phase 3: Standards and pilot

**P3-1 README standard and templates** (~400k)

```text
[BASE BRIEF] ROLE P3-1. YOU OWN: documentation_v2/standards/readme_standard.md (folder if >500 lines),
documentation_v2/standards/templates/** (readme_<kind>.md for the 12 kinds, feature_doc.md,
runbook.md, decisions_entry.md, known_bug.md, glossary_entry.md, folder_index.md), their folder
README.md files.
Write the standard exactly as decided (core order, kind table, rules, gate semantics — read
tools_v2/xtask/src/verifications/documentation/readme_coverage.rs and keep the machine-checked Contents
format identical). Each template = skeleton + one worked sample written from a REAL folder of that
kind (verified against code): e.g. api_v2/src/missions (domain), map-engine/src/spatial (leaf),
frontend pages/operations/schedule (page), apps/editor (app), developer-tools/src/bin (command-line),
contracts_v2/fixtures/missions (data), tbd-framework Systems/AI (mod scripts), tbd-framework Prefabs
(mod assets), xtask/deploy (deploy/config), apps/website (area root), apps/website/api_v2 (crate root),
a documentation_v2 folder (folder index). Also state how apps/ticketboard's README test will be
aligned (R21 edits it). Check: readme-coverage + link-check on standards/.
```

**CP3a.** The operator reviews the standard and its samples. Changes go through a P3-1 fix run.

**P3-2 Documentation standards, glossary skeleton and entry README** (~300k)

```text
[BASE BRIEF] ROLE P3-2. YOU OWN: documentation_v2/standards/documentation_standards.md (+ folder
split), documentation_v2/glossary.md, documentation_v2/README.md, and the pending_merge/P3-2/ sources
(DOCUMENTATION_STANDARDS, _doc_header).
documentation_standards: Rust (rustdoc, @route, @contract) and Enfusion (//! banners, @contract, @route,
@authority/@rpc/@replicated) comment rules verified against current code; Go/TS removed; layout rules
(target tree, README rule, feature grouping, mirror naming, snake_case + exceptions, status line, size,
links, diagrams, open work, decisions.md, spec lifecycle, templates, the three gates, same-commit
rule). Glossary: every term the pilot slices need, verified; anchors stable. Entry README: map,
authority ladder (code → CLAUDE.md → this README → standards → feature docs → frozen specs → archive).
Content preservation table for the merged sources.
```

**R01 and F01, pilot writers (parallel):** R01 uses the README writer template (Phase 4) and F01 the
doc writer template (Phase 5), with their slice rows.

**P3-3 Pilot verifier:** the wave verifier template over R01 and F01. **CP3b.** The operator reviews
the pilot output, and the style locks.

## Phase 4: README rollout (writers in parallel, 4 per wave; one verifier per wave)

**README writer template (R01–R21)**

```text
[BASE BRIEF] ROLE README writer {ID}. YOU OWN: README.md in every folder of SLICE (create or rewrite)
+ EXTRA OWNERSHIP. SLICE: {scope}. EXTRA: {constraints/checks}.
Read first: standards/readme_standard.md, standards/templates/readme_<kind>.md, glossary.md, and the
feature docs covering this code. Link only to targets that exist now; Phase 5 writers add the
remaining backlinks.
Per folder, deepest first:
1 `git ls-files <folder>` → tracked direct children; pick the kind.
2 Read enough code to be exact: //! docs, pub items (`git grep -nE 'pub (fn|struct|enum|trait|const|
  mod)'`), mod.rs/lib.rs wiring, imports (Depends on), callers via git grep (Used by); EnfScript: class
  names, [Attribute], RPC/replication (Authority); data folders: the reader/schema that consumes them.
3 Write from the kind template; Contents lists every child with a real role (globs for collections);
  parents summarise children in one line each and never repeat child detail.
4 Existing README: rewrite; carry every true fact (disposition table); drop boilerplate, bare file
  lists, history, wrong claims (cite code).
Checks: hcargo xtask verify readme-coverage --path <each slice root>; hcargo xtask verify link-check
--path <each slice root>; EXTRA checks. Report adds a table: folder, kind, new|rewritten, lines.
```

| ID | Scope (recursive) | Extra |
|---|---|---|
| R01 (pilot) | `apps/website/api_v2`: crate root, `migrations/`, `seeds/`, `src/` root, `src/bin`, `src/{core, background_workers, identity_and_access, administration, community_content, match_telemetry, command_center, server_infrastructure}` | `hcargo test -p website-api architecture_rules` |
| R02 | `apps/website/api_v2/src/{missions, operations}` | as R01 |
| R03 | frontend crate root and non-src folders, `src/` root, `src/v2/` root, `src/v2/core/**` | |
| R04 | `src/v2/pages/` root, `pages/{command_center, operations, mission_hub}/**` | |
| R05 | `pages/{administration, doctrine_and_info, field_tools, account, navigation}/**` | |
| R06 | `src/v2/apps/editor/ui/**` | |
| R07 | `src/v2/apps/` root, `editor/` root and every editor child except `ui/`, `apps/{debug, planner, aar}/**` | |
| R08 | `apps/website/map-engine/src/data/**` | |
| R09 | map-engine `src/{world, streaming, spatial}/**` | |
| R10 | map-engine crate root, `src/` root and every other `src/` folder (editing, overlay, diagnostics, frame, io, doll, camera, shaders, …) | |
| R11 | `apps/README.md` (new), `apps/website/` root, `graphics-engine/**`, `apps/website/shared`, `contracts_v2/**`, `assets_v2/**` | Run the tests that enumerate any fixture folder touched (find them with `git grep` on the folder name) |
| R12 | tbd-framework `Scripts/Game/TBD/Systems/**` | `hcargo xtask mod compile`; check `git status` for `.meta` or rdb churn |
| R13 | tbd-framework `Scripts/Game/TBD/{Session, API, Core}/**` | as R12 |
| R14 | `apps/mod/` root, tbd-framework root and the `Scripts/`, `Game/` and `TBD/` root levels, `{Gamemode, UI}/**` scripts, the asset folders `Configs, Data, Missions, Prefabs, UI, worlds/**`, `tbd-emcp/**` | as R12, plus a Workbench churn report |
| R15 | `apps/mod/tbd-export/**` | as R12 |
| R16 | xtask `src/commands/{platform, mod_ops}/**` | `hcargo test -p xtask tooling_prose_rules` |
| R17 | xtask crate root, `deploy/**`, `dedicated_server_profiles/**`, `fixtures/**`, `src/` root, `src/{core, cli}/**`, the `src/commands/` root and every command group except platform and mod_ops | as R16, plus `hcargo xtask mcp selftest` |
| R18 | xtask `src/verifications/**` | as R16 |
| R19 | developer-tools `src/{browser_testing, blueprint}/**` | as R16 |
| R20 | developer-tools crate root, `src/` root and the remaining `src/**`, `src/bin`, `fixtures/**`, `test_fixtures/**` | as R16, plus `hcargo test -p developer-tools` |
| R21 | `tools_v2/` root, `ticket-engine/**`, `verification-core/**`, `enfusion_mcp_node_package/`, `apps/ticketboard/**`, `apps/fleet_host_agent/**` | Extra ownership: `apps/ticketboard/src/tests/architecture_rules.rs` (align its README rule with the standard). Run `hcargo test -p ticketboard`, `-p ticket-engine` and the prose rules. |

**Waves:** W4.1 = R02–R05, W4.2 = R06–R09, W4.3 = R10–R13, W4.4 = R14–R17, W4.5 = R18–R21.
Each wave ends with its verifier, V4.1–V4.5.

**Wave verifier template (V*, P3-3)** (~250k)

```text
[BASE BRIEF] ROLE wave verifier {ID} (read-only + checkpoint file). INPUT: the wave's writer reports.
1 Gates on the wave scope: readme-coverage, link-check, markdown-placement; each writer's EXTRA checks.
2 Claim audit: ≥15 random claims per writer (paths, commands, routes, env vars, types, Used-by callers)
  checked against code; list failures file:line.
3 Standard conformance: sample 10 files per writer (H1, purpose, Contents root/roles/globs, section
  names, Boundaries bullets, repo-root links, no history/ticket IDs in READMEs, status lines, ≤500
  lines, terminology, glossary links).
4 Scope: files changed ⊆ the writers' YOU OWN lists; no foreign hunks.
5 Update refactor_progress_checkpoint.md rows. VERDICT: PASS or FIX-LIST (writer, file, problem, fix).
```

## Phase 5: Feature docs, runbooks, standards (4 writers per wave; one verifier per wave)

**Doc writer template (F01–F19)**

```text
[BASE BRIEF] ROLE doc writer {ID}: {title}. YOU OWN: {targets} + documentation_v2/pending_merge/{ID}/**
(delete each source once merged) + README.md indexes of the documentation_v2 folders you create + the
"## Related documentation" section (only) of the in-code READMEs for the code your docs cover (add
backlinks to your docs).
SOURCES → TARGETS: {rows from the manifest}. CONTENT PINS: {must-keep text}.
Read first: templates (feature_doc, runbook, decisions_entry, known_bug, glossary_entry, folder_index),
glossary.md, the in-code READMEs of the code covered.
1 Inventory every fact of every source; verify each (rule 4); runbooks: static check + safe runs only.
2 Write from the template; ≤500 lines per file (split into a folder with README index); glossary links
  on first use; "## Open work" = design-target gaps each linked to an open ticket spec (check status in
  the TOML); decisions → decisions.md entries.
3 Visual reference sets: folder README states "design-phase reference" or "live design target", what it
  shows and how the built UI differs.
4 Delete merged sources only after the disposition table covers every fact.
Checks: readme-coverage + link-check + markdown-placement on your targets; EXTRA checks.
```

| ID | Targets | Sources and pins |
|---|---|---|
| F01 (pilot) | `website/frontend/pages/administration/**` (audit_logs, approvals, content_manager, personnel, event_manager, server_control incl. fleet commands, fleet scenarios, machine credentials, mission deployments) | Legacy page specs: audit-logs, mission-approvals, content-manager, personnel-roster, event-manager, server-control. Plus `page.md` ×6, draft hubs and the matching visual sets. |
| F02 | `pages/{command_center, operations}/**` | Legacy page specs: announcements, dashboard, server-intel, event-schedule, event-hub, deployments, leaderboards. Plus `page.md`, draft hubs and visual sets. New docs for `/announcements/:id` and the ORBAT route. |
| F03 | `pages/{mission_hub, doctrine_and_info, field_tools}/**` | Legacy page specs: mission-library, mission-overview, wiki (including `/wiki/:slug`), vehicle-database, modpacks, mortar-calculator, debug-building-viewer. New docs for `/debug/world-los` and the artifact workspace. `mission-creator.md` goes to archive. |
| F04 | `pages/{account, navigation}/**`, `website/frontend/README.md` (route table: every route → page folder → feature doc), frontend core docs | `auth/*`, settings, not-found, `shell/*`, `nav_config.md`, `layout.md`, frontend README/INDEX/ROADMAP/TRACKING/`_template` |
| F05 | `apps/editor/feature_inventory/` part 1 (areas A–M) plus the folder README | `feature_inventory.md` first half, `reference/feds_schema.md` |
| F06 | `feature_inventory/` part 2 (areas N–Z) | second half |
| F07 | `apps/editor/{mission_creator_roadmap.md, ux_spec.md, decisions.md}`, `eden_editor_reference/eden_gap_analysis.md` | ROADMAP, `agent_execution` (the execution half goes to archive), `ux_spec`, `pages/mission-editor.md`. **Pins:** ROADMAP sync markers. Every `\| eden_id \| … priority \|` table stays in one file and passes the round trip, with no priority column. Keep "a multi-selection now OPENS multi-edit" and "T-649 ✅". Run `ticket check --strict` and `mk ci-local-leptos`. |
| F08 | `eden_editor_reference/{interactions/ (split), attributes, ui_anatomy}`, `apps/editor/arsenal/`, editor `visual_references/`, `apps/{planner, aar}/` (planned, not built), the editor folder README | Eden catalogs, arsenal docs, mockup sets, draft planner and aar hubs |
| F09 | `website/README.md`, `website/api_v2/**` (overview, env reference, domain docs, `verification/README.md` index), `website/map-engine/**`, `website/graphics-engine/**`, `standards/engine_boundary_rules.md` | `backend/ROADMAP`, `backend/README`, the rule sections of `ENGINE_SPLIT_PROGRAM`, `.env.example`, draft engine hubs |
| F10 | `mod/tbd-framework/**`: `mod_design.md`, the capability verdicts index, `UI/**` (index plus one folder per screen: spec, screenshots, Stitch sets; briefing spec split) | TBD_MOD_DESIGN, UI_STRUCTURE, UI_REFERENCES, the 13 screen specs, `vanilla_carve_coverage`. **Pins:** the 12 `@idx` citations and the section names `verify_crf_leak.rs:38-42` cites. Run `hcargo run -p developer-tools --bin enf -- citations` and `enf capability`. |
| F11 | `mod/README.md`, `mod/tbd-export/**` (exporter docs and evidence index), `mod/tbd-emcp/**` | `docs/mod/README`, the MCP parts of `CLAUDE-CODE-START`, export and evidence READMEs |
| F12 | `tools_v2/**` docs, `ticketboard/**`, `fleet_host_agent/**`, `contracts_v2/**` (including `definitions/bridge_messages.md`), `assets_v2/**` | `token_estimate_factor` (**pins:** quotes `.ai/`, `docs/TICKET_`, `Cargo.lock`; run `hcargo test -p ticket-engine estimate_provenance`), draft tools hubs |
| F13 | `runbooks/{local_development, website_deployment, database_operations, testing_and_ci}.md` | DEV_RUNBOOK, HOME_SERVER, the draft runbooks, the CODING_STANDARDS gate matrix. Safe runs only. |
| F14 | `runbooks/{editor_gates, editor_capture, enfusion_mcp_tooling, spawn_determinism}.md`, `runbooks/game_server_staging/` | EDITOR_GATE_RUNBOOK, `editor_capture`, MCP_TOOLING, SPAWN_DETERMINISM, STAGING-SERVER. Runs: `mcp selftest`, gate doctor. |
| F15 | `runbooks/two_client_playtest/` (split), `runbooks/mod_slice_workflow.md` | PLAYTEST_RUNBOOK, SLICE_WORKFLOW (its `@idx` citation moves into `mod_design` or its disposition is recorded), VERIFY_AGENT_PROMPT, `CLAUDE-CODE-START` |
| F16 | `runbooks/factory_waves/` (agent-neutral; states the Law 2 tooling exception), `runbooks/ticket_run_pipeline.md`, `runbooks/cursor_workspace_setup.md` | PLATFORM_FACTORY, FACTORY_FOR_CURSOR, EDITOR_FACTORY_*, EDITOR_SLICE/VERIFY_BRIEF (dated parts go to `archive/factory_runs`), the ticket section of AGENT_COMMIT_CHECKLIST, CURSOR_SETUP |
| F17 | `standards/{coding_standards/ (split), where_does_x_go, commit_checklist, ticket_identifiers}.md`, `known_bugs/**`, `design_system/**` | CODING_STANDARDS, WHERE_DOES_X_GO, AGENT_COMMIT_CHECKLIST, TAGS, KB-001 and KB-002, THEME, the token DESIGN files, the draft design system. Verify against `aegis.css`, `classify.rs`, `TBD_UITheme.c` and `TBD_MarkerData.c`. |
| F18 | Folder-index READMEs for every documentation_v2 folder no other writer owns: `tickets/` (specs and plans listed by glob), `archive/` and every `archive/<topic>/`, `runbooks/`, `standards/` | readme-coverage must be green across `documentation_v2/` |
| F19 | Fill in `glossary.md`, write `product_roadmap.md`, final pass on `documentation_v2/README.md` | Build plan, mod MILESTONES, north-star backlog, discord post. **CP4** |
| F19b | Apply the operator's strikes to `product_roadmap.md` | CP4 answers |

**Waves:** W5.1 = F02, F03, F04, F09 · W5.2 = F05, F06, F07, F08 · W5.3 = F10, F11, F12, F13 ·
W5.4 = F14, F15, F16, F17 · W5.5 = F18, F19. Verifiers V5.1–V5.5 follow the Phase 4 template. At the
end of Phase 5, `pending_merge/` must be empty.

## Phase 6: Agent instruction files (G1 ∥ G2 → G3)

**G1** (~250k)

```text
[BASE BRIEF] ROLE G1. YOU OWN: CLAUDE.md (dirty lines of the other session stay untouched).
Full refresh against code: atlas lines (graphics-engine folders, map-engine overlay/symbology, no MGRS
claim, fleet_host_agent, documentation_v2 tree, no docs/), laws (Law 2 + tooling exception;
docs-with-code rule), commands (no ticket views), Mission Creator naming, doc pointers. The
orchestrator copies it byte-identical to AGENTS.md.
```

**G2** (~300k)

```text
[BASE BRIEF] ROLE G2. GRANT: git mv. YOU OWN: .cursor/rules/*.mdc, apps/mod/.cursor/**,
.ai/tickets/{README.md, AI_PLAYBOOK.md → agent_playbook.md, CLAUDE_CODE_PROMPT.md →
implementation_prompt.md, SPEC_TEMPLATE.md → spec_template.md, HANDOFF_TEMPLATE.md →
handoff_template.md, plan_template.md}, and every link to those names.
Full refresh: remove the Cursor-owns-docs split; factory mode → /documentation_v2/runbooks/
factory_waves/; ticket workflow per /documentation_v2/runbooks/ticket_run_pipeline.md; templates match
standards; verify every command.
```

**G3** is the wave verifier template over G1 and G2, plus `ticket check --strict` and the prose rules.

## Phase 7: Gates on, close-out (H1 → H2)

**H1** (~300k)

```text
[BASE BRIEF] ROLE H1. YOU OWN: tools_v2/xtask/src/commands/ci/{task_definitions.rs,
task_definitions/verification_dispatch.rs, task_runner.rs, task_runner/split_cmd.rs, tests/
task_runner.rs}, core/repository_layout.rs (LAYOUT_TARGET_DIR removal), .github/workflows/ci.yml.
Add ONE composite task row `verify-documentation` (group verify, Lane::Ci; adapters in
verification_dispatch.rs — task_definitions.rs is at 499 lines) running the three gates; add it to
ci-local and update ci_local_step_set_is_frozen; add three `cargo xtask verify …` steps to ci.yml's
language-gates job; delete verify-doc-layout (row :130-138, step :120, split_cmd.rs:239-285,
task_runner.rs:142-153,213,216, LAYOUT_TARGET_DIR, test :264-280, prose). Run the gates repo-wide;
every failure outside your ownership goes to the report for fix agents.
Checks: hcargo test -p xtask; clippy -D warnings; hcargo xtask ci verify-documentation.
```

**H2** (~300k)

```text
[BASE BRIEF] ROLE H2. GRANT: git mv (refactor_* files only). YOU OWN: documentation_v2/refactor_* →
documentation_v2/archive/documentation_v2_refactor/ (+ index README), the checkpoint file.
Full matrix, one per call: ticket check --strict; cargo test for ticket-engine, xtask, developer-tools,
ticketboard, verification-core, website-api (architecture_rules); clippy -D warnings; mk
ci-local-leptos; mod compile; mcp selftest; verify readme-coverage / markdown-placement / link-check;
ci-local (podman shim + db started); verify api-readiness --execute on a quiet tree; the git grep
check. Archive the program files; final report.
```

After H2, the orchestrator updates memory (refactor-wave status, docs-path facts, resume pointers),
commits, and closes the program.

## Code pins (P0-5 builds the full catalogue; known set)

**Owner modules:**
- `ticket-engine/src/repository.rs::documentation`: TREE_DIR, PLANS_DIR, PLAN_TEMPLATE, SPECS_DIR,
  ROADMAP, GAP_ANALYSIS, TOKEN_ESTIMATE_FACTOR_DOC, the scan roots and exemptions,
  ARCHIVED_WAVE_PLAN_READERS and SPARSE_CHECKOUT_SETS. ARCHIVED_WAVE_PLANS and
  RETIRED_QUEUE_VIEW_PREFIX stay unchanged.
- `xtask/src/core/repository_layout.rs::documentation`: FACTORY_PACK_WAVE, the runbook paths,
  API_READINESS_*, CODE_TREES, ARCHIVE_DIR, PERMALINK_BASE.
- `developer-tools/src/repository_layout.rs::documentation`: MOD_DOCS_DIR, the capability verdicts
  TSV, the editor gate runbook.

**Outside the owner modules** (rewritten by P2-2):
- `shell_wiring.rs:542` (`include_str!`)
- ticketboard `file_watch.rs`, `git_status.rs` and their tests
- the ticket-engine fixtures in `status_and_shipping_tests.rs`, `schema_and_integrity_tests.rs`,
  `estimate_provenance_tests.rs` and `repository_layout_tests.rs`
- `validation/references.rs:151`
- `gate-env.json:6`, the systemd unit `:35`, `Caddyfile.website:7`, `.editorconfig-checker.json:13-14`
- `.gitignore:3,24`, the `ci.yml`/`contracts.yml` comments and the `ci.yml:164-169,259` text
- `contract_citations.rs:165-168`, `estimates.schema.json:4`, and the corpus-pins and scope-vocab comments
- `.env.example:26,85`, `seeds/discord_roles.sql:20`
- the api_v2 and fleet agent comments
- the EnfScript comments (about 30) and the developer-tools comments
- the xtask message texts (runbooks, engine-layer rules, the `verify_crf_leak` section names)
- `.ai/tickets/wave.lock` (via repack) and `queue.json` (via sync)

## Verification (end state, run by H2)

- `ticket check --strict` and `wave repack` are green.
- Every tooling crate's tests and clippy pass.
- `mk ci-local-leptos`, `mod compile`, `mcp selftest` and `ci-local` pass.
- The three documentation gates are green repo-wide and wired into `ci-local` and `ci.yml`.
- `verify api-readiness --execute` has been run once on a quiet tree.
- `git grep` finds no `docs/` path outside the historical constants, frozen permalinks, `.ai/artifacts`
  and the synthetic test fixtures.
- `pending_merge/` is empty, and `docs/` is gone.
- The operator has signed off CP1, CP2, CP3a, CP3b and CP4.
