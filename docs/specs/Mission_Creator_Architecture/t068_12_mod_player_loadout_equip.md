# T-068.12 — Mod player loadout equip on spawn

**Ticket:** T-068 · **Slice:** T-068.12  
**Status:** **SHIPPED** @ `0be53e16` (tag **T-068.12**) ·  
**Executor:** claude-code / Fable 5 (+ MCP / Workbench verify) ·  
**Verify:** [`.ai/artifacts/t068_12_verify_log.md`](../../../.ai/artifacts/t068_12_verify_log.md) ·  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) ·  
**Operator residual:** M2 dressed-player screenshot; optional M4 NPC toggle

---

## In one sentence

When a **human player** deploys on an assigned mission slot, apply that slot's compiled **loadout gear** (ResourceNames) and **cargo[]** (container items via `InsertItem`) using the proven **T-068.5.1** equip path — not the isolated test-NPC harness.

---

## Problem

Phase 1 (`TBD_LoadoutEquipComponent`) dresses a **server-spawned test NPC** from `$profile:TBD_LoadoutTest.json`. **`TBD_SpawnManager.DeployPlayer`** spawns the player with a **kit alias prefab** only — no per-slot Arsenal gear. The joining player stays visually wrong even when the editor export is correct.

---

## Goal

1. Extend mission slot JSON contract (compiled mission / schema) with optional structured loadout block per slot, e.g. `loadout.gear.{primary,uniform,vest,helmet}` as ResourceName strings (aligned with **T-068.11** compiler output).
2. Parse **`loadout.cargo[]`** (`{container, item, qty}`) from compiled JSON (T-068.11 + T-068.15 chain).
3. Parse loadout on `TBD_MissionLoader` into `TBD_MissionSlotStruct` (or companion struct).
4. After successful player spawn for a slot, run shared equip logic (extract from `TBD_LoadoutEquipComponent` — `EquipCloth` / `EquipWeapon` + worn-verify) on the **player character entity**, not a dev NPC.
5. After wear/weapons equip, **`InsertItem`** each cargo row into the matching container storage on the player (resolve `container` key → storage component; same API family as T-068.5.1 storage path).
6. Log lines clearly tagged `[TBD][Loadout][Player]` vs `[TBD][Loadout][TestNPC]` so E2E evidence is unambiguous.
7. Empty/null gear slots or empty cargo = skip (same as Phase 1).

---

## Out of scope

- Slot picker UI (**T-068.13**)
- Web roster / event claim sync (**T-114**)
- Compat matrix / smart Forge (**T-068.7–T-068.10**)
- Removing kit-alias spawn entirely (kit prefab may still define base character; loadout **layers** gear on top until kit system is redesigned)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Equip API | Reuse **T-068.5.1** — `EquipCloth` / `EquipWeapon` + deferred worn-verify |
| Subject | **Human player character** after `DeployPlayer` / respawn completes |
| Data source | Per-slot block in **compiled mission JSON** (from **T-068.11**, including `cargo[]`), not profile file |
| Cargo equip | `SCR_InventoryStorageManagerComponent.InsertItem` (or proven T-068.5.1 storage API) into resolved container |
| Dev harness | Keep `TBD_LoadoutEquipComponent` test-NPC path for manual JSON experiments |

---

## Tasks

1. Schema: document slot `loadout` shape in `packages/tbd-schema` (mission compiled schema) if not already added in T-068.11.
2. `TBD_MissionSlotStruct.c` + loader parse path.
3. Shared helper (e.g. `TBD_LoadoutEquipHelper`) callable from test component and spawn manager.
4. `TBD_SpawnManager`: hook post-spawn (or respawn callback) → equip from assigned slot loadout.
5. MCP / `wb_play` verify on dev world with golden mission fixture carrying loadout block.

---

## Verify

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
# wb_play with mission JSON containing slot loadout block
```

---

## Verification gate (mandatory)

### Automated

| ID | Command | Pass |
|----|---------|------|
| A1 | Workbench script compile (bootstrap) | No script errors |

### Manual (MCP / dedicated server)

| ID | Step | Pass condition |
|----|------|----------------|
| M1 | Load mission with slot loadout block | Loader logs parsed gear ResourceNames for slot |
| M2 | Deploy player on that slot (round-robin or forced slot id dev hook) | Player character shows primary + vest + helmet + jacket (**screenshot required**) |
| M3 | Log proof | `[TBD][Loadout][Player]` worn-verify PASS lines; no false OK |
| M4 | Regression | Test-NPC harness (`TBD_LoadoutTest.json`) still works when enabled |

### Acceptance criteria

| ID | Check | Pass |
|----|-------|------|
| A2 | Visual | Screenshot of **human player** (not NPC) with kit visible |
| A3 | API | Equip uses same paths as T-068.5.1 |
| A4 | Scope | No slot picker UI in this slice |

---

## Depends on / Unblocks

- **Depends on:** T-068.11 (compiler emits per-slot loadout gear + cargo), T-068.15.1 + T-068.15.2 (cargo data path)
- **Unblocks:** T-068.13 (slot picker can target slots that carry loadouts), T-068.14 (Phase 2 E2E)

---

## Claude Code prompt — T-068.12 (copy-paste)

Authority: this spec + program handoff. **Do not edit docs/registry/CLAUDE** (verify log OK).

```
Read CLAUDE.md first. Work on main at repo root.

Implement **T-068.12** — Mod player loadout equip + InsertItem cargo.

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger
  git rev-parse T-068.11
  bash scripts/mod/tbd-dev-bootstrap.sh

═══ READ ═══
  1. .ai/artifacts/t068_15_fable_program_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t068_12_mod_player_loadout_equip.md
  3. .ai/artifacts/t068_11_verify_log.md
  4. TBD_LoadoutEquipComponent / T-068.5.1 equip path
  5. TBD_SpawnManager DeployPlayer
  6. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  Compiled slots have gear+cargo but DeployPlayer still spawns kit-alias only —
  human players never get Arsenal wear or container contents.

═══ DO ═══
  - Parse loadout.gear + loadout.cargo[] into mission slot structs
  - Post-spawn equip player via T-068.5.1 EquipCloth/EquipWeapon path
  - InsertItem cargo into resolved container storages
  - Logs [TBD][Loadout][Player] · keep test-NPC harness
  - Verify log + tag T-068.12

═══ DO NOT ═══
  - Slot picker UI (T-068.13)
  - Edit docs/registry
  - Silent deferral of cargo InsertItem

═══ VERIFY ═══
  tbd-dev-bootstrap + wb_play; M1–M4; screenshot of PLAYER (not NPC)
  .ai/artifacts/t068_12_verify_log.md

═══ RETURN ═══
  SHA + tag T-068.12 · log excerpt [Loadout][Player] · cargo InsertItem proof
```
