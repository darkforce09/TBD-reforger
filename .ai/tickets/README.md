# Ticket pipeline

**Source of truth:** [`registry.json`](registry.json) — never hand-edit [`queue.json`](queue.json) or `docs/TICKET_*.md` (generated).

**Implementation (T-161):** `./scripts/ticket` → Rust `xtask` (`cargo run -q -p xtask -- ticket …`).
No Python ticket libs remain (`ticket_registry.py` / `ticket_queue.py` deleted).

See [`AI_PLAYBOOK.md`](AI_PLAYBOOK.md) for operator recipes.

## KISS summary

1. **Composer 2.5 / Cursor** — edit `registry.json`, write specs, `./scripts/ticket sync`
2. **Mark ready** — `./scripts/ticket mark-ready T-068 path/to/t068_....md`
3. **Implement** — `./scripts/ticket run` (or `make tickets`)
4. **Merge** — human verifies branch
5. **Done** — `./scripts/ticket done T-068` (marks shipped + sync)
6. **Docs** — Cursor syncs narrative docs on `main`

## Commands

| Command | What it does |
|---------|----------------|
| `./scripts/ticket sync` | Regenerate all derived outputs |
| `./scripts/ticket check [--strict]` | Validate registry + outputs |
| `./scripts/ticket list` | Show dev queue (from registry) |
| `./scripts/ticket mark-ready ID [SPEC]` | Mark ready in registry + sync |
| `./scripts/ticket run` | Up to `batch_size` parallel Claude Code runs |
| `./scripts/ticket done ID` | Cleanup worktree + mark shipped |
| `./scripts/ticket brief ID` | Developer handoff card |
| `./scripts/ticket prompt ID [--slice SLICE]` | Print Claude Code prompt from slice spec |
| `./scripts/ticket show ID` | One ticket card |
| `./scripts/ticket next` | Active slice + next queued |

## Makefile

```bash
make ticket-sync
make ticket-check
make ticket-check-strict
make tickets          # alias for ./scripts/ticket run
make ticket-list
```

## Status values

| Status | Meaning |
|--------|---------|
| `idea` | Brainstorm pool — no order |
| `queued` | Backlog — has order |
| `ready` | Spec on `main` — OK to `run` |
| `running` | Claude Code working |
| `review` | Branch ready for you |
| `shipped` | Done |
| `deferred` | Deprioritized |
| `cancelled` | Dropped — row kept |

## Logs

`artifacts/ticket-pipeline/T-0xx/run.log`

## Authority

[`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md) · Hub [`docs/platform/t161_ticket_xtask_program.md`](../../docs/platform/t161_ticket_xtask_program.md)
