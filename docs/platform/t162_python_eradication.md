# T-162 — Eradicate all remaining Python (→ Rust)

**Status:** **SHIPPED** (T-162.0–.4) · **Worktree:** `.ai/artifacts/worktrees/TBD-T-161/` @
`t-161-ticket-xtask` · **Executor:** Cursor Grok 4.5 (user override)

## In one sentence

Delete every remaining `.py` file and every Python interpreter invocation under
`scripts/`, porting live helpers into the T-161 **`xtask`** crate so monorepo
tooling is Rust-only.

## Supersedes T-161 D6/D7

T-161 left one-shot migration scripts and mod MCP Python out of scope. **T-162
revoked that** — deleted or ported.

## Shipped inventory

### Ported → `cargo xtask`

| Former path | xtask command |
|-------------|---------------|
| `scripts/mod/lib/mcp-consume.py` | `mcp consume` |
| `scripts/mod/lib/mcp-socket-send.py` | `mcp socket-send` |
| `mcp-daemon.sh` AF_UNIX probe | `mcp probe-sock` |
| `debug-direct-join.sh` A2S + NDJSON | `debug a2s-probe` / `debug direct-join-log` |
| `mission-version-upload-repro.sh` | `repro mission-id` / `repro mission-version-body` |

### Deleted

- `scripts/lib/seed_registry.py`
- `scripts/backfill-registry-monorepo.py`
- `scripts/rewrite-doc-links.py`
- `scripts/rewrite-ticket-paths.py`

## Slice plan (final)

| Slice | Status | Notes |
|-------|--------|-------|
| **T-162.0** | shipped | Hub + registry |
| **T-162.1** | shipped | MCP Rust + selftest 19/19 |
| **T-162.2** | shipped | One-shots deleted |
| **T-162.3** | shipped | Inline python removed |
| **T-162.4** | shipped | `verify-no-python.sh` + docs |

## Hard gate

```bash
make verify-no-python
bash scripts/mod/mcp-call-selftest.sh
cargo clippy -p xtask -- -D warnings
```

Verify: [`.ai/artifacts/t162_verify_log.md`](../../.ai/artifacts/t162_verify_log.md)

## Out of scope

- Node `mcp-daemon.mjs` / enfusion-mcp (not Python)
- Historical “Python” mentions in old verify logs
- Fixing T-147/148/149/154 registry debt
