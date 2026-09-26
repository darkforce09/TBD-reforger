**Status:** live

# Enfusion MCP Workbench bridge

The `TBD_EMCP` addon of the [mod](/documentation_v2/glossary/g_to_m.md#mod) and the commands around it let
agents and scripts drive a running [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) from the
command line: inspect and edit the open world, run editor actions, reload scripts and call the
export handlers. Mod developers and agents working in Workbench read it.

## Where it lives

- Code: [`apps/mod/tbd-emcp/`](/apps/mod/tbd-emcp/README.md), whose
  [`Scripts/WorkbenchGame/EnfusionMCP/`](/apps/mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/README.md)
  holds the nineteen Net API handlers; the command side is
  [`tools_v2/xtask/src/commands/mcp/`](/tools_v2/xtask/src/commands/mcp/README.md), the broker
  `tools_v2/developer-tools/src/bin/mcpd.rs` and the pinned package in
  `tools_v2/enfusion_mcp_node_package/package.json` (`enfusion-mcp` 0.6.1).
- Entry: `cargo xtask mod dev-bootstrap`
  (`tools_v2/xtask/src/commands/mod_ops/development_bootstrap.rs`) brings the bridge up;
  `cargo xtask mcp call` and `cargo xtask mcp wbcall` use it.
- Related: the [Enfusion MCP tooling runbook](/documentation_v2/runbooks/enfusion_mcp_tooling.md)
  (the broker, exit codes, environment and live checks) and the
  [map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md)
  feature, whose blueprint handler is called through the same Net API.

## Behaviour

### Two ways to reach Workbench

```text
cargo xtask mcp call <tool> '<json>'
    └─▶ mcpd broker (warm) or a one-shot enfusion-mcp server
          └─▶ enfusion-mcp tool ── wb_* tools ──▶ Workbench Net API :5775 ──▶ EMCP_WB_<Name> (TBD_EMCP)
                                   other tools (api_search, game_read, mod_validate, …) read the index, the pak farm or disk
cargo xtask mcp wbcall <APIFunc> '<json>'
    └─▶ Workbench Net API :5775 directly ──▶ any NetApiHandler: EMCP_WB_* of TBD_EMCP,
                                              EMCP_WB_TbdBlueprint and EMCP_WB_SourceExport of TBD_Export
```

- `mcp call` names an MCP tool of the pinned `enfusion-mcp` server. The `wb_*` tools map onto the
  nineteen handlers of this addon (the handler README's table gives the mapping); the other tools
  search the API, read game data from the pak farm or validate a mod on disk. Its exit code is 0
  success, 1 usage or empty, 2 init failed, 3 tool error, 4 timeout.
- `mcp wbcall` names a Net API handler class. It is the only way to reach a handler the MCP server
  has no tool for, such as the export addon's `EMCP_WB_TbdBlueprint`, and it also reaches this
  addon's handlers, for example `cargo xtask mcp wbcall EMCP_WB_ScriptEditor '{"action":"getAllText"}'`.
  Its exit code is 0 JSON printed, 1 usage, 2 no connection, 3 Workbench error; it waits 600 s by
  default (`--timeout`).

### Bringing the bridge up

`cargo xtask mod dev-bootstrap` prepares the workstation in this order:

1. Builds the pak symlink farm the MCP server reads as its game root, the work of
   `cargo xtask setup mcp-game-root` (default `~/.cache/enfusion-mcp-root`).
2. Runs `npm ci` in `tools_v2/enfusion_mcp_node_package/` when the pinned server is not installed;
   a failed install warns and falls back to npm's cache.
3. Stops with exit 1 when `apps/mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_Ping.c` is
   missing: the handlers are committed here and never copied into another addon.
4. When nothing listens on the Net API port (`ENFUSION_WORKBENCH_PORT`, default 5775), launches
   Workbench with `steam -applaunch 1874910 -gproj` on `apps/mod/tbd-export/addon.gproj`, which
   skips the project picker and loads `TBD_EMCP` through the export addon's dependency, and polls
   the port for up to `TBD_WB_WAIT_SEC` seconds (default 180).
5. When the port is still closed, prints "ACTION REQUIRED: Launch Arma Reforger Tools from Steam,
   open …/apps/mod/tbd-export/addon.gproj, enable Net API (File > Options > General)." and exits 1;
   the operator does that and runs the command again.
6. Pre-warms the MCP broker (`mcp daemon start`); a failure only warns, since `mcp call` falls back
   to a one-shot server.
7. Calls `wb_connect`; on failure it prints that Workbench must have the export addon open and
   exits 1.
8. Runs `mod_validate` on `apps/mod/tbd-framework` and `apps/mod/tbd-export`, printing the results
   without failing on them.

`--api` and `--server` add optional steps outside the bridge. After a clean bootstrap,
`cargo xtask mcp smoke` checks `wb_connect` and `wb_state` live, and `cargo xtask mcp selftest`
checks the call path offline, with no Workbench. `cargo xtask mcp daemon stop-all` clears stray
brokers and servers.

### Loading rules

- Workbench loads the handlers once, from `TBD_EMCP`. Opening `apps/mod/tbd-export/addon.gproj`
  loads it as a dependency; the framework does not depend on it, so a framework-only session has a
  bridge only when `TBD_EMCP` is loaded beside it.
- The MCP's `wb_launch` with `gprojPath` copies a second handler set into the addon it names;
  Workbench then fails the WorkbenchGame module on "Multiple declaration" and the bridge dies.
  `cargo xtask mod compile` exits 1 when such a copy lands in a `Scripts/WorkbenchGame/` folder of
  `apps/mod/tbd-framework/`.
- The MCP's `wb_cleanup` deletes `Scripts/WorkbenchGame/EnfusionMCP/` in the addon it names, so it
  is never pointed at `apps/mod/tbd-emcp`; the export addon keeps its own handlers outside such a
  folder for the same reason.
- A new `.c` file needs a Workbench restart, because Workbench builds its script list when it
  loads the project; an edited one needs a script reload (`wb_reload`).
- A dedicated server never loads the addon: `cargo xtask deploy staging` excludes
  `apps/mod/tbd-emcp/` and `apps/mod/tbd-export/` from its rsync
  (`tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs:31-32`).

### Native MCP tools in an editor session

`apps/mod/.mcp.json` registers the `enfusion-mcp` server for an agent session opened at
`apps/mod/`, and the repository root's `.cursor/mcp.json` registers it for a Cursor workspace
opened at the checkout root. Each passes `ENFUSION_GAME_PATH`,
`ENFUSION_WORKBENCH_PATH` and `ENFUSION_PROJECT_PATH`, which must name this machine's pak farm,
Arma Reforger Tools install and Workbench addons folder; `mcp call` fills the same three with the
usual Steam and Workbench folders when they are unset.

### Developing an exporter through the bridge

With the bridge up, an exporter is written against Workbench rather than guessed: look the API and
resource names up with `api_search`, `asset_search`, `game_read` and `game_browse`, write the
script in `apps/mod/tbd-export/`, reload with `wb_reload`, check with `mod_validate`, and run it.
Resource GUIDs come from the tools or an export, never typed by hand. The item registry follows
this path: its plugin writes `$profile:TBD_RegistryItems.json` and `$profile:TBD_RegistryCompat.json`,
which are copied into `contracts_v2/catalogs/registry-items.workbench.json` and
`registry-compat.workbench.json`, checked with `cargo xtask ci schema-validate` and loaded into the
development database with `cargo xtask db registry-import`; `cargo xtask db seed` applies
`apps/website/api_v2/seeds/registry_dev.sql` instead for a smoke run without Workbench. That
plugin's menu entry is commented out today, so the registry step has no entry point.

### Known discrepancies

- `apps/mod/.mcp.json:4-5` starts `npx -y enfusion-mcp`, unpinned, and both committed MCP configs
  carry one developer's absolute home and checkout paths (`apps/mod/.mcp.json:7-9`,
  `.cursor/mcp.json`) — every xtask caller starts the pinned `enfusion-mcp` 0.6.1 from
  `tools_v2/enfusion_mcp_node_package/` and derives the paths from the environment.
- `development_bootstrap.rs:4-5,72` says the export addon loads the framework through a dependency
  — `apps/mod/tbd-export/addon.gproj:5-8` depends on vanilla and `TBD_EMCP` only.
- The dedicated server's Steam app id disagrees between commands: `cargo xtask debug direct-join`
  reads the server build from `appmanifest_1874900.acf`
  (`tools_v2/xtask/src/commands/debug/direct_join.rs:98`) — the compile, world-boot and playtest
  gates tell the operator to install app 1890870
  (`tools_v2/xtask/src/commands/mod_ops/compile/execution.rs:94` and `:132`). Workbench (1874910) and the
  client (1874880) are consistent.

## Data

- Net API: one TCP connection per request on `ENFUSION_WORKBENCH_HOST`:`ENFUSION_WORKBENCH_PORT`
  (default 127.0.0.1:5775), length-prefixed strings carrying protocol version 1, a client id,
  `JsonRPC` and the JSON object with `APIFunc` set to the handler class
  (`tools_v2/xtask/src/commands/mcp/netapi.rs`). Each handler fills a request `JsonApiStruct` and
  answers a response struct as JSON; a handler with several operations switches on `action` and
  answers an unknown one with `status` `error` and the valid actions.
- MCP: three JSON-RPC lines per call (`initialize`, `notifications/initialized`, `tools/call` with
  id 2); the broker listens on `MCP_SOCK`, else `$XDG_RUNTIME_DIR/tbd-mcp-<uid>.sock`.

## Design

The handlers are the files `enfusion-mcp` 0.6.1 ships, kept byte-identical apart from one local
action, `getAllText` in `EMCP_WB_ScriptEditor.c`, which returns every line of the open script. The
[addon README](/apps/mod/tbd-emcp/README.md#upgrading-enfusion-mcp) gives the upgrade procedure: pin
the new version, compare the package's handlers with these, copy them over, restore `getAllText`,
restart Workbench and run `cargo xtask mcp smoke`. No visual design applies.

## Open work

- [T-1095 — Pin the apps/mod MCP configs and drop personal absolute paths](/.ai/tickets/T-1095.toml)
  (idea, no plan): the two committed MCP configs start the pinned server with portable paths.
- [T-1091 — Fix xtask mod texts saying tbd-export loads the framework](/.ai/tickets/T-1091.toml)
  (idea, no plan): the bootstrap comments and messages state the real dependencies.
- [T-1096 — Decide the dedicated server Steam app id xtask relies on](/.ai/tickets/T-1096.toml)
  (idea, no plan): one server app id across the gates, the direct-join check and the docs.
- [T-1108 — Fix mcp selftest reading a missing fixture as empty transcript](/.ai/tickets/T-1108.toml)
  (idea, no plan): a missing transcript fails the self-test instead of passing as empty.
- [T-1081 — Decide whether Workbench registry and map export plugins stay unregistered](/.ai/tickets/T-1081.toml)
  (idea, no plan): whether the registry and map export plugins get their menu entries back.

## Decisions

- The handlers are committed in their own addon, not copied in at bootstrap: every session loads
  the same set, and no shipping or export addon carries MCP code.
- The export addon, not the framework, depends on `TBD_EMCP`: opening the export addon brings the
  bridge up, and the framework stays free of editor tooling.
- Launching with `-gproj` rather than through the project picker: a launch stuck in the picker never
  opens the Net API.
- `mcp call` goes through a warm broker first: one server index load serves every later call, and
  the one-shot fallback keeps calls working when the broker cannot start.
