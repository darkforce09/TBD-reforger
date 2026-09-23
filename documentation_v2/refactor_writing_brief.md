**Status:** live — writing brief for every Documentation V2 sub-agent

# Documentation V2 writing brief

Every Documentation V2 sub-agent reads this file right after `CLAUDE.md`. It holds the base brief
that every role prompt starts from, the operator decisions, the README and document standard, how
each known contradiction between documents resolves, and the program's terminology. The whole
program, with every role template and slice table, is
[refactor_program_plan.md](/documentation_v2/refactor_program_plan.md); the resume file is
[refactor_progress_checkpoint.md](/documentation_v2/refactor_progress_checkpoint.md).

## Base brief

The plan's BASE BRIEF, verbatim. A role prompt is this text, then the role template, then the slice
parameters.

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

## Operator decisions (2026-09-23)

Verbatim from the plan. A decision overrides any document that disagrees with it.

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

## README and document standard

Verbatim from the plan. P3-1 turns it into `documentation_v2/standards/readme_standard.md` and the
templates under `documentation_v2/standards/templates/`, and CP3a reviews them; until CP3a this
section is the reference.

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

## Contradiction resolutions

Documents in the tree disagree with each other and with the code on the points below. Each entry
states the fact as the code has it, with the proof; writers carry that fact and drop the
contradicted one. Record any further contradiction as file:line on both sides (base brief rule 4).

- **Health endpoint.** The API's health probe is `GET /healthz`, routed in
  `apps/website/api_v2/src/core/http_router.rs:74` and served by
  `apps/website/api_v2/src/core/observability/health_probe.rs`. No other health route exists;
  `/api/v1/health` is wrong.
- **Ports.**
  - SPA: the Trunk server listens on `127.0.0.1:3000` (`apps/website/frontend/Trunk.toml:22`) and
    proxies `/api` and `/map-assets` to the API on `127.0.0.1:8080` (the same file's `[[proxy]]`
    blocks).
  - API: binds `PORT`, default `8080` (`apps/website/api_v2/src/core/configuration/mod.rs:137`,
    `apps/website/api_v2/.env.example:15`). On the deployed host, Caddy listens on `:3080`, serves
    the built SPA from `apps/website/frontend/dist` and reverse-proxies API traffic to
    `127.0.0.1:8080` (`tools_v2/xtask/deploy/Caddyfile.website:13`). An API on `:8081` is wrong.
  - Local Postgres: host port `5434` (`apps/website/api_v2/docker-compose.yml:13`; `DATABASE_URL`
    at `apps/website/api_v2/.env.example:63`), the compose service `db` that `cargo xtask db up`
    starts (`tools_v2/xtask/src/commands/db/operations.rs:242`).
  - Staging Postgres: `127.0.0.1:${TBD_POSTGRES_HOST_PORT:-5432}`
    (`apps/website/docker-compose.staging.yml:29`). `cargo xtask deploy website` exports the value
    from `deploy.env` (`tools_v2/xtask/src/commands/deploy/website.rs:90`), so changing it needs no
    compose edit. A fixed staging port of `:5433` is wrong.
- **Law 2 and its tooling exception.** Commits land directly on `main`, and agents never create
  branches (`CLAUDE.md` Law 2). The one exception is the `slice/<id>` branches that xtask slice and
  wave tooling creates and deletes itself. `cargo xtask platform slice-worktree new <id>` creates
  `slice/<id>` from `main` (`tools_v2/xtask/src/commands/platform/slice_worktree/git_plain.rs:198`);
  `drop` force-deletes it (`tools_v2/xtask/src/commands/platform/slice_worktree/drop.rs:83`) and
  `reap` deletes the merged ones (same file, line 184); wave landing merges them
  (`tools_v2/xtask/src/commands/platform/wave_execution/land/merge_execution.rs`); the mod wave uses
  the same names (`tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs:177`).
  `cargo xtask ticket clean <id>` and `ticket done <id>` delete a leftover `ticket/<id>` branch and
  create none (`tools_v2/xtask/src/commands/ticket/execution.rs:28`). `CLAUDE.md` does not state the
  exception; G1 adds it.
- **File-size allowlist.** Retired: no allowlist file exists and none may be created (`CLAUDE.md`
  Law 7). `cargo xtask verify file-length` walks the Rust source roots, allows at most 500 lines in a
  production file and 1000 in a test file, and has no exemption mechanism
  (`tools_v2/xtask/src/verifications/language_bans/node_and_file_limits.rs:14`,
  `tools_v2/xtask/src/verifications/language_bans/node_and_file_limits/repository_access.rs:28`).
  Any mention of `.coding-standards-allowlist.yaml` or of allowlist entries is wrong.
- **Ticket commands.** Every ticket operation is `cargo xtask ticket <verb>` (the `Ticket` group in
  `tools_v2/xtask/src/cli/mod.rs:36`; the verbs in `tools_v2/xtask/src/commands/ticket/cli.rs`). The
  repository tracks no `scripts/` directory; `./scripts/ticket` is wrong.
- **Ticket storage.** One TOML file per ticket, `.ai/tickets/T-<id>.toml` for parent and dotted
  child ids alike, beside the `.ai/tickets/ROOT` marker; the loader is
  `tools_v2/ticket-engine/src/registry/mod.rs:17`. There is no `registry.json`.
  `cargo xtask ticket sync` writes `.ai/tickets/queue.json`
  (`tools_v2/ticket-engine/src/sync/runner.rs:44`).
- **Ticket statuses.** `idea`, `queued`, `ready`, `running`, `review`, `shipped`, `deferred`,
  `cancelled` (`tools_v2/ticket-engine/src/model/status.rs:8`, `.ai/tickets/schema.json:41`). A
  status list without `running` and `review` is incomplete.
- **T-092.** Shipped (`.ai/tickets/T-092.toml:5`); "blocked on T-092" and "not yet live" are wrong.
  The route its summary names, `GET /api/v1/missions/:id/compiled`, is not in the API: the game
  server fetches compiled missions from `GET /api/v1/game-runtime/artifacts/{artifactId}`
  (`apps/website/api_v2/src/missions/routes.rs:133`) and rosters from
  `GET /api/v1/game-runtime/events/{id}/roster` (`apps/website/api_v2/src/operations/routes.rs:127`).
- **Engine split.** Done. The workspace holds `website-graphics-engine`
  (`apps/website/graphics-engine`) and `website-map-engine` (`apps/website/map-engine`, whose
  `src/data/` module holds the mission domain) (`Cargo.toml:6`), and `cargo xtask verify
  engine-layers` enforces the layer rules as a `ci-local` step
  (`tools_v2/xtask/src/commands/ci/task_definitions.rs:46`). A document that calls the split
  proposed or pending is wrong. F09 extracts its rules into
  `documentation_v2/standards/engine_boundary_rules.md`.
- **CLAUDE.md has no §Status.** `CLAUDE.md` has three sections: Core Project Laws, Monorepo
  Directory Atlas and Canonical Commands. The shipped-history record is
  `docs/platform/SHIPPED_HISTORY.md`, an archive that is not working context. A pointer to
  "CLAUDE.md §Status" is stale.
- **Frontend code location.** Frontend code lives under `apps/website/frontend/src/v2/`. The `src/`
  root holds only `main.rs`, `app_routes.rs`, `router.rs` and `tests/route_authorization.rs`.
- **Deploy files.** The deploy templates and units live under `tools_v2/xtask/deploy/`:
  `Caddyfile.website`, `deploy.env.example`, `README.md` and `systemd/` (`tbd-website-api.service`,
  `tbd-reforger.service`, `fleet-host-agent.service`, and the `tbd-website-backup` and
  `tbd-website-backup-drill` service and timer pairs). The operator's filled copy is
  `tools_v2/xtask/deploy/deploy.env`: gitignored (`.gitignore:11`), named by `DEPLOY_ENV` in
  `tools_v2/xtask/src/core/repository_layout.rs:14`; `cargo xtask deploy website` also accepts a
  `DEPLOY_ENV` environment variable that overrides the path
  (`tools_v2/xtask/src/commands/deploy/website.rs:59`). The staging compose file sits with the
  website code: `apps/website/docker-compose.staging.yml`.
- **Deploy host.** The home and staging host is whatever `TBD_SSH_HOST` names in
  `tools_v2/xtask/deploy/deploy.env`. `cargo xtask deploy website` requires it
  (`tools_v2/xtask/src/commands/deploy/website.rs:81`), and `deploy staging` and the `debug` remote
  probes read it (`tools_v2/xtask/src/commands/deploy/staging/config.rs:202`,
  `tools_v2/xtask/src/commands/debug/remote_logs/execution.rs:307`). Documents name the variable and
  never write an address; CP2 confirms the host.
- **Doc ownership.** The Cursor-writes-docs split is retired: documentation ships in the same commit
  as the code it describes, whichever agent writes that code. `.cursor/rules/tbd-platform.mdc` still
  states the split (its description line and the Cursor and Claude Code role lines); G2 removes it.

## Terminology

Every document uses these terms. Code identifiers are quoted exactly as spelled, even where they use
another word.

- **Mission Creator** — the 2D/3D CAD editor in which mission makers build missions. Code:
  `apps/website/frontend/src/v2/apps/editor/`; route `/missions/:id/edit`
  (`apps/website/frontend/src/app_routes.rs:53`). Its documentation lives under
  `documentation_v2/website/frontend/apps/editor/`. Prose never calls it the Scenario Creator.
- **mission** — the platform document a mission maker authors in the Mission Creator, with its
  versions, reviews, immutable artifacts and deployments (API domain
  `apps/website/api_v2/src/missions/`). Prose says mission everywhere. Code identifiers that still
  say scenario (for example the map-engine's `data/scenario` module) keep that spelling until the
  scenario→mission rename program, which follows this one.
- **mission header** — Enfusion's world plus game-mode configuration that a dedicated server boots,
  an `SCR_MissionHeader` config such as `apps/mod/tbd-framework/Missions/TBD_Dev_POC.conf`. Code
  identifiers still say scenario here too (for example the `/fleet/scenarios` routes in
  `apps/website/api_v2/src/server_infrastructure/routes.rs:85`) until the rename program.
  Engine-owned names, such as the dedicated-server config's `scenarioId` and the
  `SCR_EScenario*` types, keep Enfusion's spelling for good.
- **event** — a scheduled community session record: its time, attached mission, ORBAT slots,
  sign-ups and waitlist (`apps/website/api_v2/src/operations/handlers/`: the `event_*.rs` handlers,
  `slot_registration.rs`, `waitlist_promotion.rs`). "Event" alone always means this record; other
  kinds are named in full (SSE event, DOM event, script event).
- **operations** — the domain covering events, the ORBAT and slotting, and member service records:
  `apps/website/api_v2/src/operations/` (which also serves fire missions, leave requests and
  game-runtime deployments) and the pages under `apps/website/frontend/src/v2/pages/operations/`.

## Checkpoint 1 decisions (2026-09-23)

The operator accepted the recommended answer to all 21 questions in
[cp1_questions.md](/documentation_v2/refactor_move_manifest/cp1_questions.md); the manifest, the
rewrite TSVs and the pin catalogue follow them. Writers follow these rules. Paths are relative to
`documentation_v2/`.

- **Visual reference sets.** A set is named `<subject>_<kind>`, kind `blueprint`, `mockup` or
  `render` (`mortar_calculator_blueprint`, `mission_header_browser_mockup`,
  `satellite_backdrop_render`); a mod Stitch set keeps its panel folder name without
  `tbd_reforger_` and `_standalone`. Its files are `<set_name>.html` (the Stitch export),
  `<set_name>.png` (its screenshot; a render has only this) and `design_tokens.md` when the export
  carries tokens. A mod screen's in-game captures sit beside its sets in `reference_screenshots/`.
- **Set placement.** A set sits in the `visual_references/` folder of the feature it depicts, per
  page wherever the code has per-page folders:
  `website/frontend/pages/command_center/dashboard/visual_references/command_dashboard_blueprint/`.
- **Evidence folders.** Named `verification_evidence/` (`website/api_v2/verification_evidence/`,
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/`), never
  `verification/`, which collides with the mirror of the code folder `Verification/` on a
  case-insensitive filesystem. Hyphenated evidence names inside keep their spelling.
- **Feature docs.** Each feature doc is its own file beside the folder's README index:
  `<page component>_page.md` in a page folder (`personnel_roster_page.md`), `account_pages.md`
  (login, auth callback, settings), `app_layout_and_navigation.md` (layout, sidebar, top nav,
  not-found page) and `<screen>_specification.md` for a mod screen (`lobby_specification.md`).
- **pending_merge/.** Secondary sources keep their original names under `pending_merge/<writer>/`
  (exempt from snake_case); the folder is empty by the end of Phase 5.
- **Known bugs.** A resolved entry stays in `known_bugs/` with status resolved.
- **Ticket citations.** A citation this program adds reads `Design: <path>.`.
- **Redirect stubs.** A ticket mention of a redirect stub points at the stub's real target, not at
  the archived stub.
- **macOS UX methodology.** F17 writes it into `design_system/` (context retention, progressive
  disclosure, frictionless action; split pane, create-over-list dialog, inline toggles, slide-over
  dossier); the React-era audit is archived at
  `archive/go_and_react_era_design/macos_ux_architecture.md`.
- **Placements.** The shipped-history log `SHIPPED_HISTORY.md` goes to `archive/shipped_history/`;
  the Eden wiki scrape manifest stays as data beside the Eden catalogs
  (`website/frontend/apps/editor/eden_editor_reference/eden_wiki_scrape_manifest.yaml`); vanilla
  source coverage stays live at `mod/tbd-framework/vanilla_source_coverage.md`; the mod agent start
  file `CLAUDE-CODE-START.md` merges into `runbooks/mod_slice_workflow.md`.
