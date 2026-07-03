# T-092.1 — Verify log (mod spawn height + yaw policy)

**Branch:** `ticket/T-092` (worktree `.ai/artifacts/worktrees/TBD-T-092`) → merged to `main` @ `a73224f2`
**Date:** 2026-07-03 · **post-merge verify 2026-07-04 on `main`**
**Status:** **ALL GATES PASS** — automated re-run on `main` + wb_play M1–M4 + S5 measured in the
2026-07-04 coordinated window (details below). `CAPSULE_GROUND_OFFSET_M` stays **0.0** (measured
groundDelta ≤ 0.003 m — engine settles feet-on-ground; no calibration commit needed).

## Automated

| Gate | Command | Result |
|------|---------|--------|
| S1 schema | `cd packages/tbd-schema && npm run validate` | **PASS** — "All contracts valid" (golden missions incl. `bridgehead-at-levie.json` validate against the 1.2 schema; `y` optional) |
| Go regression | `cd apps/website && go build ./...` | **PASS** |

**Post-merge re-run on `main` @ `a73224f2` (2026-07-04):** S1 schema **PASS** ("All contracts valid"),
`make test-it` **PASS** (handlers 5.9 s), FE `npm run build` + `lint` **PASS**, `npm test` **53/53**
(main superset — worktree had 49; +4 from T-090.1.1 basemap suite).

## wb_play (PASS — 2026-07-04 post-merge window, `main` @ `a73224f2`)

Ran exactly per the prepared recipe below (runs A + B on the patched profile mission,
`backendUrl` blanked → profile-file path). Log: WB `logs_2026-07-04_00-00-12` (run A) /
`logs_2026-07-04_00-04-42` (run B).

| ID | Step | Pass condition | Result |
|----|------|----------------|--------|
| M1 | Spawn slot on hill | feet on ground, log delta < 0.5 m | **PASS** — build `slot=blufor:Alpha:SL:0 Y=174.375 jsonY=- surfaceY=174.375 delta=0 heading=90` (recipe's pre-sampled 174.375 exact); deployed `pos=<6399.13, 174.374, 7399.5> feetY=174.374 surfaceY=174.373 groundDelta=0.000579834` |
| M2 | Spawn slot in valley | same | **PASS** — build `Y=0.125 surfaceY=0.125` (recipe exact); deployed `pos=<7800, 0.109521, 7999> groundDelta=0.000145763` |
| M3 | Spawn with explicit jsonY | jsonY path logged | **PASS** — `slot=blufor:Alpha:TL:1 Y=102 jsonY=102 surfaceY=102 delta=0` (jsonY wins; delta 0 vs live surface) |
| M4 | headingDeg 90 vs 0 | faces east ±5° | **PASS** — heading 90 → deployed `yaw=-89.999` (engine yaw is CCW-positive; −90 engine ≡ compass 090°/east, ±0.001°); heading 0 → deployed `yaw=0` exact |
| S5 | CAPSULE_GROUND_OFFSET_M | documented with measurement | **PASS** — measured `groundDelta` on deployed human character: **+0.00058 m** (hill), **+0.000146 m** (valley), **−0.0029 m** (run C, T-092.2 log) → engine settles feet-on-ground; constant stays **0.0**, no commit |

**Ops note (root-caused during this window):** Workbench had been started 2026-07-03 23:33:19 and
compiled scripts at 23:33:25 — the T-092 merge commits landed 23:33:42, so the first wb_play attempt ran
**pre-merge bytecode** (old loader URL + old `[TBD]` log format, no `[TBD][Spawn]` lines), and
`wb_reload {"target":"scripts"}` does **not** actually recompile (every ScriptEditor/WorldEditor
`ExecuteAction` menu path returns false → `ExecuteAction=false`). Additionally `TBD_MissionLoader`
statics (`s_Loaded`) survive across play sessions within one Workbench process, so fixture edits are
only picked up by a **fresh Workbench start**. Verified procedure: kill the `ArmaReforgerWorkbenchSteamDiag.exe`
(Z:-prefixed cmdline), `wb_launch {"gprojPath": …addon.gproj}`, `wb_open_resource worlds/TBD_Dev_POC.ent`,
then `tbd-spawn-verify.sh` — one WB cycle per fixture/config change (`wb-cycle.sh` pattern, ~90 s).

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
