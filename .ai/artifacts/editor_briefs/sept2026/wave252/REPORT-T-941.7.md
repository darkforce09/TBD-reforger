# REPORT T-941.7 — Radio backbone fallback

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.7
slice/T-941.7
```

Command: `pwd && git branch --show-current` (first action). EnfusionMCP already 19 files; no copy.

## defect_verified_on_main

Worktree tracked `b59b99116` at dispatch. Defect still present before the edit.

| claim | path:line | command |
|---|---|---|
| `GetBackbone()` null refuses to tune | `apps/mod/tbd-framework/Scripts/Game/TBD/Radio/TBD_RadioTuner.c:140-145` (export twin identical code) | `git show HEAD:apps/mod/tbd-framework/Scripts/Game/TBD/Radio/TBD_RadioTuner.c \| sed -n '140,145p'` |
| Boot warning does not name fallback or say radios still tune | `TBD_RadioComponent.c:125-136` | `git show HEAD:…/TBD_RadioComponent.c \| sed -n '125,136p'` |

Quoted (framework, pre-edit):

```
		if (!IsBackboneAvailable())
		{
			report.m_eResult = TBD_ERadioTuneResult.NO_BACKBONE;
			report.m_sDetail = "world has no RadioManagerEntity";
			return report;
		}
```

```
		TBD_Log.Warn(TBD_RadioPlan.CH_RADIO,
			"backbone: MISSING — this world has no RadioManagerEntity, so the engine supports NO BaseRadioComponent on it and no frequency can be set from script. Net ASSIGNMENT and DISPLAY still work; automatic TUNING does not. Fix is a world edit in Workbench (place a RadioManagerEntity in worlds/TBD_Dev_POC.ent), not a script change.");
```

`NO_BACKBONE` left radios untunable. The warning named the entity and the `.ent` file but said "not a script change" and did not name a fallback table.

## changes

| path | what |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Radio/TBD_RadioTuner.c` | `TBD_RadioFallbackTable` + `FallbackChannelTable` (radioPlan copy-through, else defaults 42000/41000 kHz when backbone missing). `TunePlayer` no longer early-returns `NO_BACKBONE`; it still `SetFrequency` + read-back. `WorldFileName` / `FallbackSourceName` for the boot warning. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Radio/TBD_RadioTuner.c` | Export twin; ASCII comments/string dashes preserved. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Radio/TBD_RadioComponent.c` | Once-per-world warning (`s_bBackboneReported`, cleared in `OnDelete`) names world, `RadioManagerEntity`, and fallback (`radioPlan` \| `defaults`). |
| `apps/mod/tbd-export/Scripts/Game/TBD/Radio/TBD_RadioComponent.c` | Export twin. |

No `.ent` edit (operator deferral 2026-09-04, quoted).

Boot evidence (`cargo xtask mod world-boot --keep-logs`):

```
SCRIPT    (W): [TBD][Radio] backbone: MISSING — world='worlds/TBD_Dev_POC.ent' has no RadioManagerEntity; using script-side channel table (defaults). Add RadioManagerEntity in Workbench (worlds/TBD_Dev_POC.ent) to restore the engine backbone. T-941.7 fallback is in use; the world edit is on the operator checklist.
```

One line, once. POC boot had no mission loaded, so fallback source is `defaults`.

## perturbation

Broke the **definition** name in **both** trees to `FallbackChannelTableRenamed` so `TunePlayer` still called `FallbackChannelTable`. Lockstep could not mask a vacuous green.

**red_output VERBATIM** (`cargo xtask mod compile` after the rename):

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Radio/TBD_RadioTuner.c:227: Undefined function 'TBD_RadioTuner.FallbackChannelTable'
------------------------------------------------------------
1 error(s) in TBD sources, 13 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:324: Error in parameters
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:333: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:335: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Syntax error
  Scripts/Game/Map/ComponentsUI/SCR_MapUIElementContainer.c:443: Incompatible parameter 'on delete'
  Scripts/Game/Respawn/Logic/SCR_SpawnLogic.c:298: Can't find class Tuple1
  Scripts/Game/ScenarioFramework/Actions/ActionGetters/SCR_ScenarioFrameworkGetCountEntitiesInTrigger.c:10: Can't find class SCR_ScenarioFrameworkParam
  Scripts/Game/ScenarioFramework/Actions/ActionGetters/SCR_ScenarioFrameworkGetLastFinishedTaskLayer.c:11: Can't find class SCR_ScenarioFrameworkParam
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:25: Can't find class SCR_SpinningWidgetAnimation
  … 3 more
```

**restored_green:** restore both definition names, `touch` all four owns files, `cargo xtask mod compile`:

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11435x classes
    Compiling Game scripts took: 885.211000 ms
    0 warning(s) in TBD sources
```

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-941.7` (lock wait ~840s, then PASS):

```
  T-439 objects aliases    PASS
  T-444 wiki seed          PASS
  T-440 faction library seed PASS
  T-438 deploy-staging     PASS
  T-456 REST size gate     PASS
  T-468 CI schema parity   PASS
  T-437 destroy inert      PASS
  T-586 route tags         PASS
  T-296 reporter identity  PASS
  T-452 player identity    PASS
  no-python (T-620)        PASS

  gate verdict PASS @ b59b99116a09 recorded: .ai/artifacts/verdicts/T-941.7.json
SLICE GATE: PASS
```

Verdict SHA is the pre-commit worktree HEAD (`b59b99116`). Code commit below is `dcc6ea627`. `cargo xtask schema validate`: all contracts valid.

## mod_compile_verdict

`OK: compiled clean` — 5755 files, 11435 classes, 0 TBD warnings. `cargo xtask mod world-boot` → `WORLD BOOT: PASS` (roll-call includes `Radio=ok`). Did not restart :3000/:8080.

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| `TBD_RadioClient.c` `TuneLine()` (`s_sTuneResult == "NO_BACKBONE"`) | Client still says "dial these in by hand" if the server ever emits `NO_BACKBONE`. This slice no longer assigns that result. Sibling file; not in owns. |
| Engine `DEFAULT (W): World doesn't contain RadioManagerEntity…` | Still printed on POC boot. Expected until the deferred world edit. |
| `docs/platform/PLAYTEST_RUNBOOK.md` radio row | Still says automatic tuning will NOT work. Docs are command-center, not this slice. |

## deviations

- Wave brief perturbation is **rename** (`FallbackChannelTable` → `FallbackChannelTableRenamed`). Ticket text said "wrong return type"; the brief is what this agent followed.
- `TunePlayer` always calls `FallbackChannelTable` (copy-through when the backbone exists) so the rename is load-bearing even on worlds that already have `RadioManagerEntity`.
- `NO_BACKBONE` remains in the enum for the wire/client contract; it is no longer the refuse-to-tune outcome.
- `cargo test -p map-engine-core --all-features`: 1011 passed, 1 failed (`dem::peaks::tests::everon_peaks_max_above_350` — `Invalid PNG signature`). Worktree LFS pointer, not this slice. Brief already says use `cargo xtask schema validate` for LFS worktrees.
- Operator deferral (quoted): RadioManagerEntity world edit — deferred by operator 2026-09-04. Checklist only.

## commits

- `dcc6ea6276cf4ae71fc9a61053a5a588eb12dd70` — T-941.7: script-side radio fallback when backbone is missing.

## manual_checklist

In-game (gate does not stand in for this):

- Two players on `TBD_Dev_POC` with no `RadioManagerEntity`: radios auto-tune to the same fallback channel (mission `radioPlan` when the mission has accepted nets, else 42.000 MHz handheld / 41.000 MHz long-range) and hear each other.
- Console shows **one** `[TBD][Radio]` warning naming the world, `RadioManagerEntity`, and `radioPlan` or `defaults`.
- A world that already has `RadioManagerEntity`: warning is the `ok` backbone line; tuning uses the caller's `radioPlan` frequencies as before.

Operator checklist (deferred world edit, quoted 2026-09-04):

1. Open Workbench.
2. Open `worlds/TBD_Dev_POC.ent`.
3. Place a `RadioManagerEntity`.
4. Save.
5. Re-run `cargo xtask mod world-boot`. Expect the engine `RadioManagerEntity` warning gone and `[TBD][Radio] backbone ok`.

## twins_confirmed

- `apps/mod/tbd-framework/Scripts/Game/TBD/Radio/TBD_RadioTuner.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/Radio/TBD_RadioTuner.c` — on disk
- `apps/mod/tbd-framework/Scripts/Game/TBD/Radio/TBD_RadioComponent.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/Radio/TBD_RadioComponent.c` — on disk
