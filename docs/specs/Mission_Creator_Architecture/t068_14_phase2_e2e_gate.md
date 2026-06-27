# T-068.14 — Phase 2 E2E gate (editor → player)

**Ticket:** T-068 · **Slice:** T-068.14  
**Status:** Spec ready — after T-068.12 + T-068.13  
**Executor:** human  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Human sign-off that **Phase 2** works end-to-end: Mission Creator loadout → Save/Export → compiled mission JSON → mod slot picker → **human player** spawns dressed correctly.

---

## Problem

Phase 1 E2E (**T-068.6**) proved web JSON → **test NPC** only. Phase 2 adds compiler export, smart Forge, and mod spawn path — we need a explicit gate before `./scripts/ticket done T-068`.

---

## Prerequisites

| Slice | Must be shipped |
|-------|-----------------|
| T-068.10 | Smart Forge UI (or minimum: slot loadout saved in editor) |
| T-068.11 | Compiler / export carries per-slot loadout block |
| T-068.12 | Player loadout equip on deploy |
| T-068.13 | Mod LOBBY slot picker (production UI) |

---

## Checklist (human)

| ID | Step | Pass condition |
|----|------|----------------|
| P1 | Editor | Assign loadout to a character slot in MC; Save Version 201 |
| P2 | Export | Compiled mission JSON (API or export) contains slot `loadout.gear` ResourceNames |
| P3 | Server | Load mission on dev/staging dedicated server |
| P4 | Slotting | LOBBY: open slot picker; claim target slot |
| P5 | Spawn | Deploy → spawn at slot position with correct kit alias |
| P6 | **Visual** | **Screenshot: human player** wearing editor loadout (primary + uniform/vest/helmet as configured) |
| P7 | Logs | `[TBD][Loadout][Player]` worn-verify PASS; `[TBD][Slotting]` claim line for same slot id |
| P8 | Negative | Confirm test-NPC harness is not the subject of P6 (player entity only) |

---

## Sign-off template

```markdown
## T-068.14 Phase 2 E2E — PASS | FAIL
**Date:**
**Mission id / version:**
**Slot claimed:**
**Screenshot:** (attach P6)
**Log excerpts:** P7

| ID | Result | Notes |
|----|--------|-------|
| P1 | PASS | |
…
```

---

## After PASS

Cursor runs doc sync; human runs:

```bash
./scripts/ticket advance-slice T-068   # if pointer is T-068.14
./scripts/ticket done T-068
git tag T-068
```

---

## Depends on / Unblocks

- **Depends on:** T-068.10, T-068.11, T-068.12, T-068.13
- **Unblocks:** `./scripts/ticket done T-068`; **T-069+** Eden queue
