# REPORT T-941.4 — Objective HUD and capture bar replace chat

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.4
slice/T-941.4
```

Command: `pwd && git branch --show-current` (first action). EnfusionMCP already 19 files; no copy.

## defect_verified_on_main

Worktree tracked main at dispatch (`8c4669ca9`). Defect still present before the edit.

| claim | path:line | command |
|---|---|---|
| Channel documented as private chat | `TBD_ObjectivesComponent.c:32` | `sed -n '29,35p'` |
| Per-player pump | `:816` (`chat.SendPrivateMessage(text, playerId)`) | `sed -n '806,816p'` |
| Tick delivery walks every broadcast + every inside pending line | `:729-749` `Deliver` | `rg Tell\|m_aBroadcasts\|PendingInside` |
| No HUD script or layout | `Scripts/Game/TBD/UI/Hud/` and `UI/layouts/TBD_ObjectiveHud.layout` absent | `ls` before the edit |

Quoted (class header, pre-edit):

```
//! The channel is `SCR_ChatComponent.SendPrivateMessage`, the same per-player server->client path
//! `TBD_PlayAreaComponent` already uses, chosen over a HUD because a new `.layout` is INVISIBLE to
//! the engine until Workbench rewrites `resourceDatabase.rdb` (recorded landmine) — a widget
//! written on this lane could not open.
```

Quoted (`Tell`, pre-edit `:816`):

```
		chat.SendPrivateMessage(text, playerId);
```

`Deliver` pushed every `m_aBroadcasts` line and every `m_sPendingInsideMessage` (progress %, contested) to chat at 1 Hz. Spec line `:813` had drifted to `:816` on this tree.

## changes

| path | why |
|---|---|
| `apps/mod/tbd-framework/UI/layouts/TBD_ObjectiveHud.layout` (+ export twin) | Corner HUD: list + `CaptureBar`/`CaptureFill`. C3 geometry agrees. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/TBD_ObjectiveHud.c` (+ export twin) | Overlay HUD. Layout via HUD-local `LAYOUT` constant (not `TBD_UILayouts.c`). Owner RPC on `modded class SCR_PlayerController`. State icons in the list; bar bound to the local player's capture objective. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectivesComponent.c` (+ export twin) | Replicate board + capture percent at the 1 Hz tick. Drop the per-tick chat pump. Chat keeps only CAPTURED / DESTROYED / HELD. Hide HUD when leaving LIVE. |

New identifier `objectives` was avoided in added code so T-706 unread-wire baseline stays 13.

## perturbation

Broke `Find("CaptureFill")` to `Find(CaptureFill)` in **both** trees so lockstep could not mask a vacuous green.

**red_output VERBATIM** (`cargo xtask mod compile` after the missing widget name):

```
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/UI/Hud/TBD_ObjectiveHud.c:112: Can't find variable 'CaptureFill'
------------------------------------------------------------
1 error(s) in TBD sources, 19 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:324: Error in parameters
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:333: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:335: Can't find variable 'sortSlots'
  Scripts/Game/GameMode/Loadout/SCR_PlayerArsenalLoadout.c:337: Can't find variable 'sortSlots'
```

**restored_green:** quotes restored, files `touch`ed, `cargo xtask mod compile` → `OK: compiled clean` (5759 files, 0 TBD warnings).

## gate_verdict_tail

`cargo xtask platform wave gate --slice T-941.4`

```
  no-python (T-620)        PASS

  gate verdict PASS @ 8c4669ca97a4 recorded: .ai/artifacts/verdicts/T-941.4.json
SLICE GATE: PASS
```

(`8c4669ca97a4` is the dispatch parent; this report commits after the gate.)

`cargo xtask schema validate` → `All contracts valid.` (not `ci schema-validate`).

## mod_compile_verdict

GREEN after restore: `OK: compiled clean` / `0 warning(s) in TBD sources`.

`cargo xtask mod world-boot` → `WORLD BOOT: PASS` (roll-call `Objectives=ok`).

`cargo xtask verify ui-layouts` → `OK  TBD_ObjectiveHud.layout` / widget-name contract 22 names.

## files_outside_owns

[]

(Required report only: `.ai/artifacts/editor_briefs/sept2026/wave253/REPORT-T-941.4.md`.)

## found_not_fixed

- Layout GUID is not in `resourceDatabase.rdb` until the first Workbench pass. `TBD_UILayouts.Create` falls back to the bare path (same as T-941.3). Headless compile does not index `.layout` files.
- `world-boot` loads zero players and stays in LOBBY, so the HUD is never instantiated on that lane. In-game bar fill / icon change / complete-only chat is the human checklist, not a gate observation.
- HUD root is a workspace overlay without a proven ignore-cursor flag; if it steals clicks outside the panel, that needs a live client to see.

## deviations

- `tbd-export` twins are ASCII-transliterated (em-dash / box-drawing → ASCII) because the export non-ASCII scan fails closed. After transliteration, framework and export are functionally identical (`norm(framework)==export`).
- Did not edit `TBD_UILayouts.c` (T-941.3). HUD registers `{7BD1A70000000A01}UI/layouts/TBD_ObjectiveHud.layout` on its own constant.

## commits

See git log on `slice/T-941.4` after this report is committed.

## manual_checklist

- Enter a capture zone: capture bar appears and fills.
- Complete a capture / destroy / hold: list icon changes; chat shows one complete line (`CAPTURED` / `DESTROYED` / `HELD`).
- While capturing: no per-tick percent / contested / hold-mark chat lines.
- Leave LIVE: HUD closes.

## twins_confirmed

| pair | result |
|---|---|
| `TBD_ObjectiveHud.layout` framework/export | `cmp -s` identical |
| `TBD_ObjectiveHud.c` | identical after ASCII transliteration of export |
| `TBD_ObjectivesComponent.c` | identical after ASCII transliteration of export |
