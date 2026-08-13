# Ticket pipeline

**Source of truth:** one `T-*.toml` file per parent and per child, plus the [`ROOT`](ROOT) marker. Files are encoding C (flat `status` + sibling `order`, `[scope.*]` tables, `kind = "program"|"work"`). Never hand-edit generated `docs/TICKET_*.md`.

**Monolith cutover:** last commit that contained `.ai/tickets/registry.json` is `5035931ce80324db81d84fb9535433689d72f208` (`git show 5035931ce80324db81d84fb9535433689d72f208:.ai/tickets/registry.json`).

**Implementation (T-161 / T-883):** `cargo run -q -p xtask -- ticket …` (shell shim deleted).
No Python ticket libs remain.

**Work model (locked):** all ticket work lands on **`main`** — no `ticket/T-0xx` branches or worktrees as the default. See root [`CLAUDE.md`](../../CLAUDE.md).

See [`AI_PLAYBOOK.md`](AI_PLAYBOOK.md) for operator recipes.

## KISS summary

1. **Composer 2.5 / Cursor** — edit the relevant `T-*.toml`, write specs, `cargo run -q -p xtask -- ticket sync`
2. **Mark ready** — `cargo run -q -p xtask -- ticket mark-ready T-068 path/to/t068_....md`
3. **Implement** — `cargo run -q -p xtask -- ticket run` (or `cargo xtask ticket run`) on **`main`**
4. **Verify** — human checks gates / smoke
5. **Done** — `cargo run -q -p xtask -- ticket done T-068` (marks shipped + sync)
6. **Docs** — Cursor syncs narrative docs on `main`

## Commands

| Command | What it does |
|---------|----------------|
| `cargo run -q -p xtask -- ticket sync` | Regenerate all derived outputs |
| `cargo run -q -p xtask -- ticket check [--strict]` | Validate registry + outputs |
| `cargo run -q -p xtask -- ticket list` | Show dev queue (from registry) |
| `cargo run -q -p xtask -- ticket mark-ready ID [SPEC]` | Mark ready in registry + sync |
| `cargo run -q -p xtask -- ticket run` | Up to `batch_size` Claude Code runs (`claude-code` slices) |
| `cargo run -q -p xtask -- ticket done ID` | Mark shipped + sync |
| `cargo run -q -p xtask -- ticket brief ID` | Developer handoff card |
| `cargo run -q -p xtask -- ticket prompt ID [--slice SLICE]` | Print Claude Code prompt from slice spec |
| `cargo run -q -p xtask -- ticket show ID` | One ticket card |
| `cargo run -q -p xtask -- ticket next` | Active slice + next queued |

## Makefile

```bash
cargo xtask ticket sync
cargo xtask ticket check
cargo xtask ticket check --strict
cargo xtask ticket run          # alias for cargo xtask ticket run
cargo xtask ticket list
```

## Status values

| Status | Meaning |
|--------|---------|
| `idea` | Brainstorm pool — no order |
| `queued` | Backlog — has order |
| `ready` | Spec on `main` — OK to `run` |
| `running` | Claude Code working |
| `review` | Ready for human verify |
| `shipped` | Done |
| `deferred` | Deprioritized |
| `cancelled` | Dropped — row kept |

## Logs

`.ai/artifacts/ticket-pipeline/T-0xx/run.log`

## Authority

[`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md) · Hub [`docs/platform/t161_ticket_xtask_program.md`](../../docs/platform/t161_ticket_xtask_program.md)
