# REPORT T-941.1 — Safestart armed during LOBBY and BRIEFING

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.1
slice/T-941.1
```

Command: `pwd && git branch --show-current` (first action). EnfusionMCP already 19 files; no copy.

## defect_verified_on_main

Worktree tracked main at dispatch. Defect still present before the edit.

| claim | path:line | command |
|---|---|---|
| Arm predicate is SAFE_START only; anything else Lift()s | `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SafestartManager.c:248-264` (export twin identical code) | `sed -n '247,265p' apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SafestartManager.c` |

Quoted (framework, pre-edit):

```
//! relevant. That is deliberate: SAFE_START arms, and literally anything else lifts. An admin
//! who jumps SAFE_START -> END, or restarts the round back to LOBBY, must not leave a server
//! full of invulnerable players behind.
...
		if (stage == TBD_EGameStage.SAFE_START)
		{
			Arm();
			return;
		}

		Lift(typename.EnumToString(TBD_EGameStage, stage));
```

Armed stage set **before**: `{SAFE_START}` only. LOBBY auto-deploy therefore spawned unshielded bodies.

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SafestartManager.c` | 247-251 | Dated decision comment names T-941.1 (2026-09-07) and lobby auto-deploy (~250 ms). |
| same | 259 | Arm predicate widened to `LOBBY \|\| BRIEFING \|\| SAFE_START`. Any other stage (LIVE, END, DEBRIEF, LOADING) still `Lift()`s once. |
| same | 270-333 | `Arm()` is idempotent for the shield (`if (first)`). Countdown + "Live in" broadcast start only when `GetStage() == SAFE_START` and `m_iSecondsRemaining == NOT_RUNNING`. `TickSweep` still starts on first arm so late joiners during BRIEFING are covered. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SafestartManager.c` | same | Export twin; ASCII comments/string dashes preserved. |

`TickCountdown` body not edited (three exits remain). `TBD_SpawnManager.c` not touched.

## perturbation

Broke the predicate in **both** trees to `stage == "LOBBY" || ...` so lockstep could not mask a vacuous green.

**red_output VERBATIM** (`cargo xtask mod compile` after the string-literal compare):

```
   Compiling map-engine-core v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.1/crates/map-engine-core)
   Compiling tbd-gate v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.1/crates/tbd-gate)
   Compiling tbd-tickets v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.1/crates/tbd-tickets)
   Compiling tbd-tools v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.1/tools/tbd-tools)
   Compiling xtask v0.1.0 (/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.1/xtask)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.19s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Gamemode/TBD_SafestartManager.c:259: Incompatible parameter 'stage'
------------------------------------------------------------
1 error(s) in TBD sources, 4 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:25: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:27: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:452: Can't find class Tuple2
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:517: Can't find class Tuple2
```

**restored_green:** `git checkout --` both twins, `touch` both, `cargo xtask mod compile`:

```
OK: compiled clean
    Module: Game; loaded 5752x files; 11415x classes
    Compiling Game scripts took: 851.853000 ms
    0 warning(s) in TBD sources
```

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-941.1`:

```
  db_migrate persist       PASS
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

  gate verdict PASS @ a516cab3fcf4 recorded: .ai/artifacts/verdicts/T-941.1.json
SLICE GATE: PASS
```

## mod_compile_verdict

`OK: compiled clean` — 5752 files, 11415 classes, 0 TBD warnings. `cargo xtask mod world-boot` → `WORLD BOOT: PASS` (roll-call includes Safestart=ok). Did not restart :3000/:8080 (dedicated server on 21000+).

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_FrameworkManager.c:787-789` (export twin same) | Stale comment still says "SAFE_START arms the safestart and anything else lifts it". Call is still `safestart.OnStageChanged(stage)` for every transition — behaviour is correct; comment is wrong. Sibling file; T-941.1 owns only `TBD_SafestartManager.c`. |

## deviations

Arm() now starts `TickCountdown` only on SAFE_START. The brief said do not touch the countdown **loop**; `TickCountdown()` itself is unchanged (three exits at `!m_bArmed`, `GetStage() != SAFE_START` drift lift, `next <= 0` → GoLive). If Arm() still started the countdown on LOBBY, that untouched drift check would `Lift("stage-drift")` ~1 s later and the shield would be a dead mechanism. Command that forced the Arm() split: `sed -n '389,417p'` of the pre-edit `TickCountdown` (`framework.GetStage() != TBD_EGameStage.SAFE_START`).

## commits

- `a516cab3fcf4d3f4c425397b0a8a8ef9cb03f8c3` — T-941.1: arm safestart shield in LOBBY and BRIEFING.

## manual_checklist

- Join a dedicated server, auto-deploy during LOBBY, throw a grenade at your feet: no damage.
- Stay in LOBBY then BRIEFING for more than one second: still no damage (shield must not drop to the old stage-drift lift).
- Advance LOBBY → BRIEFING → SAFE_START: shield stays up; the "SAFESTART — damage OFF ... Live in" chat/countdown starts at SAFE_START, not in LOBBY.
- After GoLive / LIVE: a grenade at your feet deals damage.
- Admin `#stage END` from LOBBY/BRIEFING/SAFE_START: damage handling returns (no leftover invulnerable bodies).

## twins_confirmed

- `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SafestartManager.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SafestartManager.c` — on disk
