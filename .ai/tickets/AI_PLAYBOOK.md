# Ticket AI Playbook

**Audience:** Cursor (Composer 2.5) — ticket operator. **Source of truth:** [`registry.json`](registry.json).

## Golden rule

Edit **`tickets/registry.json`** → run **`./scripts/ticket sync`** → run **`./scripts/ticket check`** → commit registry + all generated files together.

Never hand-edit: `queue.json`, `docs/TICKET_*.md`, CLAUDE marker block, MC ROADMAP marker block, gap_analysis ticket column.

## Status lifecycle

| You say | Registry change |
|---------|-----------------|
| Add idea | `status: idea`, full context fields, **no order** |
| Promote to backlog | `status: queued`, assign `order` |
| Write spec | Create `tNNN_*.md`, set `spec`, `status: ready` |
| T-067.0 done | Update `active_slice` to T-067.1 or `status: shipped` |
| Ship T-066 | `status: shipped`, clear `active_slice`, sync |
| Cancel T-078 | `status: cancelled` — never delete row |
| Reorder wiki first | Lower T-085 `order` below T-086 |

## Recipes

### Ship a ticket

1. Human verified merge + build/lint pass
2. Set row `status: shipped`, remove `active_slice`
3. `./scripts/ticket sync`
4. Update narrative docs per [`docs/AGENT_COMMIT_CHECKLIST.md`](../docs/AGENT_COMMIT_CHECKLIST.md)

### Mark ready for Claude Code

```bash
./scripts/ticket mark-ready T-068 docs/specs/Mission_Creator_Architecture/t068_asset_registry.md
./scripts/ticket run
```

### Brainstorm (speech-to-text friendly)

1. `./scripts/ticket add "Outliner search" --program eden --surfaces LEFT --impact ui`
2. Review `docs/TICKET_BRAINSTORM.md`
3. When promoted: assign `order`, write spec, `mark-ready`

### Developer brief

```bash
./scripts/ticket brief T-067
```

## Executor gate

**CRITICAL:** `./scripts/ticket run` only executes slices with `executor: claude-code`. Rows with `workbench`, `human`, `cursor-docs`, or `ci` are skipped or handled by the matching agent.

| Executor | Agent | Scope |
|----------|-------|-------|
| `claude-code` | Claude Code | `website/` code on `ticket/T-0xx` branches |
| `cursor-docs` | Cursor | specs, registry, `./scripts/ticket sync` |
| `workbench` / `human` | Human | `mod/tbd-framework` — see [`docs/TICKET_MOD_QUEUE.md`](../docs/TICKET_MOD_QUEUE.md) |

Handoff: mark slice ready → correct executor implements → `./scripts/ticket advance-slice` or `./scripts/ticket done`.

## Claude Code plan → Cursor review → ticket (HARD)

**Authority:** [`.cursor/rules/cursor-agent-workflow.mdc`](../../.cursor/rules/cursor-agent-workflow.mdc)

Infer **intent**, not exact phrases. Rough map:

| Intent | Mode | Cursor does |
|--------|------|-------------|
| "What do you think of this plan?" + paste | A | Critique + Claude revise prompt. No files. |
| "Ok / set it up / write the ticket / like T-091" | B | One ticket + spec + handoff + sync. No code. |
| "Fix it / implement / ship" | C | Handoff → Claude Code. Cursor does not patch app source. |

If unclear: one question — *review only, or write ticket + handoff?*

**Cursor must NOT:** edit app source when exploring plans or writing audit/ticket docs. **One ticket at a time** unless user asks for more.

## Generated views

| File | Shows |
|------|-------|
| [`docs/TICKET_REGISTRY.md`](../docs/TICKET_REGISTRY.md) | All tickets |
| [`docs/TICKET_LEAD.md`](../docs/TICKET_LEAD.md) | Lead dashboard |
| [`docs/TICKET_DEV_QUEUE.md`](../docs/TICKET_DEV_QUEUE.md) | Claude Code ready queue |
| [`docs/TICKET_MOD_QUEUE.md`](../docs/TICKET_MOD_QUEUE.md) | Mod / Workbench queue |
| [`docs/MILESTONES.md`](../docs/MILESTONES.md) | M1/M2 gate from registry |
| [`docs/TICKET_BRAINSTORM.md`](../docs/TICKET_BRAINSTORM.md) | Ideas + deferred |

## Validation

```bash
make ticket-sync
make ticket-check          # structural
make ticket-check-strict   # zero legacy P/FD/BE/Track IDs
```
