# T-092.1 — Verify log (mod spawn height + yaw policy)

**Branch:** `ticket/T-092` (worktree `.ai/artifacts/worktrees/TBD-T-092`)
**Date:** 2026-07-03
**Status:** automated gates PASS · wb_play gates **PENDING** (operator decision 2026-07-03: Workbench is
held by parallel Stream A / T-090.1.1 on `main`; wb_play runs in one coordinated window later —
tag `T-092.1` only after those pass).

## Automated

| Gate | Command | Result |
|------|---------|--------|
| S1 schema | `cd packages/tbd-schema && npm run validate` | **PASS** — "All contracts valid" (golden missions incl. `bridgehead-at-levie.json` validate against the 1.2 schema; `y` optional) |
| Go regression | `cd apps/website && go build ./...` | **PASS** |

## wb_play (PENDING — coordinated window)

Workbench state at prep time: connected (`wb_connect`/`wb_state` OK), Everon/TBD_Dev_POC subscene open,
but the loaded gproj is the **main checkout** (`Z:/home/Samuel/Projects/TBD-Reforger/apps/mod/tbd-framework/addon.gproj`),
not this worktree — Stream A owns that tree + Workbench right now.

| ID | Step | Pass condition | Result |
|----|------|----------------|--------|
| M1 | Spawn slot on hill | feet on ground, log delta < 0.5 m | **PENDING** |
| M2 | Spawn slot in valley | same | **PENDING** |
| M3 | Spawn with explicit jsonY | jsonY path logged | **PENDING** |
| M4 | headingDeg 90 vs 0 | faces east ±5° | **PENDING** |
| S5 | CAPSULE_GROUND_OFFSET_M | documented with measurement | **PENDING** — constant ships as `0.0` placeholder; calibrate from `groundDelta` below |

### Prepared verification recipe (run in the coordinated window)

1. Get the T-092.1 scripts in front of Workbench (merge/overlay/worktree gproj — operator's choice).
2. `bash scripts/mod/mcp-call.sh wb_reload '{"target":"scripts"}'` — Enforce compile gate.
3. Back up Workbench profile mission `…/ArmaReforgerWorkbench/profile/missions/msn_8f3a2c.json`, then patch it:
   `schemaVersion: "1.2"`, keep all 17 slots/orbat (loader enforces count parity), edit:
   - slots[0] → hill `x=6400 z=7400` (surfaceY **174.375**), `headingDeg=90`, no `y` → surface path
   - slots[1] → shore/valley `x=7800 z=8000` (surfaceY **0.125**), `headingDeg=0`
   - slots[2] → mid `x=7000 z=7000` with explicit `"y": 102` (measured surfaceY **102**) → jsonY path
   (heights sampled live via `wb_terrain getHeight`, 2026-07-03)
4. Run A: `bash scripts/mod/tbd-spawn-verify.sh "\[TBD\]\[Spawn\]"` — player round-robins into slots[0] (hill):
   read `[TBD][Spawn] slot=… Y=… jsonY=… surfaceY=… delta=… heading=…` for every slot (covers M3 log path +
   M4 heading values at build time) and `[TBD][Spawn] deployed player=1 … feetY=… surfaceY=… groundDelta=… yaw=…`
   (M1 + capsule measurement + yaw≈90 for M4).
5. Run B: reorder fixture so the valley slot is slots[0], `headingDeg=0` → M2 + yaw≈0.
6. `groundDelta` from the deployed lines **is** the capsule offset measurement on a human character —
   set `CAPSULE_GROUND_OFFSET_M` in `TBD_SpawnManager.c` to the correction it implies (0.0 if the engine
   settles feet-on-ground), note the measured values here, re-run once to confirm delta < 0.5 m.
7. Restore the profile mission backup.

## Shipped code (this slice)

- `packages/tbd-schema/schema/mission.schema.json` — optional slot `y` (m ASL), `schemaVersion` enum + 1.2
  slots-required conditional.
- `TBD_MissionSlotStruct.c` — `float y = -1000000` sentinel (`Y_ABSENT`) + `HasJsonY()`; JsonLoadContext
  leaves missing keys at the initializer.
- `TBD_SpawnManager.c` — spawn height policy (jsonY-if-present else surfaceY, + `CAPSULE_GROUND_OFFSET_M`),
  `MAX_Y_DELTA_M = 2.0` warn, `[TBD][Spawn]` build log, `LogDeployedTransform` post-deploy diagnostic
  (feet-Y vs surface + yaw — the measurement instrument for S5/M4).
- `TBD_MissionLoader.c` — slot validation now covers `"1.2"` (was 1.1-only, a 1.2 doc skipped validation).

APIs verified via enfusion-mcp `api_search` (not guessed): `BaseWorld.GetSurfaceY(float,float)`,
`PlayerManager.GetPlayerControlledEntity(int)`, `IEntity.GetOrigin()`, `GetYawPitchRoll()`,
`ScriptCallQueue.CallLater(fn, delay, repeat, param…)`.
