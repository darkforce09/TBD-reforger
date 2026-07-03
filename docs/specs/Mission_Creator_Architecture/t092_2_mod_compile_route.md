# T-092.2 — Mod compile flatten + /compiled API

**Ticket:** T-092 · **Slice:** T-092.2  
**Status:** **shipped** @ `a73224f2` (tag **T-092.2**) — verify **PASS** 2026-07-04 @ `452ce501`  
**Executor:** claude-code  
**Authority:** [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)

---

## In one sentence

Emit mod-native mission document 1.1/1.2 (`slots[]` with deterministic ids, kit aliases, x/z/y/headingDeg), add `GET /api/v1/missions/:id/compiled`, fix mod loader path, and pass golden + round-trip verify.

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-092.1** | Spawn policy + schema `y` shipped |
| **T-091.2** | Editor `position.z` + rotation in snapshot |
| **T-068** | Registry for ResourceName → `kit:` mapping strategy |

---

## Problem

| Gap | Today |
|-----|-------|
| Compile | [`compile.ts`](../../../apps/website/frontend/src/features/mission-creator/compiler/compile.ts) → `editor.slots` only |
| API | Go **`/api/v1/missions/:id/export`** wrapper only |
| Mod fetch | `{url}/api/missions/{id}/compiled` — **wrong path** |
| Slot id | UUID vs `blufor:Alpha:SL:0` |
| kit | ResourceName vs `kit:us_sl` |

---

## Goal

1. **`flattenEditorToModDocument(snapshot)`** → full document matching [`bridgehead-at-levie.json`](../../../packages/tbd-schema/golden-missions/bridgehead-at-levie.json).
2. Deterministic slot id: `{faction}:{groupCallsign}:{role}:{index}` (align [`flatten-orbat-slots.mjs`](../../../packages/tbd-schema/scripts/flatten-orbat-slots.mjs)).
3. `assetId` → `kit:` alias via registry mapping table.
4. `orbat` **map** builder from editor factions/squads.
5. Go: `GET /api/v1/missions/:id/compiled` — `RequireServiceToken` — body = mod document (**not** `buildMissionDoc` wrapper).
6. Mod: fix [`TBD_MissionLoader.c`](../../../apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c) → `/api/v1/missions/{id}/compiled`.
7. DEV_RUNBOOK curl example.

---

## Out of scope

- LOBBY slot picker (**T-068.13**)
- Loadout blocks (**T-068.11**)
- Event ORBAT (**T-071**)

---

## Locked decisions

| Artifact | Shape |
|----------|-------|
| Version POST | `{ schemaVersion: 1, editor: { slots }, orbat[] }` unchanged |
| Mod compiled | Native 1.1/1.2 document |
| Export download | Existing camelCase wrapper unchanged |

Worker note: flatten runs on **main thread** post-compile or worker receives serializable plain object — **no DEM fetch in worker**.

---

## Verification gate (mandatory)

**Unblocks T-071 and T-068.13** only when ALL PASS.

### Automated

```bash
cd packages/tbd-schema && npm run validate
make test-it
cd apps/website/frontend && npm run build && npm run lint

# API smoke (dev stack)
curl -sS -H "X-Service-Token: $SERVICE_TOKEN" \
  http://localhost:8080/api/v1/missions/{id}/compiled | jq .schemaVersion
```

### Acceptance criteria (S1–S5 from hub)

| ID | Check | Pass condition |
|----|-------|----------------|
| S1 | `npm run validate` | All golden missions + new compile output validate |
| S2 | Mod load | Profile cache JSON loads; spawn points built |
| S3 | headingDeg | ±5° @ 3 anchors (log + screenshot) |
| S4 | API | `/compiled` body matches profile shape |
| S5 | Schema 1.2 | Optional `y` on slots; old missions still valid |
| S6 | Round-trip | Editor save → compiled → mod spawn @ bridgehead coords ±2 m horizontal |

### Manual

| ID | Step | Pass condition |
|----|------|----------------|
| M1 | Save mission in MC | Version 201 |
| M2 | curl /compiled | `slots[]` length matches editor slot count |
| M3 | Dedicated server load | `[TBD] SpawnManager: built slot spawn` × N |
| M4 | Deploy test player | Position matches editor slot x/z within 2 m |

---

## Related

- [`t068_13_mod_slotting_screen_poc.md`](t068_13_mod_slotting_screen_poc.md)
- [`t071_orbat_manager_program.md`](t071_orbat_manager_program.md)
