#!/usr/bin/env node
// Persistent enfusion-mcp broker (T-090.0).
//
// Spawns ONE enfusion-mcp child, initializes it once (paying the ~35 s index load a single
// time), and serves tools/call requests over an AF_UNIX socket — so repeated mcp-call.sh
// invocations reuse the warm server instead of re-paying init per call. Requests are
// serialized (the Workbench NetAPI is single-stream); responses are matched by id and
// relabelled to id==2 so scripts/mod/lib/mcp-consume.py parses them unchanged.
//
// Args: --socket <path> [--pidfile <path>]
// Env:  ENFUSION_MCP_BIN (resolved by mcp-daemon.sh), MCP_DAEMON_IDLE (s, default 1800; 0=never),
//       MCP_CALL_TIMEOUT (s, default 180), MCP_DEBUG=1.
import net from "node:net";
import { spawn } from "node:child_process";
import { existsSync, unlinkSync, writeFileSync } from "node:fs";
import readline from "node:readline";

const argv = process.argv.slice(2);
const getArg = (k, d) => {
  const i = argv.indexOf(k);
  return i >= 0 && i + 1 < argv.length ? argv[i + 1] : d;
};
const SOCK = getArg("--socket", process.env.MCP_SOCK);
if (!SOCK) {
  console.error("mcp-daemon: --socket required");
  process.exit(2);
}
const PIDFILE = getArg("--pidfile", SOCK + ".pid");
const IDLE_MS = Number(process.env.MCP_DAEMON_IDLE ?? "1800") * 1000;
// Hard backstop: the daemon self-terminates after this even if "busy" or started with IDLE=0, so it can
// never linger/leak indefinitely. The next mcp-call.sh transparently restarts it. 0 = disabled.
const MAX_LIFE_MS = Number(process.env.MCP_DAEMON_MAX_LIFE ?? "14400") * 1000;
const CALL_MS = Number(process.env.MCP_CALL_TIMEOUT ?? "180") * 1000;

const log = (...a) => {
  if (process.env.MCP_DEBUG === "1") console.error("[mcp-daemon]", ...a);
};

function resolveRunner() {
  const bin = process.env.ENFUSION_MCP_BIN;
  if (bin && existsSync(bin)) return ["node", bin];
  const pinned = new URL("../node_modules/enfusion-mcp/dist/index.js", import.meta.url).pathname;
  if (existsSync(pinned)) return ["node", pinned];
  return ["npx", "-y", "enfusion-mcp"];
}
const RUNNER = resolveRunner();

let child = null;
let server = null;
let idleTimer = null;
let nextId = 100;
const pending = new Map(); // id -> { resolve, reject }

function resetIdle() {
  if (idleTimer) clearTimeout(idleTimer);
  if (IDLE_MS > 0) idleTimer = setTimeout(() => { log("idle timeout"); shutdown(0); }, IDLE_MS);
}

function shutdown(code) {
  try { child?.kill(); } catch {}
  try { server?.close(); } catch {}
  for (const p of [SOCK, PIDFILE]) {
    try { if (existsSync(p)) unlinkSync(p); } catch {}
  }
  process.exit(code);
}
process.on("SIGTERM", () => shutdown(0));
process.on("SIGINT", () => shutdown(0));

function startChild() {
  return new Promise((resolve, reject) => {
    log("spawning", RUNNER.join(" "));
    child = spawn(RUNNER[0], RUNNER.slice(1), { stdio: ["pipe", "pipe", "pipe"], env: process.env });
    child.on("error", (e) => reject(e));
    child.stderr.on("data", (d) => log("child stderr:", String(d).trim()));
    child.on("exit", (c) => { log("child exited", c); onChildExit(); });

    const rl = readline.createInterface({ input: child.stdout });
    rl.on("line", (line) => {
      line = line.trim();
      if (!line) return;
      let o;
      try { o = JSON.parse(line); } catch { return; }
      const p = pending.get(o.id);
      if (p) { pending.delete(o.id); p.resolve(o); }
    });

    const initId = 1;
    let done = false;
    pending.set(initId, { resolve: () => { done = true; resolve(); }, reject });
    child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id: initId, method: "initialize", params: { protocolVersion: "2024-11-05", capabilities: {}, clientInfo: { name: "tbd-daemon", version: "1.0" } } }) + "\n");
    child.stdin.write(JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) + "\n");
    setTimeout(() => { if (!done) { pending.delete(initId); reject(new Error("init timeout")); } }, CALL_MS);
  });
}

let restarting = false;
function onChildExit() {
  for (const [, p] of pending) p.reject(new Error("child exited"));
  pending.clear();
  child = null;
  if (restarting) return;
  restarting = true;
  startChild()
    .then(() => { restarting = false; log("child restarted"); })
    .catch((e) => { log("restart failed", e.message); shutdown(1); });
}

// Serialized tool call. Always resolves to a JSON-RPC object with id==2.
let queue = Promise.resolve();
function callTool(tool, args) {
  const run = () =>
    new Promise((resolve) => {
      if (!child) {
        resolve({ jsonrpc: "2.0", id: 2, error: { code: -32000, message: "daemon child unavailable" } });
        return;
      }
      const id = ++nextId;
      log("→child", id, "name=" + tool, "args=" + JSON.stringify(args));
      const finish = (obj) => { obj.id = 2; resolve(obj); };
      pending.set(id, {
        resolve: finish,
        reject: () => finish({ jsonrpc: "2.0", error: { code: -32000, message: "child error" } }),
      });
      child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method: "tools/call", params: { name: tool, arguments: args || {} } }) + "\n");
      setTimeout(() => {
        if (pending.has(id)) { pending.delete(id); finish({ jsonrpc: "2.0", error: { code: -32001, message: "tool timeout" } }); }
      }, CALL_MS);
    });
  const p = queue.then(run);
  queue = p.catch(() => {});
  return p;
}

function startServer() {
  if (existsSync(SOCK)) { try { unlinkSync(SOCK); } catch {} } // stale
  server = net.createServer((conn) => {
    resetIdle();
    let buf = "";
    conn.on("error", () => {});
    conn.on("data", (d) => {
      buf += d.toString();
      const nl = buf.indexOf("\n");
      if (nl < 0) return;
      let req;
      try { req = JSON.parse(buf.slice(0, nl)); } catch { conn.end(); return; }
      callTool(req.tool, req.args).then((resp) => {
        try { conn.write(JSON.stringify(resp) + "\n"); } catch {}
        conn.end();
        resetIdle();
      });
    });
  });
  server.on("error", (e) => { console.error("mcp-daemon: server error", e.message); shutdown(1); });
  server.listen(SOCK, () => {
    log("listening on", SOCK);
    try { writeFileSync(PIDFILE, String(process.pid)); } catch {}
    resetIdle();
    if (MAX_LIFE_MS > 0) setTimeout(() => { log("max lifetime reached"); shutdown(0); }, MAX_LIFE_MS);
  });
}

startChild()
  .then(startServer)
  .catch((e) => { console.error("mcp-daemon: child init failed:", e.message); shutdown(2); });
