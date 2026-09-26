**Status:** live

# Enfusion MCP tooling

Drives a running [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) from a terminal through the
pinned `enfusion-mcp` server: brings the bridge up, calls MCP tools and raw Net API handlers, reads
back a Workbench Play log, and cleans up the broker. [Mod](/documentation_v2/glossary/g_to_m.md#mod) developers and agents run it whenever a
task needs Workbench. The first call pays a one-time index load of about 35 seconds; later calls go
to the warm broker. How the bridge is built, and the `mcp call` against `mcp wbcall` choice, is in
[Enfusion MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md); every command's
synopsis and exit codes are in the [MCP commands README](/tools_v2/xtask/src/commands/mcp/README.md).

## Prerequisites

- Arma Reforger Tools installed through Steam (app 1874910), with the Net API enabled in
  Workbench's general options. Check: `ss -tln` lists port 5775 once Workbench runs.
- Node.js and npm, for the pinned server `enfusion-mcp` 0.6.1 in
  `tools_v2/enfusion_mcp_node_package/package.json`; `cargo xtask mod dev-bootstrap` runs `npm ci`
  there when it is missing. Its `node_modules/` is gitignored.
- The `timeout` and `pgrep` tools on `PATH`: a one-shot call without `timeout` fails rather than
  hangs.
- The three paths the server reads, which `mcp call` fills with these defaults when unset:

  | Variable | Default | Names |
  |---|---|---|
  | `ENFUSION_GAME_PATH` | `~/.cache/enfusion-mcp-root` | the pak symlink farm that `cargo xtask setup mcp-game-root` builds |
  | `ENFUSION_WORKBENCH_PATH` | `~/.local/share/Steam/steamapps/common/Arma Reforger Tools` | the Workbench install |
  | `ENFUSION_PROJECT_PATH` | `~/Documents/Games/ArmaReforgerWorkbench/addons` | the Workbench addons folder |

## Steps

Run every command from the repository root.

1. Check the call path offline, with no Workbench.

   ```bash
   cargo xtask mcp selftest
   ```

   Expected: `✓ mcpd build+path`, then a `✓` line for each recorded transcript, one-shot and
   daemon arm, ending `mcp-call-selftest: ALL PASS (<n>)` and exit 0; a failed arm prints a `✗`
   line on stderr, the run ends `mcp-call-selftest: FAIL (<n> failed, <m> passed)` and exits 1.

2. Bring the bridge up: the pak farm, the pinned server, Workbench on the export addon (which loads
   `TBD_EMCP`), the warm broker, then `wb_connect` and `mod_validate` on both addons.

   ```bash
   cargo xtask mod dev-bootstrap
   ```

   Expected: `== TBD dev bootstrap ==`, `Port 5775 is listening.`, `Pre-warming MCP daemon...`,
   the `wb_connect` answer, the two `mod_validate` results, then `Bootstrap complete.` and exit 0.
   When the port stays closed for `TBD_WB_WAIT_SEC` seconds (default 180) it prints
   `ACTION REQUIRED: Launch Arma Reforger Tools from Steam, open …/apps/mod/tbd-export/addon.gproj,
   enable Net API (File > Options > General).` and exits 1; do that and run it again.

3. Check the broker.

   ```bash
   cargo xtask mcp daemon status
   ```

   Expected: `running (<socket>, pid <pid>)` and exit 0; `stopped` and exit 1 when no broker runs,
   in which case the next `mcp call` starts one.

4. Call an MCP tool; the arguments are one JSON object and default to `{}`.

   ```bash
   cargo xtask mcp call api_search '{"query":"GetWorldBounds"}'
   ```

   Expected: the text content of the result on stdout and exit 0. The `wb_*` tools reach the open
   Workbench (`cargo xtask mcp call wb_state`); `mod_validate` checks an addon on disk
   (`cargo xtask mcp call mod_validate '{"modPath":"'"$PWD"'/apps/mod/tbd-framework"}'`).

5. Call a Net API handler directly, for a handler no MCP tool maps to.

   ```bash
   cargo xtask mcp wbcall EMCP_WB_Ping
   ```

   Expected: the handler's response JSON on stdout and exit 0; exit 2 when nothing listens on
   `ENFUSION_WORKBENCH_HOST`:`ENFUSION_WORKBENCH_PORT` (default 127.0.0.1:5775). It waits up to
   600 s (`--timeout <s>`).

6. After a Play session in Workbench, grade the newest Workbench console log for the framework's
   spawn lines.

   ```bash
   cargo xtask mcp wb-logs
   ```

   Expected: the matching log extract and a verdict: exit 0 PASS (a player was assigned a [slot](/documentation_v2/glossary/n_to_z.md#slot)),
   1 FAIL, 2 PARTIAL (no player deployed yet), 3 ENVIRONMENT (no log found, or a usage error).
   `--file <path>` grades a given log.

7. When the session ends, or when stray brokers or servers load the machine, stop them all.

   ```bash
   cargo xtask mcp daemon stop-all
   ```

   Expected: `mcp-daemon: stop-all done`; no `mcpd` or `enfusion-mcp` process remains
   (`pgrep -af 'mcpd|enfusion-mcp'` prints nothing).

## Verify

With Workbench open on the export addon:

```bash
cargo xtask mcp smoke
```

Expected: `mcp-smoke: wb_connect OK` and `mcp-smoke: wb_state OK`, exit 0; a tool that fails prints
`mcp-smoke: <tool> FAIL (rc=<n>)` and the exit is 1.

## Troubleshooting

`mcp call` exits 0 on success, 1 on a missing tool name or an empty answer after every retry, 2
when the server's `initialize` failed, 3 on a JSON-RPC error or a result with `isError: true`
(its text on stderr), and 4 on a timeout. `MCP_DEBUG=1` prints the runner tier and the captured
stderr of a failed attempt.

| Symptom | Cause | Fix |
|---|---|---|
| exit 3 with the error on stderr | the tool reported an error, such as a bad argument or no Workbench connection; never retried | read the message; for a `wb_*` tool, run step 2 |
| exit 4 | every one-shot attempt ran past `MCP_CALL_TIMEOUT` (180 s by default, so 360 s with the default retry) | raise `MCP_CALL_TIMEOUT` for a long tool; check Workbench is responsive |
| exit 2 | the server did not answer `initialize`: a missing or broken package | `npm ci` in `tools_v2/enfusion_mcp_node_package/`, or set `ENFUSION_MCP_BIN` |
| the first call takes about 35 s | the server's index load; the broker then stays warm | none; step 2 pre-warms it |
| `mcp-daemon: failed to start (see <socket>.log)` | `mcpd` exited or its socket did not accept within 60 s | read the log; `mcp call` still works one-shot |
| Workbench reports "Multiple declaration" and the `wb_*` tools die | `wb_launch` with `gprojPath` copied a second handler set into that addon | delete that addon's copied `Scripts/WorkbenchGame/EnfusionMCP/`, restart Workbench; never pass `gprojPath` |
| `wb_connect failed — Workbench must have apps/mod/tbd-export/addon.gproj open …` | Workbench has another project open, so `TBD_EMCP` is not loaded | open the export addon and rerun step 2 |
| machine load climbs while nothing calls | stray brokers or servers from crashed sessions | step 7 |

Never point the MCP's `wb_cleanup` at `apps/mod/tbd-emcp`: it deletes the committed handlers. The
loading rules behind both warnings are in
[Enfusion MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md).

### Environment

| Variable | Default | Purpose |
|---|---|---|
| `MCP_SOCK` | `$XDG_RUNTIME_DIR/tbd-mcp-<uid>.sock` | the broker socket; `/tmp/tbd-mcp-<uid>.sock` when `XDG_RUNTIME_DIR` is unset or the path passes 100 bytes |
| `MCP_CALL_TIMEOUT` | `180` | seconds per call |
| `MCP_CALL_RETRIES` | `1` | one-shot retries after exit 1, 2 or 4; never after 0 or 3 |
| `MCP_NO_DAEMON` | unset | `1` skips the broker and always runs one-shot |
| `MCP_DEBUG` | unset | `1` prints the runner tier and captured stderr |
| `MCP_DAEMON_IDLE` | `1800` | seconds idle before the broker exits; `0` never |
| `MCP_DAEMON_MAX_LIFE` | `14400` | seconds of life before the broker exits, idle or not; `0` never |
| `MCPD_CARGO_TARGET_DIR` | `target-dev-mcpd` in the checkout | where `daemon start` builds `mcpd` |
| `ENFUSION_MCP_BIN` | unset | the server entry file, ahead of every other tier |
| `ENFUSION_WORKBENCH_HOST`, `ENFUSION_WORKBENCH_PORT` | `127.0.0.1`, `5775` | the Net API that `wbcall` reaches |

### Files and server tiers

The broker keeps its files beside the socket: `<socket>.pid` (the pid `daemon stop` kills),
`<socket>.log` (`mcpd`'s output) and `<socket>.lock` (held while one call starts the broker, so
two concurrent calls never start two). `daemon stop-all` kills every `mcpd --socket` process and
any server child left behind, and removes the `tbd-mcp-*` files in `XDG_RUNTIME_DIR` and `/tmp`.

The server command comes from the first tier that resolves
(`tools_v2/developer-tools/src/enfusion_tooling/enfusion_mcp_entrypoint.rs`):

1. `ENFUSION_MCP_BIN`, when it names an existing file;
2. `tools_v2/enfusion_mcp_node_package/node_modules/enfusion-mcp/dist/index.js`, after `npm ci`;
3. a copy npm already downloaded under `~/.npm/_npx`;
4. `npx -y enfusion-mcp`, which downloads the newest release on demand, unpinned.

## Related

- [MCP commands](/tools_v2/xtask/src/commands/mcp/README.md) — every `cargo xtask mcp` command, its
  flow and its exit codes.
- [Enfusion MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md) — the bootstrap
  order, the loading rules and the known gaps.
- [TBD EMCP addon](/apps/mod/tbd-emcp/README.md) — the nineteen Net API handlers and the
  `enfusion-mcp` upgrade procedure.
- [Developer tool executables](/tools_v2/developer-tools/src/bin/README.md) — the `mcpd` broker.
- [MCP transcript fixtures](/tools_v2/xtask/fixtures/mcp/README.md) — the recorded replies the
  self-test replays.
- [Spawn determinism](/documentation_v2/runbooks/spawn_determinism.md) — the Workbench gate that
  drives Play through `mcp call`.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — how mod work uses
  Workbench and the gates.
