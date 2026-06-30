// Offline test stub for the enfusion-mcp server (T-090.0 self-test).
//
// Emulates the server's stdout (newline-delimited JSON-RPC) without a Workbench or the
// ~35 s index load, so mcp-call.sh (one-shot) and mcp-daemon.mjs can be exercised
// deterministically. Run as the resolved RUNNER via `ENFUSION_MCP_BIN=<this file>`.
//
//   STUB_MODE    success | error | empty | initfail   (default success)
//   STUB_DAEMON  1 → request/response server (replies to each id; for daemon tests)
//                else → one-shot: emit id=1 + id=2 once, then linger
//   STUB_LINGER  seconds to stay alive in one-shot mode (default 1; mimics "no exit on EOF")
//
// Always writes a stderr marker so the stderr-surfacing gate (T7) is observable.
import readline from "node:readline";

const mode = process.env.STUB_MODE || "success";
const linger = Number(process.env.STUB_LINGER || "1");
const emit = (obj) => process.stdout.write(JSON.stringify(obj) + "\n");

process.stderr.write(`STUB-STDERR-MARKER mode=${mode}\n`);
process.stdout.on("error", () => process.exit(0));

if (process.env.STUB_DAEMON === "1") {
  // Request/response server: reply to each request with a matching id (for daemon broker tests).
  const rl = readline.createInterface({ input: process.stdin });
  rl.on("line", (line) => {
    line = line.trim();
    if (!line) return;
    let o;
    try {
      o = JSON.parse(line);
    } catch {
      return;
    }
    if (o.method === "initialize") {
      emit({ jsonrpc: "2.0", id: o.id, result: { protocolVersion: "2024-11-05", capabilities: {}, serverInfo: { name: "stub", version: "0" } } });
    } else if (o.method === "tools/call") {
      const name = o.params && o.params.name;
      const args = JSON.stringify((o.params && o.params.arguments) || {});
      if (mode === "error") emit({ jsonrpc: "2.0", id: o.id, error: { code: -32601, message: "Stub error: " + name } });
      else emit({ jsonrpc: "2.0", id: o.id, result: { content: [{ type: "text", text: "STUB-DAEMON-OK " + name + " args=" + args }] } });
    }
    // notifications/* ignored
  });
} else {
  // One-shot mode: drain stdin, emit once, linger like the real server.
  process.stdin.resume();
  process.stdin.on("data", () => {});
  if (mode !== "initfail") {
    emit({ jsonrpc: "2.0", id: 1, result: { protocolVersion: "2024-11-05", capabilities: {}, serverInfo: { name: "stub", version: "0" } } });
  }
  if (mode === "success") {
    emit({ jsonrpc: "2.0", id: 2, result: { content: [{ type: "text", text: "STUB-OK wb_state edit 123" }] } });
  } else if (mode === "error") {
    emit({ jsonrpc: "2.0", id: 2, error: { code: -32601, message: "Stub error: unknown tool" } });
  }
  setTimeout(() => process.exit(0), Math.max(0, linger) * 1000);
}
