# Ticket pipeline

**Source of truth:** one `T-*.toml` file per parent and per child, plus the [`ROOT`](ROOT) marker. Files are encoding C (flat `status` + sibling `order`, `[scope.*]` tables, `kind = "program"|"work"`). Never hand-edit what `cargo xtask ticket sync` writes: `queue.json`, the roadmap's recommended-next-work block, and the gap-analysis ticket column.

**Implementation:** the `ticket` subcommand of `xtask`, backed by the `ticket-engine` crate at
`tools_v2/ticket-engine/`. Every verb below is a `cargo xtask ticket …` call.

**Work model (locked):** all ticket work lands on **`main`** — no `ticket/T-0xx` branches or worktrees as the default. See root [`CLAUDE.md`](../../CLAUDE.md).

See [`AI_PLAYBOOK.md`](AI_PLAYBOOK.md) for operator recipes.

## KISS summary

1. **Composer 2.5 / Cursor** — edit the relevant `T-*.toml`, write specs, `cargo xtask ticket sync`
2. **Mark ready** — `cargo xtask ticket mark-ready T-068 path/to/t068_....md`
3. **Implement** — `cargo xtask ticket run` on **`main`**
4. **Verify** — human checks gates / smoke
5. **Done** — `cargo xtask ticket done T-068` (marks shipped + sync)
6. **Docs** — Cursor syncs narrative docs on `main`

## Commands

| Command | What it does |
|---------|----------------|
| `cargo xtask ticket sync` | Regenerate all derived outputs |
| `cargo xtask ticket check [--strict]` | Validate the ticket files + generated outputs |
| `cargo xtask ticket list` | Show the dev queue |
| `cargo xtask ticket mark-ready ID [SPEC]` | Mark ready in the ticket file + sync |
| `cargo xtask ticket run` | Up to `batch_size` Claude Code runs (`claude-code` slices) |
| `cargo xtask ticket done ID` | Mark shipped + sync |
| `cargo xtask ticket brief ID` | Developer handoff card |
| `cargo xtask ticket prompt ID [--slice SLICE]` | Print Claude Code prompt from slice spec |
| `cargo xtask ticket show ID` | One ticket card |
| `cargo xtask ticket next` | Active slice + next queued |

`cargo xtask ticket --help` lists the full verb set, including the reporting queries.

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
| `cancelled` | Dropped — the ticket file stays |

## Logs

`.ai/artifacts/ticket-pipeline/T-0xx/run.log`

## Authority

[ticketboard](/apps/ticketboard/README.md) · Hub [`docs/platform/t161_ticket_xtask_program.md`](../../docs/platform/t161_ticket_xtask_program.md)
