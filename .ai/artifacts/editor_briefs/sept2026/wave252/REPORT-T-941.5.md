# REPORT T-941.5 — Spectator range clamp

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.5
slice/T-941.5
```

Command: `pwd && git branch --show-current` (first action). EnfusionMCP already 19 files; no copy.

## defect_verified_on_main

Worktree tracked main at dispatch. Defect still present before the edit.

| claim | path:line | command |
|---|---|---|
| `m_fHostMaxRangeM` attribute defaults to 0 meaning unlimited | `apps/mod/tbd-framework/Scripts/Game/TBD/Spectator/TBD_SpectatorComponent.c:54-62` (export twin identical code, ASCII dashes) | `sed -n '54,74p' apps/mod/tbd-framework/Scripts/Game/TBD/Spectator/TBD_SpectatorComponent.c` |
| Authority passes the raw attribute into the host; 0 never becomes a finite leash | same file `:74` | `TBD_SpectatorHost.Start(m_sHostPrefab, m_fHostMaxRangeM)` |
| Host then treats `<= 0` as unlimited on every client position request | `TBD_SpectatorHost.c:820-823` (`if (s_fMaxRangeM <= 0) return wanted;`) | sibling, not owned |

Quoted (framework, pre-edit `:54-74`):

```
	//! T-181.24 — how far a spectator may steer their own streaming origin from where they died.
	//!
	//! 0 (the default) is unlimited, because watching the AO is the entire point of a spectator
	//! camera. The cost of unlimited is stated plainly in the `TBD_SpectatorHost` header: it is the
	//! engine's replication range that stops a MODIFIED client from seeing the enemy, and this
	//! feature moves that range on request. An operator who cares more about that than about
	//! spectator reach sets a number here.
	[Attribute("0", desc: "Max metres a spectator may steer their streaming host from their own death position. 0 = unlimited.")]
	protected float m_fHostMaxRangeM;
...
			TBD_SpectatorHost.Start(m_sHostPrefab, m_fHostMaxRangeM);
```

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Spectator/TBD_SpectatorComponent.c` | 38-40 | `DEFAULT_HOST_MAX_RANGE_M = 2000` dated T-941.5 (2026-09-07). |
| same | 58-68 | `:56-60` comment restated with the T-941.5 date; attribute default `"2000"`. 0 is the default, never unlimited. |
| same | 80 | Authority arms the host with `ClampHostMaxRangeM(m_fHostMaxRangeM)`, not the raw attribute. |
| same | 112-126 | `ClampHostMaxRangeM`: non-positive → 2000; otherwise `Math.Clamp(value, 0, ceiling)` so a type-broken clamp is compile-visible. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Spectator/TBD_SpectatorComponent.c` | same | Export twin; ASCII comment dashes preserved. Normalized-equal to framework. |

`TBD_SpectatorController.c` not touched (T-291). `TBD_SpectatorHost.c` not touched (not in owns). Position requests still go through `TBD_SpectatorHost.ClampToRange`; that path is live because this slice never arms `s_fMaxRangeM <= 0`.

## perturbation

Broke **both** twins: `return Math.Clamp(value, 0, ceiling);` → `return Math.Clamp(value, 0, "2000");` so lockstep could not mask a vacuous green.

**red_output VERBATIM** (`cargo xtask mod compile` after the string-literal clamp):

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Spectator/TBD_SpectatorComponent.c:126: Cannot convert 'string' to 'float' for argument '2' in method 'Clamp'
------------------------------------------------------------
1 error(s) in TBD sources, 5 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/ScenarioFramework/Actions/ActionGetters/SCR_ScenarioFrameworkGetLastFinishedTaskLayer.c:11: Can't find class SCR_ScenarioFrameworkParam
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:25: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Components/SCR_SpinningWidgetComponent.c:27: Can't find class SCR_SpinningWidgetAnimation
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:452: Can't find class Tuple2
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:517: Can't find class Tuple2
```

**restored_green:** restored `Math.Clamp(value, 0, ceiling)` in both twins, `touch` both, `cargo xtask mod compile`:

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11434x classes
    Compiling Game scripts took: 873.173000 ms
    0 warning(s) in TBD sources
```

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-941.5` (re-run on the code commit):

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

  gate verdict PASS @ 260c0cccbcdf recorded: .ai/artifacts/verdicts/T-941.5.json
SLICE GATE: PASS
```

## mod_compile_verdict

`OK: compiled clean` — 5755 files, 11434 classes, 0 TBD warnings. `cargo xtask mod world-boot` → `WORLD BOOT: PASS` (roll-call includes Spectator=ok). `cargo xtask schema validate` → All contracts valid. Did not restart :3000/:8080 (dedicated server on 21000+).

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Spectator/TBD_SpectatorHost.c:98` (export twin same) | Header still says `m_fHostMaxRangeM` is `0 = unlimited`. Behaviour is correct once this slice never passes `<= 0`; the comment is stale. Sibling file; T-941.5 owns only `TBD_SpectatorComponent.c`. |
| same `:226` | `Start` log still prints `range=%1 m (0 = unlimited)`. |
| same `:820-823` | `ClampToRange` still returns the unclamped client position when `s_fMaxRangeM <= 0`. A future caller of `TBD_SpectatorHost.Start(..., 0)` would restore unlimited. Only caller today is this component. |
| `docs/mod/TBD_MOD_DESIGN.md:149-156` | Design doc still describes default `0` as unlimited. Docs are out of owns; no docs edit this slice. |

## deviations

None. Position-request clamp stays in `TBD_SpectatorHost.ClampToRange` (not owned). The owned change is the finite leash the host is armed with, which is what makes that clamp fire.

## commits

- `260c0cccbcdf7ab124c8d67657ddad87fb99d532` — T-941.5: default spectator host range to 2000 m.

## manual_checklist

- Unset `m_fHostMaxRangeM` on the game-mode spectator component: streaming host arms at 2000 m (log must not say unlimited).
- Prefab or Workbench value `0`: still 2000 m, never unlimited.
- Server configured at 2000 m; spectator / modified client requests a host 10 km from death: host is held at 2 km from the death anchor.
- Server configured at 500 m: a 2 km request is held at 500 m.
- Positive operator value above 2000 (e.g. 4000) is honoured as the ceiling; it is not recapped to 2000.

## twins_confirmed

- `apps/mod/tbd-framework/Scripts/Game/TBD/Spectator/TBD_SpectatorComponent.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/Spectator/TBD_SpectatorComponent.c` — on disk
