# T-0xx — {TITLE}

**Ticket:** T-0xx  
**Status:** Spec ready — code pending  
**Git tag on ship:** **T-0xx**  
**Authority:** [`docs/TICKET_LEAD.md`](../docs/TICKET_LEAD.md) · [`tickets/registry.json`](registry.json)

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

On ship: run `./scripts/ticket ship T-0xx`; update narrative docs per [`docs/AGENT_COMMIT_CHECKLIST.md`](../docs/AGENT_COMMIT_CHECKLIST.md).

---

## Claude Code prompt — T-0xx (copy-paste)

**Read this file first.** Do **not** edit any documentation.

```
Read CLAUDE.md first.

Implement T-0xx per Design_Docs/Mission_Creator_Architecture/t0xx_{slug}.md.

LOCKED: see spec Locked decisions.

Verify: npm run build && npm run lint (+ make test-it if backend).

Commit on branch ticket/T-0xx when verify passes. DO NOT edit documentation — Cursor Composer 2.5 syncs docs after merge.

Return file list + build/lint output + manual verify notes.
```
