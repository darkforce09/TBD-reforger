# REPORT T-941.3 — END and DEBRIEF screens

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.3
slice/T-941.3
```

Command: `pwd && git branch --show-current` (first action). EnfusionMCP already 19 files; no copy.

## defect_verified_on_main

Worktree tracked main at dispatch. Defect still present before the edit.

| claim | path:line | command |
|---|---|---|
| Only `SCREEN_SHELL` and `LIST_ROW` registered; no END/DEBRIEF layouts | `apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_UILayouts.c:28-31` (export twin identical constants) | `sed -n '26,34p'` on `HEAD~1` |
| `SetStage` entered LOBBY/BRIEFING/LIVE only; END/DEBRIEF had no screen hook | `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c:977-995` (export twin same dispatch) | `sed -n '970,996p'` on `HEAD~1` |
| No `UI/End/` scripts; layouts dir had only shell + list-row | `apps/mod/tbd-framework/Scripts/Game/TBD/UI/` and `UI/layouts/` | `ls` before the edit |

Quoted (`TBD_UILayouts.c`, pre-edit):

```
	static const ResourceName SCREEN_SHELL = "{7BD1A70000000701}UI/layouts/TBD_ScreenShell.layout";

	//! One pooled row of a TBD_ListBox.
	static const ResourceName LIST_ROW     = "{7BD1A70000000702}UI/layouts/TBD_ListRow.layout";
```

Quoted (`SetStage` dispatch, pre-edit):

```
		if (stage == TBD_EGameStage.LOBBY)
			OnEnterLobby();
		else if (stage == TBD_EGameStage.BRIEFING)
			OnEnterBriefing();
		else if (stage == TBD_EGameStage.LIVE)
			OnEnterLive();
	}
```

`NotifyLocalStageUI` forwarded only to `pc.TBD_OnStageChanged` (briefing). A mission ending was a bare stage change.

Spec line numbers 1022/1123/1162/1295 had drifted; the live holes were the missing OnEnterEND/DEBRIEF arms and the two-constant layout registry.

## changes

| path | why |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_UILayouts.c` (+ export twin) | Register `END_SCREEN` and `DEBRIEF_SCREEN`. |
| `apps/mod/tbd-framework/UI/layouts/TBD_EndScreen.layout` (+ export twin) | Shell chrome + `Winner` / `Reason` widgets. Reuses list-row via the shell `TBD_ListBox` default. |
| `apps/mod/tbd-framework/UI/layouts/TBD_DebriefScreen.layout` (+ export twin) | Shell chrome; empty-state "No players recorded." |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/End/TBD_EndScreen.c` (+ export twin) | Overlay: winner + reason. `Open`/`Close` cannot refuse `SetStage`. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/End/TBD_DebriefScreen.c` (+ export twin) | Overlay scoreboard, sorted by kills (toggle via `SORT BY KILLS` / header row). `TBD_ResultsReporter.FillScoreboard` as a `modded class` (T-940.4 keeps the reporter file). |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c` (+ export twin) | `OnEnterEnd` / `OnEnterDebrief`; `ApplyEndScreens` from `NotifyLocalStageUI`; replicated winner/reason/board; kill map on `OnPlayerKilled` while LIVE. |

Screens are workspace overlays (`TBD_UILayouts.Create`), not Chimera menus: this slice does not own `chimeraMenus.conf`, and a menu that swallows Esc must not be able to block a stage change.

## perturbation

Broke `FindText("Winner")` to `FindText(Winner)` in **both** trees so lockstep could not mask a vacuous green.

**red_output VERBATIM** (`cargo xtask mod compile` after the missing widget name):

```
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/UI/End/TBD_EndScreen.c:67: Can't find variable 'Winner'
------------------------------------------------------------
1 error(s) in TBD sources, 19 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:324: Error in parameters
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:333: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:335: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Syntax error
  Scripts/Game/Map/ComponentsUI/SCR_MapUIElementContainer.c:443: Incompatible parameter 'on delete'
  Scripts/Game/Plugins/Persistence/System/Serializers/Entities/SCR_FactionManagerSerializer.c:26: Error in parameters
  Scripts/Game/Plugins/Persistence/System/Serializers/Entities/SCR_FactionManagerSerializer.c:29: Can't find variable 'friendlyDefaultMapping'
  Scripts/Game/Plugins/Persistence/System/Serializers/Entities/SCR_FactionManagerSerializer.c:42: Can't find variable 'friendlyDefaultMapping'
  Scripts/Game/Plugins/Persistence/System/Serializers/Entities/SCR_FactionManagerSerializer.c:52: Can't find variable 'friendlyDefaultMapping'
  … 9 more
```

**restored_green:** restored `FindText("Winner")` in both twins, `touch` both, `cargo xtask mod compile`:

```
OK: compiled clean
    Module: Game; loaded 5757x files; 11440x classes
    Compiling Game scripts took: 871.145000 ms
    0 warning(s) in TBD sources
```

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-941.3` (lock wait ~840s, then):

```
  T-440 faction library seed PASS
  T-438 deploy-staging     PASS
  T-456 REST size gate     PASS
  T-468 CI schema parity   PASS
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 43c90bf9a691 recorded: .ai/artifacts/verdicts/T-941.3.json
SLICE GATE: PASS
```

## mod_compile_verdict

`OK: compiled clean` — 5757 files, 11440 classes, 0 TBD warnings. `cargo xtask mod world-boot` → `WORLD BOOT: PASS` (roll-call SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok PlayArea=ok Markers=ok Radio=ok Objectives=ok). Did not restart :3000/:8080.

`cargo xtask schema validate` PASS. `cargo test -p map-engine-core --all-features`: 1011 passed, 1 failed (`dem::peaks::tests::everon_peaks_max_above_350` — `Invalid PNG signature`, worktree LFS pointer; this slice did not touch DEM). Slice gate did not fail that arm (no `.rs` in the diff).

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_ResultsReporter.c:7-22` (export twin same; T-940.4 owns the file) | Reporter still omits kill tracking (`kills` absent from the POST, not a live counter). T-940.4 is nested-vs-flat JSON, not a kill feed. DEBRIEF reads deaths from the reporter's ONE LIFE source (`SpawnManager.IsPlayerDead`) via `FillScoreboard`; kills are counted on `TBD_FrameworkManager.OnPlayerKilled` while LIVE. |
| `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` (not owned) | No `TBD_UIEnd` / `TBD_UIDebrief` presets. Screens are workspace overlays. First Workbench pass still required for GUID index; `TBD_UILayouts.Create` falls back to the bare path. |

## deviations

Overlays instead of `TBD_MenuStack.Open(ChimeraMenuPreset…)` because `chimeraMenus.conf` is outside owns and a Chimera menu can swallow input. `ApplyEndScreens` is local widget ops only and cannot refuse `SetStage`. Kill credits live on the owned game-mode component rather than rewriting `TBD_ResultsReporter.c`.

## commits

- `43c90bf9a6915f68ad70fa21404de7214de05d60` — T-941.3: add END and DEBRIEF overlay screens with scoreboard.

## manual_checklist

- End a mission (clock, elimination, objective, or `#tbd stage END`): END overlay titles **MISSION ENDED**, names the winning faction (or "No winner") and the reason; dedicated-server log has `[TBD] END - winner=… reason=…`.
- Advance to DEBRIEF: END overlay closes; DEBRIEF lists every connected player with kills / deaths, default-sorted by kills high-first; `SORT BY KILLS` (or the header row) toggles direction.
- Next stage (LOBBY / named jump): both overlays close. Dismissing with BACK must not freeze the stage machine.
- A player kill during LIVE increments that player's DEBRIEF kill cell; world/AI/self kills do not.

## twins_confirmed

- `apps/mod/tbd-framework/Scripts/Game/TBD/UI/TBD_UILayouts.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/UI/TBD_UILayouts.c` — on disk
- `apps/mod/tbd-framework/Scripts/Game/TBD/UI/End/TBD_EndScreen.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/UI/End/TBD_EndScreen.c` — on disk
- `apps/mod/tbd-framework/Scripts/Game/TBD/UI/End/TBD_DebriefScreen.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/UI/End/TBD_DebriefScreen.c` — on disk
- `apps/mod/tbd-framework/UI/layouts/TBD_EndScreen.layout` — on disk
- `apps/mod/tbd-export/UI/layouts/TBD_EndScreen.layout` — on disk
- `apps/mod/tbd-framework/UI/layouts/TBD_DebriefScreen.layout` — on disk
- `apps/mod/tbd-export/UI/layouts/TBD_DebriefScreen.layout` — on disk
- `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c` — on disk
