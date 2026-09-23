# Ticket AI Playbook

**Audience:** Cursor (Composer 2.5) — ticket operator. **Source of truth:** one `T-*.toml` file per ticket in this directory, beside the [`ROOT`](ROOT) marker.

## Golden rule

Edit the ticket's **`T-*.toml`** → run **`cargo xtask ticket sync`** → run **`cargo xtask ticket check`** → commit the ticket file and every generated file together.

Never hand-edit: `queue.json`, the `<!-- ticket-sync:next -->` block in [`docs/specs/Mission_Creator_Architecture/ROADMAP.md`](../../docs/specs/Mission_Creator_Architecture/ROADMAP.md), the gap_analysis ticket column.

## HARD — No deferrals without operator word

Do not put "fold forward", "deferred to later slice", or self-authored Out-of-scope into specs /
handoffs / finish plans unless the **operator explicitly** authorized that deferral in-thread.
Complete-outcome asks ("replace the whole surface", "finish the port") mean **finish**, not a thin
first pass plus an appendix.
Rule: [`.cursor/rules/no-silent-deferrals.mdc`](../../.cursor/rules/no-silent-deferrals.mdc).

## Status lifecycle

| You say | Ticket file change |
|---------|--------------------|
| Add idea | `status = "idea"`, full context fields, **no order** |
| Promote to backlog | `status = "queued"`, assign `order` |
| Write spec | Create `tNNN_*.md`, set `spec`, `status = "ready"` |
| Slice done | Advance `active_slice` to the next child, or `status = "shipped"` |
| Ship a ticket | `status = "shipped"`, clear `active_slice`, sync |
| Cancel a ticket | `status = "cancelled"` — never delete the file |
| Reorder | Lower one ticket's `order` below its sibling's |

## Recipes

### Ship a ticket

1. Human verified merge + build/lint pass
2. Set the ticket file's `status = "shipped"`, remove `active_slice`
3. `cargo xtask ticket sync`
4. Update narrative docs per [`docs/website/AGENT_COMMIT_CHECKLIST.md`](../../docs/website/AGENT_COMMIT_CHECKLIST.md)

### Mark ready for Claude Code

```bash
cargo xtask ticket mark-ready T-068 docs/specs/Mission_Creator_Architecture/t068_asset_registry.md
cargo xtask ticket run
```

**Prompt standard:** [`CLAUDE_CODE_PROMPT.md`](CLAUDE_CODE_PROMPT.md) · handoff skeleton: [`HANDOFF_TEMPLATE.md`](HANDOFF_TEMPLATE.md)

1. Write slice spec + §Claude Code prompt (fenced block in spec — **not** only in SEND_TO_CLAUDE).
2. Write `.ai/artifacts/{slug}_claude_code_handoff.md`.
3. Optional thin `.ai/artifacts/{slug}_SEND_TO_CLAUDE.md` → points at `cargo xtask ticket prompt ID`.
4. Human sends: `cargo xtask ticket prompt T-0xx` → paste into Claude Code.

### Brainstorm (speech-to-text friendly)

1. `cargo xtask ticket add "Outliner search" --program eden --surfaces LEFT --impact ui`
2. Review the `idea` and `deferred` columns in [ticketboard](/apps/ticketboard/README.md)
3. When promoted: assign `order`, write spec, `mark-ready`

### Developer brief

```bash
cargo xtask ticket brief T-0xx
```

## Executor gate

**CRITICAL:** `cargo xtask ticket run` only executes slices with `executor: claude-code`. Slices with `workbench`, `human`, `cursor-docs`, or `ci` are skipped or handled by the matching agent.

| Executor | Agent | Scope |
|----------|-------|-------|
| `claude-code` | Claude Code | `apps/website/{api,frontend}/` code on **`main`** |
| `cursor-docs` | Cursor | specs, ticket files, `cargo xtask ticket sync` |
| `workbench` / `human` | Human | `apps/mod/tbd-framework` — filter [ticketboard](/apps/ticketboard/README.md) by executor |

Handoff: mark slice ready → correct executor implements → `cargo xtask ticket advance-slice` or `cargo xtask ticket done`.

## Claude Code plan → Cursor review → ticket (HARD)

**Authority:** [`.cursor/rules/cursor-agent-workflow.mdc`](../../.cursor/rules/cursor-agent-workflow.mdc)

Infer **intent**, not exact phrases. Rough map:

| Intent | Mode | Cursor does |
|--------|------|-------------|
| "What do you think of this plan?" + paste | A | Critique + Claude revise prompt. No files. |
| "Ok / set it up / write the ticket / like the last one" | B | One ticket + spec + handoff + sync. No code. |
| "Fix it / implement / ship" | C | Handoff → Claude Code. Cursor does not patch app source. |

If unclear: one question — *review only, or write ticket + handoff?*

**Cursor must NOT:** edit app source when exploring plans or writing audit/ticket docs. **One ticket at a time** unless user asks for more.

## Viewing tickets

[ticketboard](/apps/ticketboard/README.md) shows every parent and child ticket from the ticket files, by status column and program tree; `cargo xtask ticket list` prints the dispatch queue.

## Validation

```bash
cargo xtask ticket sync
cargo xtask ticket check          # structural
cargo xtask ticket check --strict   # adds the pre-T id scan, the gap-analysis column check, the honesty counters
```
