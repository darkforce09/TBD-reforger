# T-068.13 — Mod LOBBY slot picker (production UI)

**Ticket:** T-068 · **Slice:** T-068.13  
**Status:** Spec ready — **queued** (after **T-092.2** + **T-071.2**)  
**Executor:** claude-code (+ MCP / Workbench verify)  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

> Filename retains `_poc` for link stability; this slice is **production LOBBY UI**, not a CLI/debug hack.

---

## In one sentence

Add an **in-game LOBBY slot picker** (Aegis-quality, frictionless UX) listing mission `slots[]` from compiled mod JSON so a player **chooses a slot** before deploy.

---

## Dependencies (hard)

| Gate | Why |
|------|-----|
| **T-092.2** | Mod-native `slots[]` with correct x/z/y/headingDeg |
| **T-071.2** | Squad names + slotting order in export (player-facing labels) |

Does **not** require full T-071 modal — reads **compiled** `slots[]`.

---

## Problem

`TBD_SpawnManager` uses roster poll or **round-robin**. No in-game UI to pick Alpha SL vs Bravo AR.

---

## Goal

1. LOBBY-stage UI showing slots grouped **faction → squad (groupCallsign) → role**.
2. Per row: role, kit alias, optional loadout summary.
3. **Claim slot:** explicit `playerId → slotId` (one claim per slot server-side).
4. **Deploy** via `DeployPlayer` (position + kit + **T-068.12** loadout).
5. Walk-in: unclaimed slots visible without roster.

---

## UX bar

- macOS/Aegis design principles — readable, gamepad-friendly, **no CLI/debug menus**
- Frictionless claim → deploy flow

---

## Out of scope

| Deferred | Ticket |
|----------|--------|
| Event roster sync | **T-114** + **T-118** |
| Web ORBAT authoring | **T-071** (labels improve with T-071.2) |

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| v1 auth | Local server authority |
| Stage | `TBD_EGameStage.LOBBY` |
| Slot source | `TBD_MissionLoader.GetSlots()` |
| Production roster | **T-114** |

---

## Verification (S1–S5)

- [ ] **S1** Picker lists all mission slots grouped correctly
- [ ] **S2** Claim assigns slot; second player blocked on taken slot
- [ ] **S3** Deploy spawns at slot x/z with correct heading (T-092)
- [ ] **S4** Screenshot of LOBBY UI @ dev server
- [ ] **S5** With T-068.12 loadout, human player wears kit at chosen slot

---

## Related

- [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)
- [`t068_14_phase2_e2e_gate.md`](t068_14_phase2_e2e_gate.md)
