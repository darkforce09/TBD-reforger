# T-092.1 — Mod spawn height + yaw policy

**Ticket:** T-092 · **Slice:** T-092.1  
**Status:** **shipped** @ `4eefc169` (tag **T-092.1**) — verify **PASS** 2026-07-04 @ `452ce501`  
**Executor:** claude-code (+ MCP / Workbench verify)  
**Authority:** [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md)

---

## In one sentence

Add optional slot `y` to schema + mod struct, implement spawn height policy (`json y` → else `GetSurfaceY` + measured capsule offset), apply `headingDeg` on deploy, and verify feet-on-ground at 3 hilly anchors.

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-091.2** | **Done** @ `dde589e` — editor stores meaningful `position.z` |
| **T-091.0** | Anchor verify PASS |
| **Workbench** | `tbd-dev-bootstrap.sh` + wb_play |

---

## Problem

[`TBD_SpawnManager.c`](../../../apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c) uses `GetSurfaceY(x,z)` only; ignores JSON height; no yaw from mission slot; no optional `y` in [`TBD_MissionSlotStruct.c`](../../../apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c).

---

## Goal

1. Schema: optional `y` on `$defs/slot` in [`mission.schema.json`](../../../packages/tbd-schema/schema/mission.schema.json); bump **schemaVersion "1.2"** when present.
2. Mod struct: optional `float y`; parse from JSON.
3. Spawn policy:

```text
spawnY = slot.y if finite
       else GetSurfaceY(slot.x, slot.z)
spawnY += CAPSULE_GROUND_OFFSET_M   // measured in wb_play — NOT guessed
heading = slot.headingDeg
```

4. Log: `[TBD][Spawn] slot=<id> Y=<final> jsonY=<y> surfaceY=<sy> delta=<d> heading=<h>`
5. Warn if `|jsonY - GetSurfaceY| > MAX_Y_DELTA_M` when both present.

---

## Out of scope

- Full compiler flatten (**T-092.2**)
- `/compiled` API route (**T-092.2**)
- Slot picker UI (**T-068.13**)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Capsule offset | Measure on **human character** spawn @ wb_play; constant in mod config |
| MAX_Y_DELTA_M | Start **2.0 m** — tune after first data |
| headingDeg | Degrees 0–360; mod applies to entity yaw |
| Golden missions | Still validate without `y` (optional field) |

---

## Verification gate (mandatory)

### Workbench / MCP

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
# Load golden or test mission with 3 slots at different elevations
bash scripts/mod/tbd-spawn-verify.sh   # extend if needed
```

### Manual

| ID | Step | Pass condition |
|----|------|----------------|
| M1 | Spawn slot on hill | Feet on ground (visual + log delta < 0.5 m) |
| M2 | Spawn slot in valley | Same |
| M3 | Spawn with explicit jsonY | Uses jsonY + offset; log shows path |
| M4 | headingDeg 90 vs 0 | Entity faces east (screenshot) |

### Automated

```bash
cd packages/tbd-schema && npm run validate
cd apps/website && go build ./...
```

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| S1 | Schema | Golden missions validate with optional `y` |
| S2 | M1–M2 | Feet on ground @ 3 elevations |
| S3 | M3 | jsonY path logged |
| S4 | M4 | heading ±5° |
| S5 | Offset | CAPSULE_GROUND_OFFSET_M documented with measurement log |

---

## Related

- [`t092_2_mod_compile_route.md`](t092_2_mod_compile_route.md)
