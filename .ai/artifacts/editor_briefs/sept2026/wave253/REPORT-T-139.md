# REPORT T-139 — Lobby loadout visual preview

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-139
slice/T-139
```

Command: `pwd && git branch --show-current` (first action). EnfusionMCP already 19 files; no copy.

## defect_verified

Worktree at dispatch HEAD `9f4a8c7d1`. Defect present before the edit.

| claim | evidence |
|---|---|
| No `TBD_LoadoutPreview.c` in either tree | `test ! -f` both paths → ABSENT |
| No `TBD_LoadoutPreview.layout` | `ls apps/mod/tbd-framework/UI/layouts/` had only shell, list-row, END, DEBRIEF |
| Lobby lists seats only | `TBD_LobbyScreen.c` had no `LoadoutPreview` / KIT hook |

The lobby still showed slot names, holder, OPEN/HELD/DEAD. A player could not see the kit before taking the seat.

## changes

| path | why |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Lobby/TBD_LoadoutPreview.c` (+ export twin) | New widget. `Refresh(TBD_MissionSlotStruct, int)` builds up to 13 icon cells from the scalar gear fields; empty fields skipped; missing UIInfo icon → letter glyph. Logs `[TBD][LoadoutPreview] slot=<n> items=<k>`. HUD-local `LAYOUT` ResourceName; `TBD_UILayouts.Create` then `CreateWidgets` fallback. Did **not** edit `TBD_UILayouts.c`. |
| `apps/mod/tbd-framework/UI/layouts/TBD_LoadoutPreview.layout` (+ export twin) | Right-column frame, `PreviewTitle` / `PreviewEmpty` / `IconGrid` of 13 ListRow-style cells (`Cell0`–`Cell12`). |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Lobby/TBD_LobbyScreen.c` (+ export twin) | Host preview beside the slot list (`FrameSlot` shrink of `List`); `GetOnHighlight` updates the grid without claiming; slot activate also refreshes. Reads `TBD_MissionLoader.GetSlotById` (local, no new RPC). |

## perturbation

Broke `FindAnyWidget("PreviewTitle")` to `FindAnyWidget(PreviewTitle)` in **both** trees so lockstep could not mask a vacuous green.

**red_output VERBATIM** (`cargo xtask mod compile` after the missing widget name):

```
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 40.74s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/UI/Lobby/TBD_LoadoutPreview.c:47: Can't find variable 'PreviewTitle'
------------------------------------------------------------
1 error(s) in TBD sources, 2 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:452: Can't find class Tuple2
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:517: Can't find class Tuple2
```

**restored_green:** restored `FindAnyWidget("PreviewTitle")` in both twins, `touch` both, `cargo xtask mod compile`:

```
OK: compiled clean
    Module: Game; loaded 5759x files; 11465x classes
    Compiling Game scripts took: 891.867000 ms
    0 warning(s) in TBD sources
```

## gate_verdict_tail

Last lines of `cargo xtask platform wave gate --slice T-139`:

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

  gate verdict PASS @ 9f4a8c7d1dff recorded: .ai/artifacts/verdicts/T-139.json
SLICE GATE: PASS
```

## mod_compile_verdict

`OK: compiled clean` — 5759 files, 11465 classes, 0 TBD warnings.

`cargo xtask verify ui-layouts` → PASSED (5 layouts; 22 bound names).

`cargo xtask schema validate` → All contracts valid.

`cargo xtask mod world-boot` → `WORLD BOOT: PASS` (roll-call SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok PlayArea=ok Markers=ok Radio=ok Objectives=ok). Did not restart :3000/:8080.

## files_outside_owns

[]

## found_not_fixed

- `TBD_LobbySlot` still has no gear fields (`TBD_LobbyData.c` is not in this slice's owns). Preview looks up `TBD_MissionLoader.GetSlotById(key)`. That is local file/state, not a new lobby RPC, but the lobby header documents MissionLoader as server-only. On a thin dedicated-server client where the mission document is not loaded, the grid will stay empty (`items=0`) until that document exists locally.
- 2D mannequin stretch is out of this ticket's acceptance (plan); icon grid only.
- Gear prefabs without a UIInfo `Icon` / `m_Image` show a letter glyph instead of a texture (planned placeholder).
- `TBD_LoadoutPreview.layout` GUID is not in `resourceDatabase.rdb` until a Workbench pass (same class of first-run as T-941.3); `Create` retries the bare path.
- Attachments[] and cargo rows are not among the 13 scalar gear fields and are not drawn.

## commits

See git log on `slice/T-139` after this report is committed. Explicit paths only; EnfusionMCP not staged.

## twins_confirmed

`TBD_LoadoutPreview.c` byte-identical across trees. `TBD_LoadoutPreview.layout` byte-identical. `TBD_LobbyScreen.c` logic lockstep (export twin keeps its pre-existing ASCII comment style). `TBD_UILayouts.c` untouched.
