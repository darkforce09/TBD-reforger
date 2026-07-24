# Slot-Body Materialization — session handoff (2026-07-24, Fable 5)

**Read with:** `CLAUDE.md` · plan `/home/Samuel/.claude/plans/read-claude-md-first-work-sorted-forest.md` ·
prior program log `.ai/artifacts/spawn_determinism_verify_log.md` · this file.
**Operator decisions (full-information, locked):** bodies created at MISSION LOAD from the
compiled JSON (not baked into the world — trade study in the plan); PS-style lobby
experience; re-equip every spawn; dynamic per-side slot counts (128 = player cap only).

## What is DONE and smoke-proven (single wb_play, log receipts in-session)

Architecture (CRF+PlayableSelector synthesis, operator's model): at LOADING→settle the
mod **materializes one numbered body per compiled slot** — kit prefab at the exact JSON
transform, AI disabled once (CRF pattern, no PS 500 ms hammer), Arsenal loadout applied
(`[TBD][Loadout][Slot]` tag) — all BEFORE the lobby. Deploy = claim + **bind onto the
standing body** via `SCR_PlayerController.SetInitialMainEntity` (PS `PS_PlayableManager.c:267`
+ CRF `CRF_PlayerHelper.c:40-43` precedent); the vanilla `RequestSpawn` pipeline (measured
double-spawn source) is never invoked; the reaper died with it. Death → next deploy finds
the body dead → **rematerializes a fresh dressed body** at the slot (corpse stays).
Smoke receipt: `[TBD][Slots] materialized 1 bodies (1 loadouts applied)` → `bound player 1
to slot s1 body` → `[TBD][Audit] characters=1 bodies=1 players=1`, zero errors.

Key code: `TBD_SpawnManager.c` (`MaterializeSlotBodies`/`SpawnSlotBody`/`DisableBodyAI`/
`DeployPlayerEx` bind path/`ClaimSlot` PS-shaped guard/`IsBodyDead`), menu-logic +
framework-manager renames (`AreSlotBodiesMaterialized`/`MaterializeSlotBodies`), harness
`scripts/mod/tbd-spawn-determinism.sh` updated (materialize sentinel, `characters==bodies`
census, bind-dup check, **broadened error grep — see trap #2**).

## OPEN DEFECTS (next session starts here)

1. **`set.Remove(index)` crash — FIXED IN CODE, COMPILE-PENDING.** VM exceptions
   ("Index out of bounds", `TBD_SpawnManager.c:461`, stack in logs_…18:44 session log)
   because EnforceScript `set<T>.Remove()` is BY INDEX. Both `set<int>` members converted
   to `map<int,bool>` (15 call sites, mechanical sed). **First action next session:
   restart WB → confirm 0 compile errors → smoke → 5-run gate**
   (`bash scripts/mod/tbd-spawn-determinism.sh 5` — restarts WB per run; coordinate with
   the operator, they interrupted two runs today while using the machine).
2. **Automatic faction cycling — UNDIAGNOSED.** 138 `player … has switched from faction`
   INFO lines in one session (~1 s cadence, cycling US/USSR/FIA/CIV), starting at world
   init and CONTINUING AFTER a successful bind, while the operator sat on the LOADING
   screen despite being bound. Evidence: session log `logs_…18:44…/console.log` lines
   ~407-474. Leads, in order: (a) the two VM exceptions aborted DeployPlayerEx mid-flight
   twice before the successful third attempt — re-test after the fix; (b) the vanilla
   deploy/loading menu never FINALIZES because SetInitialMainEntity bypasses its state
   machine → menu loops (loading screen + faction preview cycling that really mutates
   player faction). PlayableSelector's solution: **disable vanilla respawn/menu components
   on the player controller prefab** (`SCR_RespawnComponent { Enabled 0 }`,
   `SCR_PossessSpawnRequestComponent { Enabled 0 }` — see PS `DefaultPlayerControllerMP_Coop.et`)
   and run their own menu; CRF instead completes the vanilla flow. Likely fix = PS route +
   fast-track a minimal T-068.13 lobby, or find the menu-finalize API. PS clone for
   reference: `/tmp/claude-1000/…/scratchpad/PlayableSelector` (scratchpad — re-clone if
   gone: github.com/JiraF4/PlayableSelector; NO LICENSE — mirror design, never copy).
3. **`EngineFactionKey` lacks `indfor` (FIA) + civ mappings** (`TBD_SpawnManager.c`
   `EngineFactionKey`): returns "" → wrong/empty faction for FIA missions. Two-line add
   (`case "indfor": return "FIA";`), fold into the next compile.

## Landmines (all measured this program — do not relearn)

`wb_reload` never recompiles; restart WB (kill `pkill -f "WorkbenchSteamD[i]ag"`, Steam
auto-relaunches; game-dead boots happen — probe `wb_open_resource` and cycle) ·
loader statics survive re-play within one WB process (restart per gate run) ·
"Can't initialize the game" dialog = usually a Game-module COMPILE ERROR — read the boot
log's `Compiling Game scripts` section first · `wb_script_editor` is 0-based ·
Enforce `set/array.Remove` = by-index (use `map<K,bool>`/`RemoveItem`) · VM exceptions
have no `[TBD]` tag (harness now greps them) · backend for tests: `make db-up && make api`;
mission `6d291619-8182-4164-866d-4e165a5516af` v0.1.3 is the verify mission
($profile TBD_BackendConfig.json points at it, SERVICE_TOKEN from api/.env).

## After the gate passes

Verify-log addendum (spawn_determinism_verify_log.md) + Cursor mints tickets/tags for:
materialization program commit(s) · then T-068.13 lobby UI (PS faction→group→role tree as
UX reference, ClaimSlot backend ready) · T-114 roster · dedicated/JIP manual gate.
