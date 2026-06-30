# Workbench MCP shell tooling

**Shipped:** `e7e7232` (2026-06-30) · **Pinned package:** `enfusion-mcp@0.6.1` in `scripts/mod/package.json`  
**Entry for agents:** [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md) · **Bootstrap:** `bash scripts/mod/tbd-dev-bootstrap.sh`

Reliable shell access to **enfusion-mcp** for Claude Code terminal sessions. Replaces the old flaky one-shot `timeout 90 npx …` path that hung to the full timeout or returned empty mid-stream.

---

## Architecture

```text
mcp-call.sh
  ├─ (default) warm daemon  →  AF_UNIX socket  →  lib/mcp-daemon.mjs  →  one enfusion-mcp child
  └─ fallback one-shot      →  node …/dist/index.js  →  lib/mcp-consume.py (early exit on id==2)
```

| Component | Path | Role |
|-----------|------|------|
| Call wrapper | `scripts/mod/mcp-call.sh` | Daemon-first; one-shot fallback; exports all three `ENFUSION_*` paths |
| JSON-RPC consumer | `scripts/mod/lib/mcp-consume.py` | Shared parser + exit-code contract (daemon, one-shot, self-test) |
| Daemon broker | `scripts/mod/lib/mcp-daemon.mjs` | One index load (~35 s cold); serializes `tools/call` |
| Daemon control | `scripts/mod/mcp-daemon.sh` | `start` · `stop` · `status` · `restart` · **`stop-all`** |
| Socket client | `scripts/mod/lib/mcp-socket-send.py` | Sends framed requests to the daemon |
| Offline gates | `scripts/mod/mcp-call-selftest.sh` | 19 fixture tests, no Workbench |
| Live smoke | `scripts/mod/mcp-smoke.sh` | `wb_connect` + `wb_state` after bootstrap |

**Bootstrap** (`tbd-dev-bootstrap.sh`) runs `npm ci` in `scripts/mod/` when needed, pre-warms the daemon, then `wb_connect` + `mod_validate`.

---

## Usage

```bash
bash scripts/mod/mcp-call.sh <tool> '<json-args>'   # args default to {}
bash scripts/mod/mcp-daemon.sh status
bash scripts/mod/mcp-daemon.sh stop-all             # nuke every stray broker + orphaned server
bash scripts/mod/mcp-call-selftest.sh               # offline — no Workbench
bash scripts/mod/mcp-smoke.sh                       # live — Workbench Net API up
```

**Examples:**

```bash
bash scripts/mod/mcp-call.sh wb_connect '{}'
bash scripts/mod/mcp-call.sh wb_state '{}'
bash scripts/mod/mcp-call.sh api_search '{"query":"GetWorldBounds"}'
bash scripts/mod/mcp-call.sh mod_validate '{"modPath":"'"$PWD"'/apps/mod/tbd-framework"}'
```

Warm daemon calls return in **~0.3 s**. First call (or after daemon idle/max-life) pays the **~35 s** one-time 8,693-class index load once, then stays warm.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success — non-empty stdout |
| 1 | Usage error, or empty stdout after all retries |
| 2 | MCP init failed (no valid id=1 / no JSON-RPC stream) |
| 3 | Tool error — JSON-RPC `"error"` **or** `result.isError:true` |
| 4 | Timeout (`timeout` exit 124) |

Use **`MCP_DEBUG=1`** to print runner tier + captured stderr on failure.

---

## Environment

| Variable | Default | Purpose |
|----------|---------|---------|
| `ENFUSION_GAME_PATH` | `~/.cache/enfusion-mcp-root` | Pak symlink farm (see `setup-mcp-game-root.sh`) |
| `ENFUSION_WORKBENCH_PATH` | Steam Arma Reforger Tools path | Workbench install |
| `ENFUSION_PROJECT_PATH` | Workbench addons folder | Project/addon discovery |
| `ENFUSION_MCP_BIN` | (auto) | Override enfusion-mcp entry |
| `MCP_SOCK` | `$XDG_RUNTIME_DIR/tbd-mcp-<uid>.sock` | Daemon socket |
| `MCP_CALL_TIMEOUT` | `180` | Per-call hard ceiling (seconds) |
| `MCP_CALL_RETRIES` | `1` | Re-run once on empty stdout (not on exit 3) |
| `MCP_NO_DAEMON` | `0` | Force one-shot path |
| `MCP_DAEMON_IDLE` | `1800` | Daemon self-exit after idle (seconds; `0` = no idle timer) |
| `MCP_DAEMON_MAX_LIFE` | `14400` | Hard max daemon lifetime (4 h) — fires even if `IDLE=0` |

---

## Binary resolution (4 tiers)

1. `ENFUSION_MCP_BIN` if set and file exists  
2. `scripts/mod/node_modules/enfusion-mcp/dist/index.js` (after `npm ci`)  
3. First hit under `~/.npm/_npx/**/enfusion-mcp/dist/index.js`  
4. `npx -y enfusion-mcp` (offline/cache-missing fallback)

Install pinned deps: `(cd scripts/mod && npm ci)`.

---

## Root causes fixed @ `e7e7232`

### 1. Bash brace bug (primary flake during T-090.3.0 spike)

`ARGS="${2:-{}}"` is **wrong** — bash matches the first `}` to close the expansion, appending a literal trailing `}` to every args-bearing call. Example: `{"query":"X"}` became `{"query":"X"}}` → corrupt JSON → tools silently received `{}`. That is why args-bearing MCP calls were flaky during the spike and raw hand-rolled JSON-RPC was used as a workaround.

**Fix:** `ARGS="${2:-}"` then `[ -z "$ARGS" ] && ARGS='{}'`. Regression gate: self-test **T7 args round-trip**.

### 2. One-shot hung to full timeout

The stdio MCP server does not exit on stdin EOF. The old consumer looped until EOF, so `timeout 90` always waited ~90 s (or SIGKILL mid-response → empty stdout).

**Fix:** `mcp-consume.py` calls `sys.exit(0)` immediately after printing id==2 → pipe closes → server gets SIGPIPE → returns at response time. One-shot path also uses a generous `MCP_CALL_TIMEOUT` backstop.

### 3. Tool errors misclassified

enfusion-mcp returns some failures as `result.isError:true` (not a top-level JSON-RPC `"error"`).

**Fix:** both shapes map to **exit 3** with stderr detail — no retry as “empty”.

### 4. Missing path exports

Old `mcp-call.sh` only exported `ENFUSION_GAME_PATH`. `wb_*` tools need all three paths (now mirrored from bootstrap).

---

## Safeguards against load / leak

| Safeguard | Behavior |
|-----------|----------|
| Daemon idle timeout | Exits after 30 min idle (configurable) |
| Daemon max-life | Hard 4 h cap — restarts transparently on next call |
| One-shot fail-fast | Early consumer exit; timeout is a ceiling, not the common path |
| `mcp-daemon.sh stop-all` | Kills all brokers, reaps orphaned `enfusion-mcp` children, clears sockets |
| Self-test cleanup | Short idle in tests; verifies zero stray processes after run |
| `.gitignore` | `scripts/mod/node_modules/` — never commit npm tree |

**If load spikes:** run `bash scripts/mod/mcp-daemon.sh stop-all` and confirm no `enfusion-mcp` / `mcp-daemon.mjs` processes remain.

---

## Verification gates

| Gate | Command | PASS |
|------|---------|------|
| **T1–T7** (offline) | `bash scripts/mod/mcp-call-selftest.sh` | exit 0, 19/19 |
| **S1** (live) | `bash scripts/mod/mcp-smoke.sh` | exit 0 after bootstrap |
| **S2** | `time bash scripts/mod/mcp-call.sh wb_state '{}'` ×3 | exit 0, ~response time (not ~180 s) |
| **S5** | `bash scripts/mod/mcp-call.sh totally_fake_tool '{}'` | exit 3 |

---

## Related tickets

- **T-121** item 3 (MCP helper hardening) — **shipped** @ `e7e7232`; remaining T-121 scope: Arland DEM re-export, optional game-mode fallback.
- **T-090.3.0** spike — enumeration proven; some probe calls used raw JSON-RPC before this fix (see spike spec lessons).
