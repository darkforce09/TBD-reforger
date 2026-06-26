# Cursor workspace setup — TBD Reforger

**Purpose:** Fresh-start checklist for opening this monorepo in Cursor — local stack, persistent agent artifacts, scoped chats, and health checks. No application code changes.

**Authority:** [`CLAUDE.md`](../../CLAUDE.md) · [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json) · [DEV_RUNBOOK.md](DEV_RUNBOOK.md)

---

## Recommendation

**Commit both artifacts** — a root always-on rule ([`.cursor/rules/tbd-platform.mdc`](../../.cursor/rules/tbd-platform.mdc)) and this setup doc. Matches how the repo treats agent authority (`CLAUDE.md`, ticket registry, mod rules under `apps/mod/.cursor/`). Survives machine switches without re-pasting prompts.

For a one-off session with zero git noise, skip the file commits and use Cursor UI **Project Rules** only — same behavior locally, nothing shared or versioned.

---

## Why `.cursor` stays at root (not under `.ai/`)

| Folder | Role |
|--------|------|
| [`.ai/`](../../.ai/) | Repo-owned agent **pipeline** — [`tickets/`](../../.ai/tickets/) (registry) + [`artifacts/`](../../.ai/artifacts/) (generated research) |
| [`.cursor/`](../../.cursor/) | **Cursor IDE mount point** — product reads `.cursor/rules/*.mdc` and `.cursor/mcp.json` only from workspace root; mod scripts ([`scripts/mod/manual-test.sh`](../../scripts/mod/manual-test.sh)) also expect root `.cursor/mcp.json` |

Cursor will **not** auto-load rules from `.ai/`. Do not move project rules into `.ai/cursor/` unless you also keep a root `.cursor/` symlink — the IDE only discovers rules at repo root.

`apps/mod/.cursor/` remains **mod-scoped** (Workbench); do not copy `single-branch-main.mdc` to root — it conflicts with the ticket-branch workflow in [`CLAUDE.md`](../../CLAUDE.md).

---

## Current repo state (check on open)

| Item | Expected |
|------|----------|
| Workspace root | `/home/Samuel/Projects/TBD-Reforger` — monorepo layout |
| Agent bible | [`CLAUDE.md`](../../CLAUDE.md) — §Status synced from registry |
| Ticket registry | [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json) — `active_slice` + executor gate |
| Root `.cursor/` | [`.cursor/rules/tbd-platform.mdc`](../../.cursor/rules/tbd-platform.mdc) (always-on platform rule) |
| Stack docs | [DEV_RUNBOOK.md](DEV_RUNBOOK.md) — Go PATH gotcha |
| Health targets | `make verify-migration`, `make ticket-check-strict`, frontend build/lint |

Ignore sibling archived folders outside the workspace (`TBD_Website`, `Arma reforger`).

---

## Step 1 — Open workspace

**File → Open Folder** → `/home/Samuel/Projects/TBD-Reforger`

Confirm key paths exist:

- [`Makefile`](../../Makefile)
- [`apps/website/`](../../apps/website/)
- [`docs/specs/Mission_Creator_Architecture/`](../specs/Mission_Creator_Architecture/)
- [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json)

---

## Step 2 — Local stack (once)

From repo root:

```bash
cp apps/website/.env.example apps/website/.env   # only if .env missing
make db-up
PATH="$HOME/.local/go/bin:$PATH" make api   # background terminal
make web                                     # background terminal
curl -sf http://localhost:8080/healthz          # confirm API up (JSON status ok)
```

First-time frontend deps (if `node_modules` is absent):

```bash
cd apps/website/frontend && npm ci
```

- **Dev login:** `http://localhost:8080/api/v1/auth/dev-login?role=mission_maker`
- **Mission Creator:** `http://localhost:5173/missions/:id/edit` (after creating/opening a mission)
- **Full details:** [DEV_RUNBOOK.md](DEV_RUNBOOK.md)

**Mod Workbench** (T-068.1 / T-068.5 / T-068.8 — `workbench` slices): copy MCP config when needed — see Step 4.

---

## Step 3 — Repo artifacts

### Root Cursor rule — `.cursor/rules/tbd-platform.mdc`

Already in repo. Condensed contract:

- Read [`CLAUDE.md`](../../CLAUDE.md) first every session.
- Ticket source of truth: [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json) — never hand-edit generated `docs/TICKET_*.md` or marker blocks.
- **Cursor** writes docs/registry/sync; **Claude Code** implements only `executor: claude-code` slices via `./scripts/ticket run`.
- Respect executor gate: stop on `workbench`, `human`, or `ci` slices.

### This doc

Linked from [docs hub README](README.md).

---

## Step 4 — MCP (mod work only)

Copy [`apps/mod/.cursor/mcp.json`](../../apps/mod/.cursor/mcp.json) → `.cursor/mcp.json` at repo root **only** when opening Workbench/Enfusion chats. Paths inside are machine-specific (`ENFUSION_*`); verify against your Steam/Workbench install.

---

## Step 5 — Two scoped Cursor chats (manual UI)

Name chats exactly so they are searchable. Paste each opening prompt **once** per chat.

Claude Code runs in the **terminal** — no dedicated Cursor chat. Paste verify output into **Docs & Tickets**.

| Chat name | Purpose |
|-----------|---------|
| **Brainstorm** | Everything exploratory — MC, platform, backend, mod. No code unless asked. No registry/spec edits. |
| **Docs & Tickets** | Registry, spec markdown, `./scripts/ticket sync`, mark-ready, doc sync. Claude Code handoff + status. |

### Chat 1 — Brainstorm (opening prompt)

```
You are in the Brainstorm chat for TBD Reforger — ideas and design for anything in the monorepo (Mission Creator, website, backend, mod).

Read CLAUDE.md §Status when you need current ticket context. No code unless I ask.
Do not edit .ai/tickets/registry.json or spec files here — landed decisions go to the Docs & Tickets chat.
For MC work, useful refs: TICKET_LEAD, MC ROADMAP, program hub (`t068_virtual_arsenal_program.md`).
```

Optional @ pins: `CLAUDE.md`, `docs/TICKET_LEAD.md`.

### Chat 2 — Docs & Tickets (opening prompt)

```
You are in the Docs & Tickets chat for TBD Reforger.

You own cursor-docs work: .ai/tickets/registry.json, specs under docs/specs/, and doc sync on main.
Never hand-edit generated docs/TICKET_*.md. After registry edits: ./scripts/ticket sync && make ticket-check-strict.
Stop on workbench/human/ci executor slices.

Claude Code runs outside Cursor (./scripts/ticket run). I paste **Verify paste blocks** (program hub §Verification contract) here; Cursor checks **§Verification gate** tables before `./scripts/ticket advance-slice T-068`.
For status: read CLAUDE.md §Status + docs/TICKET_LEAD.md and answer in one screen.
```

Optional @ pins: `CLAUDE.md`, `.ai/tickets/registry.json`, `docs/TICKET_LEAD.md`.

**Claude Code** (terminal):

```bash
./scripts/ticket brief T-068
./scripts/ticket run   # claude-code slices only
```

---

## Step 6 — Health check (once after open)

From repo root:

```bash
make verify-migration
make ticket-check-strict
cd apps/website/frontend && npm run build && npm run lint
```

Optional deeper check:

```bash
PATH="$HOME/.local/go/bin:$PATH" make build
```

All green → workspace is healthy. Use **Brainstorm** for T-068.0 ideas; **Docs & Tickets** when landing registry/spec changes.

---

## Brainstorm → ship pipeline

Matches [`.ai/tickets/AI_PLAYBOOK.md`](../../.ai/tickets/AI_PLAYBOOK.md):

```
Idea (Brainstorm) → registry row (Docs & Tickets) → ./scripts/ticket sync
→ spec markdown → mark-ready → Claude Code (claude-code executor)
→ ticket done + Cursor doc sync (Docs & Tickets)
```

### MC doc read order (when doing MC work)

1. [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md)
2. [`docs/specs/Mission_Creator_Architecture/ROADMAP.md`](../specs/Mission_Creator_Architecture/ROADMAP.md)
3. [`agent_execution.md`](../specs/Mission_Creator_Architecture/agent_execution.md)
4. [`t068_virtual_arsenal_program.md`](../specs/Mission_Creator_Architecture/t068_virtual_arsenal_program.md)
5. [`eden/gap_analysis.md`](../specs/Mission_Creator_Architecture/eden/gap_analysis.md)
6. [`feature_inventory.md`](../specs/Mission_Creator_Architecture/feature_inventory.md)

---

## What this setup does NOT do

- No Go/React implementation until Claude Code slice **T-068.0.1+** on `ticket/T-068`
- Workbench export (**T-068.1**) and mod equip (**T-068.5**) are workbench/human executors
- No changes to ticket registry unless you start spec work in **Docs & Tickets**
