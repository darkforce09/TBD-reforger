# T-068.10 — Smart Loadout Forge UI

**Ticket:** T-068 · **Slice:** T-068.10  
**Status:** Spec ready — Phase 2  
**Executor:** claude-code  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Replace dumb dropdowns with registry-worker-validated Forge UI — block invalid combos before export.

---

## Problem

Phase 1 dumb UI allows impossible weapon/optic/mag combinations.

---

## Goal

1. Upgrade Attributes **Arsenal** tab (or dedicated Forge panel) to use `registryClient` validation on every pick.
2. Show blocked state when `canEquip` / `canAttach` fails (Aegis error chip).
3. Export still produces `loadout-export.json` — extended Phase 2 shape if spec updated in T-068.7 (attachments array deferred unless locked in .7).
4. Preserve Phase 1 export path for missions without compat data (graceful degrade if worker stale).

---

## Out of scope

- Compiler mission envelope (T-068.11)
- Eden palette changes

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Validation | Worker-only before commit to export object |
| UI home | Same Attributes Arsenal tab (smart mode) |

---

## Tasks

1. `useArsenalValidation.ts` bridge to worker
2. Forge UI components (picker grids / attachment rows per engineering_plan)
3. Wire to export helper from T-068.4

---

## Verify

```bash
cd apps/website/frontend && npm run build && npm run lint
```

---

## Verification gate (mandatory)

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | Build/lint | Exit 0 |
| A2 | Block invalid | UI prevents/export blocks invalid optic combo (paste steps + result) |
| A3 | Allow valid | Valid loadout exports; jq gate on download passes |
| A4 | Worker used | Network/worker trace shows validation call before export |
| A5 | No regression | Dumb export path still works when worker warm |

Manual table M1–M3 required in verify paste (invalid blocked, valid passes, worker trace).

---

## Depends on / Unblocks

- **Depends on:** T-068.9
- **Unblocks:** T-068.11

---

## Claude Code prompt — T-068.10

```
Read CLAUDE.md §Status. Active slice: T-068.10.
Implement ONLY docs/specs/Mission_Creator_Architecture/t068_10_smart_forge_ui.md
Do not edit documentation. Branch: ticket/T-068
Verify: cd apps/website/frontend && npm run build && npm run lint
Return: files changed + manual verify notes.
```
