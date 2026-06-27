# T-068.6 — Phase 1 E2E human gate

**Ticket:** T-068 · **Slice:** T-068.6  
**Status:** **shipped** — Phase 1 sign-off **PASS** @ 2026-06-27 (after **T-068.5.1** @ `b233b11`)  
**Executor:** human  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Manual golden path proved dumb Virtual Arsenal works **web → file → mod (NPC test spawn)** before Phase 2 starts.

---

## Phase 1 boundary (read before E11)

| Who gets the web-export kit? | Who does **not**? |
|-----------------------------|-------------------|
| **`TBD_LoadoutEquipComponent`** spawns a **non-player test character** (AI/NPC entity) @ game-mode coords (~6400) and equips `$profile:TBD_LoadoutTest.json` | The **joining human player** — `DeployPlayer` / `SCR_MenuSpawnLogic` use mission **slot kit aliases** (`TBD_SpawnManager`), **not** the dumb loadout-export file |

Phase 1 proved **ResourceName wear works in-engine** on that test NPC. Wiring editor loadout → **human player** is **Phase 2**: **T-068.11** (compiler data) → **T-068.12** (mod equip on deploy) → **T-068.13** (mod slot picker) → **T-068.14** (E2E sign-off).

---

## Problem

No signed-off proof that registry + palette + loadout export + mod equip chain works end-to-end.

---

## Goal

Human runs checklist with **objective PASS/FAIL per row**; signs off Phase 1 only when **100% PASS**. Registry stays `ready` (do **not** `./scripts/ticket done T-068`).

Optional: git tag commit note "T-068 Phase 1" — full **T-068** tag @ **T-068.14** only.

---

## Preconditions (hard gate)

Before starting T-068.6, confirm **verify paste blocks exist** in Docs & Tickets chat for:

- T-068.0.1, T-068.1, T-068.2, T-068.3, T-068.4, T-068.5, **T-068.5.1** (required for E11 visual)
- **T-068.1 MCP verify paste required** for Phase 1 sign-off (dev-seed-only E2E is smoke, not ship proof — waive only with explicit note)

Stack:

```bash
make db-up && make api && make web
curl -sf http://localhost:8080/api/v1/health
```

Dev-login: `http://localhost:8080/api/v1/auth/dev-login?role=mission_maker`

---

## E2E checklist (all must PASS)

| ID | Step | Pass condition | Evidence required |
|----|------|----------------|-----------------|
| E1 | Editor boot | `/missions/:id/edit` loads; no error overlay | Screenshot or URL + load time |
| E2 | Registry network | DevTools: `GET /api/v1/registry` **200**; if Workbench data: paste **`import-registry-items`** command run | Status + row count + import command (if applicable) |
| E3 | Factions tree | NATO + CSAT from API; not static mock | Tree root labels pasted |
| E4 | Search `medic` | Exactly one visible Medic row path | Pasted filter string + visible label |
| E5 | Search `nato` | NATO subtree visible | Pasted observation |
| E6 | Drag place | Character placed on map | OBJ count +1 |
| E7 | `assetId` | Placed slot uses GUID `resource_name` (paste value) | Exact string |
| E8 | Arsenal tab | **Stub gone** — 4 **enabled** dropdowns + **enabled** download (not “Loadout Forge soon”) | Screenshot showing dropdowns + no stub copy |
| E9 | Download | `loadout-export.json` passes jq gate from T-068.4 spec | jq command outputs |
| E10 | Profile copy | `TBD_LoadoutTest.json` at documented profile path | `ls -la` + `sha256sum` |
| E11 | Mod NPC equip | Workbench `wb_play`: **non-player test spawn** @ ~6400 receives kit from profile JSON — **T-068.5.1** worn-verify logs + **screenshot** (M60, BDU jacket, PASGT vest, helmet). **Not** the human player body. See [`TBD_LoadoutEquipComponent.c`](../../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c) | Log excerpt + entity id + **screenshot** |
| E12 | Perf smoke | Pan/zoom 10s on mission with ≥200 slots (or largest available) — no freeze | FPS counter ≥55 or subjective "no stall" with slot count noted |

**E11 note:** T-068.5 @ `21ec91e` log-only `equip OK` was a **false pass** (naked mesh). **T-068.5.1** @ `b233b11` fixed wear (`EquipCloth` / `EquipWeapon` + deferred worn-verify). E11 requires **visual** proof on the **test NPC**, not log lines alone.

---

## Recorded sign-off (2026-06-27)

```markdown
## T-068.6 verify — PASS
**Phase 1 E2E gate**
**Date:** 2026-06-27
**Data source:** dev seed + Workbench profile (`TBD_LoadoutTest.json` from web export)

### Checklist
| ID | Result | Evidence |
|----|--------|----------|
| E1 | PASS | Editor boot, small mission |
| E2 | PASS | GET /api/v1/registry 200, 21 rows |
| E3 | PASS | NATO + CSAT from API |
| E4 | PASS | Search medic |
| E5 | PASS | Search nato |
| E6 | PASS | Character placed, OBJ +1 |
| E7 | PASS | assetId = GUID resource_name |
| E8 | PASS | Arsenal 4 dropdowns + download |
| E9 | PASS | loadout-export.json jq gates |
| E10 | PASS | Profile TBD_LoadoutTest.json |
| E11 | PASS | T-068.5.1 @ b233b11 — test NPC @ spawn: M60, BDU jacket, PASGT vest, helmet+goggles (screenshots). Player body unchanged. |
| E12 | PASS | Large mission ~367k, pan/zoom OK |

**Phase 2 approved to start:** YES
```

**Unblocks:** **T-068.7** (compat matrix spec, cursor-docs).

---

## Verification gate (mandatory)

### Sign-off format (paste to Cursor)

```markdown
## T-068.6 verify — PASS | FAIL
**Phase 1 E2E gate**
**Date:**
**Data source:** dev seed only | workbench import (state which)

### Checklist
| ID | Result | Evidence |
|----|--------|----------|
| E1 | PASS | … |
… E12 … |

**Phase 2 approved to start:** YES | NO
```

**Advance to T-068.7 only when:** header says **PASS**, E1–E12 all **PASS**, and **Phase 2 approved: YES**.

---

## Out of scope

- Phase 2 worker/UI
- `./scripts/ticket done T-068`
- **Human player spawn loadout** (T-068.11–T-068.14); in-game slot picker POC (T-068.13); production roster picker (T-114)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Gate | Zero-fail checklist; no waivers without new ticket |
| Ticket status | Remains `ready` through Phase 2 |
| E11 subject | **Non-player test NPC** only — not DeployPlayer |

---

## Depends on / Unblocks

- **Depends on:** T-068.1–T-068.5.1 verify pastes + E1–E12
- **Unblocks:** T-068.7 (after approval)

---

## Documentation sync (Cursor)

**Done @ 2026-06-27:** Phase 1 acceptance in program hub; CLAUDE §Status; MC ROADMAP; mod README NPC vs player boundary; `active_slice` → **T-068.7**.

**Mod script (E11):** [`TBD_LoadoutEquipComponent.c`](../../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_LoadoutEquipComponent.c) · [`TBD_GameMode.et`](../../../apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et) · wear fix **T-068.5.1** @ `b233b11` · scaffold **T-068.5** @ `21ec91e`.
