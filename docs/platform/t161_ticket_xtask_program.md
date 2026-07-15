# T-161 — Ticket CLI: Python → Rust xtask

**Status:** **SHIPPED** (T-161.0–.5) · **Worktree:** `.ai/artifacts/worktrees/TBD-T-161/` @
`t-161-ticket-xtask` · **Executor:** Cursor Grok 4.5 (user override of Claude Code lane)

## In one sentence

Replace the Python ticket pipeline (`scripts/lib/ticket_*.py` + bash `scripts/ticket`) with a
Cargo **`xtask`** binary so `./scripts/ticket …` / `make ticket-*` run on Rust — same CLI,
same derived docs, no Python runtime for everyday ticket ops.

## Locked decisions

| ID | Decision |
|----|----------|
| **D1** | Workspace member `xtask/`; `./scripts/ticket` → `cargo run -q -p xtask -- ticket …` |
| **D2** | CLI surface preserved (sync/check/brief/prompt/show/next/add/remove/reorder/ship/mark-ready/advance-slice/milestone/plan-batch/list/run/done/clean + queue helpers) |
| **D3** | Sync derived files byte-identical to Python oracle (G1 `cmp -s`) |
| **D4′** | `schema.json` documents shape; runtime check = hand-rolled Python rules (**no** JSON Schema crate) |
| **A4′** | `check` / `--strict` must match Python **exit + sorted ERROR set** (not exit 0 — registry debt T-147/148/149/154) |
| **D5** | Branch `t-161-ticket-xtask` / worktree `TBD-T-161` |
| **D6** | One-shot rewrite/backfill/seed scripts not ported |
| **D7** | Mod MCP Python out of scope |

## Slice plan (final)

| Slice | Status | Verify |
|-------|--------|--------|
| **T-161.0** | shipped | hub + registry + worktree |
| **T-161.1** | shipped | [`.ai/artifacts/t161_1_verify_log.md`](../../.ai/artifacts/t161_1_verify_log.md) |
| **T-161.2** | shipped | [`.ai/artifacts/t161_2_verify_log.md`](../../.ai/artifacts/t161_2_verify_log.md) |
| **T-161.3** | shipped | [`.ai/artifacts/t161_3_verify_log.md`](../../.ai/artifacts/t161_3_verify_log.md) |
| **T-161.4** | shipped | [`.ai/artifacts/t161_4_verify_log.md`](../../.ai/artifacts/t161_4_verify_log.md) |
| **T-161.5** | shipped | this hub + README + registry |

## How to run

```bash
./scripts/ticket sync
./scripts/ticket check [--strict]
./scripts/ticket brief T-161
cargo xtask ticket sync   # same binary via alias
```

## Out of scope

- Fixing T-147/148/149/154 registry debt (separate ticket)
- Map-assets Node / mod MCP Python
- Leptos / map-engine app code
