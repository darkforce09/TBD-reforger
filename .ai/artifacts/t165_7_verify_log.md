# T-165.7 verify log — MCP broker + stub → Rust

Scope: `mcpd` (tools/tbd-tools) replaces `scripts/mod/lib/mcp-daemon.mjs` (165-LOC broker) +
`scripts/mod/fixtures/stub-runner.mjs` (57-LOC stub); runner resolution fixed so native
binaries exec directly. The socket CLIENT was already Rust (`xtask mcp socket-send|consume`,
T-162) — this closes the server half.

## Ported

- **Broker** (`mcpd --socket <path> [--pidfile <path>]`): tokio UnixListener; ONE
  enfusion-mcp child spawned + initialized once (id 1 + `notifications/initialized`);
  requests over the socket as newline-JSON `{tool, args}`; serialized `tools/call` queue
  (tokio Mutex — the Workbench NetAPI is single-stream); responses matched by id and
  **relabelled to id==2** so `xtask mcp consume` parses them unchanged; per-call timeout →
  `-32001 tool timeout`, child-gone → `-32000`; auto-restart on child exit (restart failure
  → exit 1); idle reaper (`MCP_DAEMON_IDLE`, 1800 s default) + hard max-life backstop
  (`MCP_DAEMON_MAX_LIFE`, 4 h) with socket+pidfile unlink; SIGTERM/SIGINT clean shutdown;
  `MCP_DEBUG=1` stderr logging. Async type cycle (start_child ↔ on_child_exit) broken with
  a `BoxFuture`.
- **Stub** (`mcpd --stub` or env `MCP_STUB=1`): STUB_MODE success|error|empty|initfail,
  STUB_DAEMON request/response mode, STUB_LINGER; byte-exact strings
  (`STUB-OK wb_state edit 123`, `STUB-DAEMON-OK <tool> args=<json>`,
  `STUB-STDERR-MARKER mode=<mode>`). **Mode precedence: broker wins whenever `--socket` is
  present** — the selftest exports MCP_STUB=1 around `mcp-daemon.sh start` so the broker's
  argv-less child resolves to the stub, while the broker itself (launched with `--socket`)
  never can; recursion is structurally impossible.

## Runner-resolution fix (the plan's trap item)

Every tier used to hardcode `node <entry>`. Now: `.js`/`.mjs` entries → `node <entry>`;
anything else execs directly — in `mcp-call.sh` `resolve_runner()` tier1 and in `mcpd`'s own
child spawn. `mcp-daemon.sh` launches the built broker binary via new
`scripts/mod/lib/mcpd-bin.sh` (cached `cargo build`, direct `setsid "$mcpd_bin" --socket …`
— no node/cargo wrapper in argv), and `stop_all`'s pkill pattern targets `mcpd --socket`
(the orphaned-server reaper keeps the node_modules pattern — the real enfusion-mcp child
stays Node by design).

## Acceptance

- **`mcp-call-selftest.sh`: ALL PASS (19)** — assertions unchanged: consumer fixtures
  T2/T3/T3b/T4/T5 (rc 0/3/3/2/1), T6 usage rc 1, one-shot stub success/error/init-fail/
  empty+retry/timeout (rc 0/3/2/1/4, `STUB-OK wb_state edit 123` byte-exact, T7 stderr
  surfaced), daemon start+status rc 0, daemon call
  (`STUB-DAEMON-OK wb_state args={}`), args round-trip
  (`STUB-DAEMON-OK api_search args={"query":"Ztest"}` — the ${2:-{}} brace-corruption
  regression guard), fallback-when-no-daemon rc 0. Re-run green AFTER the .mjs deletions.
- **tier1 .js branch probe** (pre-delete): `ENFUSION_MCP_BIN=fixtures/stub-runner.mjs`
  one-shot → `STUB-OK wb_state edit 123` rc=0 — the node lane the real
  `enfusion-mcp/dist/index.js` rides is intact.
- **Idle self-reap probe**: `MCP_DAEMON_IDLE=2` broker → status `running` → self-terminated
  → status `stopped`, socket unlinked, `pgrep -f 'mcpd --socket'` empty (zero strays).
- `cargo clippy -p tbd-tools --all-targets -- -D warnings` rc=0 · fmt clean ·
  `./scripts/ticket check` OK.

## Deleted

`scripts/mod/lib/mcp-daemon.mjs`, `scripts/mod/fixtures/stub-runner.mjs` (both spawner-free
after the rewires; stale-ref sweep clean). Tracked non-mod `.mjs`: 43 → 41. Node's remaining
role in the MCP lane = the `enfusion-mcp` server itself (the permitted floor).

## Note

The real-server lane (`mcp-daemon.sh start` against the actual enfusion-mcp + Workbench)
is untouched in shape — same socket protocol, same consume path, same env plumbing — and
gets its live proof the next time the operator drives a Workbench session; the broker
mechanics (spawn/init/serialize/remap/reap) are fully exercised offline by the stub daemon.
