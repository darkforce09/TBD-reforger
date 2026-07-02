# T-0xx — {TITLE}

**Ticket:** T-0xx  
**Status:** Spec ready — code pending  
**Git tag on ship:** **T-0xx**  
**Authority:** [`docs/TICKET_LEAD.md`](../../docs/TICKET_LEAD.md) · [`tickets/registry.json`](registry.json)

**Agent roles (locked):** **Cursor Composer 2.5** authors and syncs all documentation. **Claude Code reads this spec and implements code only** — return verify output to Cursor; do **not** edit docs.

---

## In one sentence

{One sentence goal.}

---

## Problem

{What is broken or missing today?}

---

## Goal

{Numbered acceptance criteria.}

---

## Out of scope

- {Item}

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| {Decision} | {Choice} |

---

## Tasks

1. {File} — {change}
2. …

---

## Verify

```bash
cd frontend && npm run build && npm run lint
# make test-it  # if backend touched
```

**Manual:**
- {Checklist item}

---

## Documentation sync (Cursor Composer 2.5 — after human merge)

On ship: run `./scripts/ticket ship T-0xx`; update narrative docs per [`docs/AGENT_COMMIT_CHECKLIST.md`](../../docs/website/AGENT_COMMIT_CHECKLIST.md).

---

## Claude Code handoff (Mode B checklist)

When marking a slice ready for Claude Code:

1. Write this spec (problem, locked decisions, verify, manual acceptance).
2. Write `.ai/artifacts/{slug}_claude_code_handoff.md` — [`HANDOFF_TEMPLATE.md`](HANDOFF_TEMPLATE.md).
3. Add **§Claude Code prompt** below using the skeleton in [`CLAUDE_CODE_PROMPT.md`](CLAUDE_CODE_PROMPT.md).
4. Optional: `.ai/artifacts/{slug}_SEND_TO_CLAUDE.md` — one line: run `./scripts/ticket prompt T-0xx`.
5. Registry: `active_slice`, `slice_plan.{id}.status: ready`, `./scripts/ticket sync`.

**Do not** put the only copy of the prompt in SEND_TO_CLAUDE — `./scripts/ticket run` reads the spec.

---

## Claude Code prompt — T-0xx (copy-paste)

**Format:** [`CLAUDE_CODE_PROMPT.md`](CLAUDE_CODE_PROMPT.md). **Extract:** `./scripts/ticket prompt T-0xx`

```
Read CLAUDE.md first.

Implement **T-0xx** — {one-line title}.

═══ PREFLIGHT ═══
  git pull && make map-assets-link
  ./scripts/ticket brief T-0xx

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t0xx_claude_code_handoff.md
  2. docs/specs/.../t0xx_{slug}.md

═══ PROBLEM ═══
  {2–4 sentences}

═══ SHIPPED (do not reopen) ═══
  - …

═══ LOCKED ═══
  - See spec §Locked decisions

═══ DO ═══
  1. …

═══ DO NOT ═══
  - Edit docs/**, registry, docs/TICKET_*.md, CLAUDE status markers

═══ VERIFY (all exit 0) ═══
  cd apps/website/frontend && npm run build && npm run lint

═══ MANUAL ═══
  - …

═══ RETURN ═══
  - Commit SHA + tag T-0xx
  - **Ready for Cursor doc sync.**
```
