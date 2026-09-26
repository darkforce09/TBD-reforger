**Status:** live

# Setting up the Cursor workspace

Opens this repository in Cursor so that its project rules load, the local stack runs, the
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) MCP server reaches
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) for [mod](/documentation_v2/glossary/g_to_m.md#mod)
work, and the health checks pass. Run it once per machine, and again after a clone to a new
path; it changes no code and takes about ten minutes plus the first builds.

## Prerequisites

- Cursor, and a clone of the repository on `main`.
- For the local stack: the Rust toolchain and a container runtime, as
  [Local development](/documentation_v2/runbooks/local_development.md#prerequisites) lists.
- For mod work only: Arma Reforger Tools with the Net API enabled, and Node.js with npm, as
  [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md#prerequisites) lists.

## Where the workspace files live

| Path | Tracked | Role |
|---|---|---|
| `.cursor/rules/*.mdc` | yes | the project rules Cursor loads for the whole workspace |
| `.cursor/mcp.json` | no, gitignored (`.gitignore:25`) | this machine's MCP servers for the workspace |
| `apps/mod/.cursor/rules/single-branch-main.mdc` | yes | a nested rule for the mod folder: work on `main` only, never a feature branch |
| `apps/mod/.cursor/mcp.json` | yes | the Enfusion MCP server entry to copy from: `node` on the pinned package, and the three `ENFUSION_*` paths |
| `apps/mod/.mcp.json` | yes | the same server for an agent started inside `apps/mod/`, launched as `npx -y enfusion-mcp` |
| `.ai/tickets/`, `.ai/artifacts/` | tickets yes, artifacts partly | the [ticket](/documentation_v2/glossary/n_to_z.md#ticket) files and the agents' working files; Cursor loads no rule from `.ai/` |

Both tracked MCP files hard-code one workstation's absolute paths, and `apps/mod/.mcp.json` starts
whatever `enfusion-mcp` release npm serves rather than the pinned 0.6.1 in
`tools_v2/enfusion_mcp_node_package/`. Treat them as the shape of the entry, not as values: step 4
writes the machine's own copy.

## The project rules

Cursor applies each rule in `.cursor/rules/` to every chat (`alwaysApply: true`) or, for
`application-code-forbidden.mdc`, when a chat touches a file its globs match.

| Rule | Applies | What it binds |
|---|---|---|
| `tbd-platform.mdc` | always | read `CLAUDE.md` first; the ticket files; `main` only; the executor gate; factory mode overrides the single-ticket lines |
| `cursor-agent-workflow.mdc` | always | the modes Cursor infers from a message: plan review (read-only), ticket and docs, code (not Cursor), platform factory |
| `application-code-forbidden.mdc` | [API](/documentation_v2/glossary/a_to_f.md#api), app and mod sources | no Cursor edit of application code without the operator's word, except in factory mode |
| `platform-factory-mode.mdc` | always | when the operator starts the factory, Cursor orchestrates slice agents that edit code in `slice/<id>` worktrees |
| `no-silent-deferrals.mdc` | always | the whole ask is done; only the operator defers a piece (`CLAUDE.md` law 1) |
| `no-duplicate-slice-agents.mdc` | always | one agent per ticket worktree until it finishes or the operator replaces it |
| `acceptance-gates-reproducible.mdc` | always | gates are pinned and fail fast; a gate that cannot run is fixed, never skipped |
| `class-r-plans.mdc` | always | plan rigor: every pin measured before a plan is written |
| `claude-prompt-delivery.mdc` | always | a prompt for another agent is delivered as one complete fenced block per stream |
| `subagent-model-routing.mdc` | always | cheap models for locating code, expensive ones for hard analysis and coding |

**Documentation ships with its code.** Several rules still give Cursor all documentation and
forbid the coding agent to touch it: the description and the Cursor and Claude Code lines of
`tbd-platform.mdc`, and the agent-split table of `cursor-agent-workflow.mdc`. That split is
retired: documentation ships in the same commit as the code it describes, whichever agent writes
that code, as the [commit checklist](/documentation_v2/standards/commit_checklist.md) states. Where
a rule and this decision disagree, the decision wins.

**The executor gate.** A ticket's `executor` says who may take it. `claude-code` means any AI
coding agent run through the ticket tooling; `cursor-docs` a ticket, spec or documentation pass;
`workbench`, `human` and `ci` mean an agent stops and waits for that party.

**Branches.** Work lands on `main` (`CLAUDE.md` law 2). The one exception is the `slice/<id>`
branches that `cargo xtask platform slice-worktree` and the [wave](/documentation_v2/glossary/n_to_z.md#wave)
tooling create, merge and delete themselves, which is what factory mode uses; the nested mod rule
agrees with law 2 and needs no copy at the root.

## Steps

Run every command from the repository root.

1. Open the checkout root as the workspace (File > Open Folder), not a parent folder: the rules
   and the MCP file load from the opened folder's `.cursor/`.

   ```bash
   git rev-parse --show-toplevel
   ```

   Expected: the path to open.

2. Confirm the rules are present.

   ```bash
   git ls-files .cursor/rules
   ```

   Expected: the ten `.mdc` files of the table above. Cursor's project rules settings list them;
   a chat started now answers under them. For a machine where nothing may be committed, the same
   text can go into Cursor's own project rules instead, unversioned and unshared.

3. Start the local stack: the database, the API on port 8080 and the single-page app on port
   3000, as [Local development](/documentation_v2/runbooks/local_development.md#start-the-stack)
   describes step by step. Then check the API.

   ```bash
   curl -sf http://127.0.0.1:8080/healthz
   ```

   Expected: `{"status":"ok"}`. The [dev login](/documentation_v2/glossary/a_to_f.md#dev-login) is
   `http://localhost:3000/api/v1/auth/dev-login?role=mission_maker`, and the
   [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) opens at
   `/missions/<id>/edit` once a [mission](/documentation_v2/glossary/g_to_m.md#mission) exists.

4. For mod work only, give Cursor the Enfusion MCP server: copy the tracked entry into the
   workspace's own, gitignored file.

   ```bash
   cp apps/mod/.cursor/mcp.json .cursor/mcp.json
   ```

   Expected: no output. Then edit all four paths in the copy for this machine: the `node` argument
   to `<checkout>/tools_v2/enfusion_mcp_node_package/node_modules/enfusion-mcp/dist/index.js`, and
   `ENFUSION_GAME_PATH`, `ENFUSION_WORKBENCH_PATH` and `ENFUSION_PROJECT_PATH` to the pak farm, the
   Workbench install and the addons folder, whose defaults the
   [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md#prerequisites)
   runbook gives.

5. For mod work only, install the pinned server and bring the bridge up.

   ```bash
   cargo xtask mod dev-bootstrap
   ```

   Expected: `== TBD dev bootstrap ==`, `Port 5775 is listening.`, the `wb_connect` answer and the
   two `mod_validate` results, then `Bootstrap complete.`; its `npm ci` creates the `dist/index.js`
   the MCP file names. Cursor's MCP settings then show `enfusion-mcp` connected.

6. Check the ticket files.

   ```bash
   cargo xtask ticket check --strict
   ```

   Expected: the debt and token counters, then `check OK`.

7. Check the contracts: generated types fresh, the golden missions valid and the contract
   citations resolved.

   ```bash
   cargo xtask ci ci-local-schema
   ```

   Expected: `verify-codegen-fresh`, `schema-validate` and `verify-citations` run in turn and the
   command exits 0.

8. Check the app.

   ```bash
   cargo xtask mk ci-local-leptos
   ```

   Expected: formatting, clippy for `wasm32`, the app's tests and a release build pass. The deeper
   `cargo xtask ci build` builds the API in release and the app; the whole CI replay is
   `cargo xtask ci ci-local`, in [Testing and CI](/documentation_v2/runbooks/testing_and_ci.md).

## Working in Cursor

Two chats, named so they can be found again, keep exploration apart from changes. Paste each
opening prompt once.

**Brainstorm** explores anything in the repository and changes nothing:

```text
You are in the Brainstorm chat for TBD Reforger: ideas and design for anything in the monorepo
(Mission Creator, website, API, mod). Read CLAUDE.md first. Write no code and edit no file unless I
ask. Decisions that should land go to the Tickets chat. For ticket context run
`cargo xtask ticket next` or `cargo xtask ticket show <id>`.
```

**Tickets** turns decisions into tickets, specs and plans:

```text
You are in the Tickets chat for TBD Reforger. You file and change tickets with `cargo xtask ticket`
commands, write specs and plans under documentation_v2/tickets/, and keep ticket files in the
engine's canonical form. After a change run `cargo xtask ticket check --strict`. Never edit
.ai/tickets/queue.json or other files `ticket sync` writes. Stop at a ticket whose executor is
workbench, human or ci.
```

Code for a ready ticket runs through the ticket tooling, whichever agent does it:
[Taking a ticket from idea to shipped](/documentation_v2/runbooks/ticket_run_pipeline.md) covers
`cargo xtask ticket brief`, `ticket run` and the ship. When the operator starts the platform
factory, Cursor orchestrates instead, as [Factory waves](/documentation_v2/runbooks/factory_waves/README.md)
describes.

For Mission Creator work, read in this order: the
[editor documentation index](/documentation_v2/website/frontend/apps/editor/README.md), the
[roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md), the
[decisions](/documentation_v2/website/frontend/apps/editor/decisions.md), the
[feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
and the [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md).

## Verify

```bash
cargo xtask ticket check --strict
```

Expected: `check OK`; with steps 3, 7 and 8 green and, for mod work, `enfusion-mcp` connected, the
workspace is ready.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| a chat ignores the project rules | a parent or a subfolder was opened as the workspace | open the checkout root (step 1) |
| the `enfusion-mcp` server fails to start: `Cannot find module …/dist/index.js` | the pinned package is not installed, or the path in `.cursor/mcp.json` names another checkout | step 5, then fix the `node` argument (step 4) |
| MCP calls time out | Workbench is not running, or its Net API is off | the `ACTION REQUIRED` line of `mod dev-bootstrap` names the fix; see [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md#troubleshooting) |
| an agent inside `apps/mod/` runs another `enfusion-mcp` release | `apps/mod/.mcp.json` launches `npx -y enfusion-mcp`, unpinned | point it at the pinned `dist/index.js`, as step 4 does for Cursor |
| an agent refuses to update documentation beside its code | a rule still carries the retired documentation split | documentation ships with its code (see [The project rules](#the-project-rules)) |
| `curl` exits 22 on `/healthz` | the probe returned 503: the database is down or the migrations failed | [Local development](/documentation_v2/runbooks/local_development.md#troubleshooting) |

## Related

- [Local development](/documentation_v2/runbooks/local_development.md) — the whole local stack.
- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the MCP server, the
  broker and the Workbench bridge.
- [Taking a ticket from idea to shipped](/documentation_v2/runbooks/ticket_run_pipeline.md) — the
  ticket lifecycle and `ticket run`.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — what factory mode runs.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the same cycle for mod
  tickets.
- [Commit checklist](/documentation_v2/standards/commit_checklist.md) — what every commit carries.
