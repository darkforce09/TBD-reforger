# Enfusion MCP client

The `enfusion_mcp` crate, the library behind the `cargo xtask mcp` group: tool calls to the
pinned `enfusion-mcp` server that drives
[Workbench](/documentation/glossary/n_to_z.md#workbench), through a persistent broker when one is up,
the broker's lifecycle, raw calls to the Workbench NET API, and the read-back of a Workbench Play
log. Mod developers and agents run them; the offline self-test is the check any change here must
pass. The broker itself, which needs tokio, is the separate
[`enfusion_mcp_broker`](/tools/enfusion/enfusion_mcp_broker/README.md) crate behind the `mcpd`
binary.

## Contents

```text
tools/commands/enfusion_mcp/
├── Cargo.toml  the `enfusion_mcp` library package: `clap`, `libc`, `process_runner`, `repository_layout`, `serde_json`, `thiserror`, `verification_core`; layout tier 2
└── src/        the command line and dispatch, the call path, the daemon control, the selftest and smoke, the NET API call, the log verdict, the server entrypoint
```

## How it works

`tools/xtask/src/cli/dispatch.rs` passes the parsed `McpCmd` to `enfusion_mcp::run`, after
`enfusion_mcp::preprocess_cli_args` has rewritten an empty `wb-logs --file` so the command, not
clap, answers it. A call is three JSON-RPC lines, `initialize`, `notifications/initialized` and a
`"tools/call"` with id 2, and `consume` reads the replies:

```text
mcp call <tool> [args]
  ├─ socket: MCP_SOCK, else $XDG_RUNTIME_DIR/tbd-mcp-<uid>.sock (moved to /tmp past 100 bytes)
  ├─ daemon (unless MCP_NO_DAEMON=1): start it under <socket>.lock if it is not running,
  │    then xtask mcp socket-send <socket> <tool> <args>, its output fed to xtask mcp consume
  │    0 or 3 ─▶ done;  anything else ─▶ one-shot
  └─ one-shot: requests ─▶ timeout MCP_CALL_TIMEOUT <enfusion-mcp> ─▶ xtask mcp consume
       124 ─▶ 4; 0 and 3 end at once; 1, 2 and 4 retry MCP_CALL_RETRIES times (default 1)
```

`call` fills `ENFUSION_GAME_PATH`, `ENFUSION_WORKBENCH_PATH` and `ENFUSION_PROJECT_PATH` with the
usual Steam and Workbench folders when unset, and the server command comes from
`server_entrypoint::resolve`: `ENFUSION_MCP_BIN` when it names a
file, else the package `npm ci` installed in `tools/enfusion_mcp_node_package/`, else a copy in
the npx cache, else `npx -y enfusion-mcp`. `daemon start` builds `mcpd` into `MCPD_CARGO_TARGET_DIR`
(default `target/dev-mcpd` in the checkout), spawns it in its own session with `--socket` and
`--pidfile <socket>.pid`, logs to `<socket>.log`, and counts it as running once the socket accepts a
connection. A missing `timeout` binary makes a one-shot attempt fail rather than hang. Each pipe
runs as two `process_runner` children in turn, the first one's whole stdout becoming the second
one's stdin: `socket-send` prints one line and exits, and the server exits once it has read its
requests, so neither waits on the other. `src/README.md` describes each module.

`wb-logs` finds the newest `logs_*` folder of Workbench under the Proton prefix of Steam app
1874910, else under `~/Documents/Games/ArmaReforgerWorkbench/logs`, and grades its `console.log`
with the same line vocabulary `mod remote-logs` uses in
`tools/commands/remote_debugging/src/debug/remote_logs.rs`, a copy kept in step by hand.

## Getting started

Run from the repository root:

```bash
cargo xtask mcp selftest     # the whole call path offline, against the recorded transcripts and the mcpd stub
```


## Commands

Each runs as `cargo xtask mcp <command>`; a clap usage error exits 2.

### call

- Synopsis: `cargo xtask mcp call <tool> ['<json-args>']`; the arguments default to `{}`.
- Does: one tool call, through the daemon when it can, else one-shot; prints the text content of
  the result, or the whole result as JSON when it has none. `MCP_CALL_TIMEOUT` (default 180 s),
  `MCP_CALL_RETRIES`, `MCP_NO_DAEMON` and `MCP_DEBUG=1` tune it.
- Exit codes: 0 success; 1 no tool name, or an empty answer after every retry; 2 the server's
  initialize failed; 3 a JSON-RPC or tool-reported error, its text on stderr; 4 timeout on the
  last attempt. Only 0 and 3 end a one-shot call at once; 1, 2 and 4 are retried.
- Example: `cargo xtask mcp call wb_state`

### daemon

- Synopsis: `cargo xtask mcp daemon [start|stop|status|restart|stop-all]`; the default is
  `status`.
- Does: manages the `mcpd` broker for the resolved socket; `stop` kills the pid in
  `<socket>.pid`; `stop-all` kills every `mcpd --socket` process and any server child left
  behind, and removes the `tbd-mcp` files under `XDG_RUNTIME_DIR` and `/tmp`.
- Exit codes: 0 done, or running; 1 stopped (for `status`), or a failed build or start; 2 an
  unknown action.
- Example: `cargo xtask mcp daemon start`

### selftest

- Synopsis: `cargo xtask mcp selftest`
- Does: builds `mcpd`, feeds `consume` the recorded transcripts in `tools/xtask/fixtures/mcp/`
  (success, JSON-RPC error, tool-reported error, failed initialize, empty), then drives `call`
  against the `mcpd` stub one-shot (success, error, failed initialize, empty with retry, timeout)
  and through a stub daemon, checking each exit code and stream. Every arm runs; no Workbench or
  network is needed.
- Exit codes: 0 every arm passed; 1 any arm failed.
- Example: `cargo xtask mcp selftest`

### smoke

- Synopsis: `cargo xtask mcp smoke`
- Does: runs `mcp call wb_connect` and `mcp call wb_state` against a running Workbench and
  requires a non-empty answer from each, trying both whatever the first gives.
- Exit codes: 0 both answered; 1 either failed.
- Example: `cargo xtask mcp smoke`

### wbcall

- Synopsis: `cargo xtask mcp wbcall <APIFunc> ['<json-object>'] [--timeout <s>]`; the timeout
  defaults to 600 s.
- Does: sends one request to the Workbench NET API at `ENFUSION_WORKBENCH_HOST` and
  `ENFUSION_WORKBENCH_PORT` (default 127.0.0.1:5775) over a fresh TCP connection, as length-prefixed
  strings: protocol version 1, client id, `JsonRPC`, and the JSON object with `APIFunc` added; it
  prints the response JSON. This reaches NET API handlers, such as the `EMCP_WB_TbdBlueprint`
  handler the blueprint pipeline drives, that the MCP server has no tool for.
- Exit codes: 0 the JSON printed; 1 no function name or arguments that are not JSON; 2 no
  connection; 3 a Workbench error or an unreadable response.
- Example: `cargo xtask mcp wbcall EMCP_WB_TbdBlueprint '{"action":"recon"}'`

### wb-logs

- Synopsis: `cargo xtask mcp wb-logs [--file <path>] [<pattern>]`; `cargo xtask mcp wb-logs
  --selftest`. The pattern filters only the printed extract.
- Does: grades a Workbench Play log: it needs `[TBD][` tagged lines, the mission-loaded and slot
  lines, and no compile, unknown-class or spawn errors, then passes when a player was assigned a
  slot. `--selftest` checks the verdict over fixture logs.
- Exit codes: 0 PASS; 1 FAIL; 2 PARTIAL (no player deployed yet); 3 ENVIRONMENT (no log, an
  unreadable log, an invalid pattern) and usage, `--help` included.
- Example: `cargo xtask mcp wb-logs --file console.log`

### consume, socket-send, probe-sock

- Synopsis: `cargo xtask mcp consume` (NDJSON on stdin); `cargo xtask mcp socket-send <socket>
  <tool> ['<json-args>']`; `cargo xtask mcp probe-sock <socket>`.
- Does: the pieces `call` pipes together. `consume` prints the id 2 result; `socket-send` writes
  `{"tool","args"}` to the broker's socket and prints its one response line, within
  `MCP_CALL_TIMEOUT`; `probe-sock` tries to connect for up to 2 s.
- Exit codes: `consume` 0 a result, 1 initialized with no result, 2 no initialize reply, 3 an
  error; `socket-send` 0 a response line, 7 the broker is unavailable or answered nothing;
  `probe-sock` 0 connectable, 1 not.
- Example: `cargo xtask mcp probe-sock "$XDG_RUNTIME_DIR/tbd-mcp-$(id -u).sock"`

## Boundaries

- Depends on: `process_runner` (`Run`, its detached spawn, `which`), `repository_layout` (the
  root walk, `MCP_TRANSCRIPT_FIXTURES_DIR`, the `dev-mcpd` build subfolder, the pinned entry module),
  `verification_core` (`lock::flock_exclusive`, `Pattern`, `gate::probe_str`), `clap`, `libc`
  (`getuid`), `serde_json` and `thiserror`; the `mcpd`
  binary of `tools/developer_tools/`, which this crate builds and starts; the `timeout`, `kill`,
  `pgrep` and `pkill` tools; a running Workbench with its NET API for `smoke`, `wbcall` and live
  calls.
- Used by:
  - the xtask binary: `tools/xtask/src/cli/` parses `McpCmd` and calls `run`;
  - `cargo xtask mod spawn-verify` and the spawn-determinism checks in
    `tools/checks/mod_script_checks/src/`, which run `mcp wb-logs` and `mcp call`;
  - `tools/commands/mod_operations/src/development_bootstrap.rs`, which starts the daemon through
    `daemon::start_at`;
  - `enfusion_mcp_broker`, which resolves its server child through `server_entrypoint`;
  - the operator steps `cargo xtask map export-terrain` prints, and people and agents driving
    Workbench.
- Rules: no tokio and no broker dependency, so xtask's dependency closure stays free of them
  (`cargo xtask verify crate-tiers`); `call`'s exit codes are the contract its callers read, and
  `selftest` pins every one of them against recorded transcripts and the stub; a missing
  transcript reads as an empty one, so a new arm adds its fixture in the same change; `wb-logs`
  returns 3 for usage and never a spawn verdict; its vocabulary and that of `mod remote-logs`
  change together.

## Related documentation

- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — the broker, the
  pinned server package and calling Workbench from the command line.
- [Enfusion MCP broker](/tools/enfusion/enfusion_mcp_broker/README.md) — the `mcpd` daemon this
  crate starts and talks to.
- [MCP transcript fixtures](/tools/xtask/fixtures/mcp/README.md) — the recorded replies the
  self-test replays.
- [Spawn determinism](/documentation/runbooks/spawn_determinism.md) — the Workbench gate that
  drives Play through `mcp call`.
