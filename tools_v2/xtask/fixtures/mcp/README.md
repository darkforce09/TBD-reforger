# MCP transcript fixtures

Recorded responses of the enfusion-mcp server, one file per response shape, that
`cargo xtask mcp selftest` replays to pin the exit code each shape produces without a running
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench).

## Contents

```text
tools_v2/xtask/fixtures/mcp/
├── mcp-empty.jsonl             an initialize reply and nothing after it: exit 1, empty stdout
├── mcp-init-fail.jsonl         a non-JSON line and a notification with no id: exit 2
├── mcp-tool-error.jsonl        a JSON-RPC error for the tool call: exit 3, the error JSON on stderr
├── mcp-tool-iserror.jsonl      a tool result flagged `isError`: exit 3, the message on stderr
└── mcp-wb-state-success.jsonl  a `wb_state` result with text content: exit 0, non-empty stdout
```

## How it works

`cargo xtask mcp selftest` (`tools_v2/xtask/src/commands/mcp/call_selftest.rs`) joins
`MCP_TRANSCRIPT_FIXTURES_DIR` from `tools_v2/xtask/src/core/repository_layout.rs` onto the
checkout root, runs `cargo xtask mcp consume` once per file with the file on stdin, and checks the
exit code and output against the expectation in each Contents role. `mcp consume` prints the
result of the message whose `id` is 2, the tool call that follows the `id` 1 initialize reply. The
selftest then drives `cargo xtask mcp call` against the `mcpd` stub for the same codes end to end;
that half reads no file here.

## Format

- Encoding: UTF-8 newline-delimited JSON-RPC 2.0, one message per line, as enfusion-mcp writes
  to stdout; `mcp-init-fail.jsonl` also holds one line that is not JSON on purpose. Files are
  named `mcp-<response shape>.jsonl`.
- Schema: none in `contracts_v2/`. The shape is JSON-RPC 2.0 with MCP's `initialize` result
  (`protocolVersion`, `capabilities`, `serverInfo`) as `id` 1 and a `tools/call` result
  (`content`, optional `isError`) or `error` object as `id` 2.
- Adding a file: record the server's stdout for the new shape, add the file here, add its arm to
  `run_at` in `call_selftest.rs` with the exit code it must produce, then run
  `cargo xtask mcp selftest`.

## Producers and consumers

- Producers: people, from recorded enfusion-mcp output; no command writes these files.
- Consumers: `cargo xtask mcp selftest`, which names each file in
  `tools_v2/xtask/src/commands/mcp/call_selftest.rs`.

## Boundaries

- Depends on: the exit-code contract of `cargo xtask mcp consume` (0 success, 1 empty,
  2 initialize failed, 3 tool error), declared on `McpCmd` in
  `tools_v2/xtask/src/commands/mcp/cli.rs`.
- Used by: `cargo xtask mcp selftest` alone.
- Rules: a file keeps its name, because the selftest names each one; `mcp selftest` must exit 0
  after any change here. A missing file reads as an empty transcript (`xtask_consume` falls back
  to an empty body), so a rename without the matching selftest edit can still pass the empty arm.

## Related documentation

- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the broker, the
  call path and the selftest.
