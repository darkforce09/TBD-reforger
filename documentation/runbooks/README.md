**Status:** live

# Runbooks

Every operator procedure in the repository, written to be run step by step: bringing the web
platform up, testing and deploying it, running the browser gates, driving
[Workbench](/documentation/glossary/n_to_z.md#workbench), playtesting the
[mod](/documentation/glossary/g_to_m.md#mod) and working with sub-agents. Developers, the operator and AI agents read the runbook for the task
at hand before running any of its commands.

## Contents

```text
documentation/runbooks/
├── ballistics_oracle_run.md    record the engine oracle, trim, calibrate and publish a ballistics catalog
├── database_operations.md      SQL, sample data, integration tests, checksum repair, backups, restores
├── editor_capture.md           screenshot a running Mission Creator in headless Chromium
├── editor_gates.md             run and debug the headless browser gates of the single-page app
├── enfusion_mcp_tooling.md     drive a running Workbench through the pinned MCP bridge
├── game_server_staging/        prepare, deploy, verify and join the staging dedicated server
├── local_development.md        bring up Postgres, the API and the single-page app on a dev machine
├── offline_mortar_page.md      prepare the offline pack of the mortar calculator, gate it, check it by hand
├── spawn_determinism.md        prove Workbench Play, spawn and equip repeat byte-identically
├── staging_verification/       record the fleet, Discord and load receipts against the staging host
├── sub_agent_orchestration.md  when a session uses sub-agents, and how it runs and reviews them
├── testing_and_ci.md           run the repository's gates locally before a push
├── two_client_playtest/        the live mod playtest with one server and two clients
└── website_deployment.md       deploy the API and the single-page app to the home server
```

## How it works

Each runbook follows the [runbook template](/documentation/standards/templates/runbook.md):
Prerequisites, numbered Steps with one command and an `Expected:` line each, Verify,
Troubleshooting and Related. A runbook is one snake_case file named after its procedure; one that
would pass 500 lines becomes a folder whose README indexes its topic files, as
`game_server_staging/` and `two_client_playtest/` do. Every command a runbook cites is checked
against the `cargo xtask` source, and only safe runs (`--help`, `--dry-run`, the local database
and the gate doctor) are executed to check one.

Start from the task:

| Task | Runbook |
|---|---|
| First checkout, or after a reboot | [local development](/documentation/runbooks/local_development.md) |
| Database work beyond the first seed | [database operations](/documentation/runbooks/database_operations.md) |
| Before a push to `main` | [testing and CI](/documentation/runbooks/testing_and_ci.md); [editor gates](/documentation/runbooks/editor_gates.md) for the browser gates |
| Looking at the live [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) | [editor capture](/documentation/runbooks/editor_capture.md) |
| Shipping the website | [website deployment](/documentation/runbooks/website_deployment.md) |
| Running the dedicated game server | [game server staging](/documentation/runbooks/game_server_staging/README.md), or [two-client playtest](/documentation/runbooks/two_client_playtest/README.md) on a development machine |
| Mod work that needs Workbench | [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md), then [spawn determinism](/documentation/runbooks/spawn_determinism.md) after a spawn or loadout change |
| Rebuilding a terrain's object and road data | [terrain export runbook](/documentation/mod/tbd-export/Scripts/WorkbenchGame/MapExport/terrain_export_runbook.md) |
| A new game build, or a change to the ballistics oracle | [ballistics oracle run](/documentation/runbooks/ballistics_oracle_run.md) |
| The mortar calculator offline | [offline mortar page](/documentation/runbooks/offline_mortar_page.md) |
| A broad search or parallel independent pieces handed to sub-agents | [sub-agent orchestration](/documentation/runbooks/sub_agent_orchestration.md) |
| A [ticket](/documentation/glossary/n_to_z.md#ticket), or a [wave](/documentation/glossary/n_to_z.md#wave) of tickets run by agents in worktrees | the central ticket manager (`ttm --project reforger …`) and its runner (`ttm worktree`, `ttm wave`), documented in the ticket manager's own repository and configured here by [`ticket_manager_execution.toml`](/ticket_manager_execution.toml) |

The terrain export runbook lives beside the exporter it drives, in the Workbench map export
feature folder, rather than here; this index lists it so every procedure is found from one place.

## Code

- [xtask command groups](/tools/xtask/src/commands/) — every `cargo xtask` command the runbooks
  run: `mk`, `ci`, `db`, `deploy`, `mod`, `mcp`, `map`, `ballistics`, `schema` and `setup`.
  Ticket work runs through the central ticket manager's CLI, `ttm --project reforger …`.
- [Browser testing](/tools/browser_testing/browser_gate_suites/) — the gate, doctor and
  capture drivers behind the editor gates and editor capture.
- [Deploy files](/deploy/) — the templates and systemd units the deployment
  runbooks install.
- [Enfusion MCP handlers](/mod/tbd-emcp/) — the Workbench side of the MCP bridge.

## Boundaries

- Depends on: the [runbook template](/documentation/standards/templates/runbook.md); the xtask
  command tree (`tools/xtask/src/cli/` and each group's `cli.rs`), which is the final word on
  every command; `deploy/api.env.example` and `deploy/deploy.env.example`
  for settings.
- Used by: xtask code that prints or pins runbook paths (`HOME_SERVER_RUNBOOK`,
  `STAGING_SERVER_RUNBOOK` and `SPAWN_DETERMINISM_RUNBOOK` in `tools/foundation/repository_layout/src/documentation_locations.rs`;
  `EDITOR_GATE_RUNBOOK` in `tools/browser_testing/browser_gate_suites/src/gate_layout.rs`); comments in the
  browser testing drivers, the deploy units, `Caddyfile`,
  `deploy/compose.staging.yml`, `deploy/api.env.example` and mod scripts; the code READMEs of the folders above; the glossary and
  the entry README.
- Rules: a runbook path a constant pins keeps its spelling, or the constant changes in the same
  commit (the layout tests of `cargo test -p xtask` and `cargo test -p developer_tools` require
  every pinned path to exist); a runbook stays at or under 500 lines;
  every cited command exists in the command tree
  (`cargo xtask verify link-check`); no runbook writes a host address, only `TBD_SSH_HOST` from
  `deploy/deploy.env`.

## Related documentation

- [Runbook template](/documentation/standards/templates/runbook.md) — the sections every
  runbook follows.
- [Known bugs](/documentation/known_bugs/README.md) — recorded defects the gate runbooks cite.
- [Commit checklist](/documentation/standards/commit_checklist.md) — what a commit verifies
  before it lands.
