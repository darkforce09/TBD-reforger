# REPORT T-941.6 — Account-link off public chat

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.6
slice/T-941.6
```

Command: `pwd && git branch --show-current` (first action). EnfusionMCP already 19 files; no copy.

## defect_verified_on_main

Worktree tracked main at dispatch (`b59b99116` T-942 wave 252 briefs). Defect still present before the edit.

| claim | path:line | command |
|---|---|---|
| `super.OnNewMessage` runs before the link handler | `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_AdminCommands.c:32` then `:45` (export twin identical code, ASCII dashes) | `sed -n '30,46p' apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_AdminCommands.c` |
| IdentityLink entry is `TryHandleChat` | `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_IdentityLink.c:162` | `rg -n 'static bool TryHandleChat'` |
| Header admitted the leak | same file `:48-55` | `sed -n '47,56p'` |

Quoted (framework, pre-edit):

```
	override void OnNewMessage(string msg, int channelId, int senderId)
	{
		super.OnNewMessage(msg, channelId, senderId);
		...
		if (TBD_IdentityLink.TryHandleChat(this, msg, senderId))
			return;
```

`#tbd link <code>` therefore reached `SCR_ChatPanelManager` (public chat) before TBD handled it. Replies were already `SendPrivateMessage`; the broadcast of the typed line was the leak.

## changes

| path | line | why |
|---|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_AdminCommands.c` | 32-40 | T-941.6 consume-before-broadcast: `TryConsumeBeforeBroadcast` runs **before** `super.OnNewMessage`. TRUE → return (no super, no public echo). Authority flag is `RplSession.Mode() != RplMode.Client`. |
| same | (removed :45) | Post-super `TryHandleChat` deleted so the link path cannot double-handle or race the admin gate. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_AdminCommands.c` | same | Export twin; ASCII comments/string dashes preserved. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_IdentityLink.c` | 47-56 | Header rewritten: consume-before-super, private replies, no filter beyond `#tbd link`. |
| same | 149-172 | `TryConsumeBeforeBroadcast` + `IsLinkCommand` (prefix only; `#tbd Link` folded). Authority calls `TryHandleChat`; every peer still returns TRUE so super is skipped. |
| same | 241 | Usage no longer claims public visibility. |
| same | 324 | Synthetic-identity refusal no longer says the code was visible in chat. |
| same | 650 | `NewCodeAdvice`: "not consumed", not "typed in public chat". |
| same | 757, 767 | Unchanged: `SendPrivateMessage` for sync and async replies. Code is POSTed in `BuildPayload`, never put in a chat string. |
| `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_IdentityLink.c` | same | Export twin; ASCII comments/string dashes preserved. |

Bare tokens without the `#tbd link` prefix remain ordinary public chat.

## perturbation

Broke the consume-before-broadcast **call** in **both** trees to `TryConsumeBeforeBroadcastBroken` so lockstep could not mask a vacuous green.

**red_output VERBATIM** (`cargo xtask mod compile` after the misspelled guard):

```
    Blocking waiting for file lock on artifact directory
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.72s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Gamemode/TBD_AdminCommands.c:37: Undefined function 'TBD_IdentityLink.TryConsumeBeforeBroadcastBroken'
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

**restored_green:** restored the method name in both twins, `touch` all four owns files, `cargo xtask mod compile`:

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11434x classes
    Compiling Game scripts took: 850.656000 ms
    0 warning(s) in TBD sources
```

(First green, before perturbation, was the same verdict at 877.078000 ms, 5755 files / 11434 classes.)

## gate_verdict_tail

Last 15 lines of `cargo xtask platform wave gate --slice T-941.6`:

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

  gate verdict PASS @ ec424991348b recorded: .ai/artifacts/verdicts/T-941.6.json
SLICE GATE: PASS
```

## mod_compile_verdict

`OK: compiled clean` — 5755 files, 11434 classes, 0 TBD warnings. `cargo xtask mod world-boot` → `WORLD BOOT: PASS` (roll-call SpawnManager=ok Safestart=ok LoadoutEquip=ok Spectator=ok Lobby=ok PlayArea=ok Markers=ok Radio=ok Objectives=ok). Did not restart :3000/:8080.

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| `docs/platform/PLAYTEST_RUNBOOK.md:807-809` and `:1286-1289` | Still describes T-327 as unfixable from script (`super` first). Sibling docs; this slice does not own them. |
| `.ai/tickets/T-327.toml` | Still `deferred` / `executor: workbench`. Command center owns ticket status. |

`#tbd missions` and other admin commands still go through `super.OnNewMessage` (public). Ticket forbids filtering beyond the link command.

## deviations

T-327 / the IdentityLink header claimed OnNewMessage cannot suppress because it is receive-side. T-941.6 locked consume-before-broadcast in script anyway. Implementation: skip `super.OnNewMessage` on **every** peer when `IsLinkCommand`, and run `TryHandleChat` only on authority. That matches the ticket (consume before broadcast, private `SendPrivateMessage`) without a new UI field. Two-client proof is the human checklist; Enfusion compile cannot see chat panels.

Ticket/plan perturbation wording was "misspell `SendPrivateMessage`". Brief wording was "break the consume-before-broadcast guard → compile RED". The misspelled guard name is the brief's perturbation; it is also a missing-method compile RED.

## commits

- `ec424991348bf963e0b35a07c96f01fe39a0e1c1` — T-941.6: consume account-link before public chat broadcast.

## manual_checklist

- Two clients on a dedicated server: player A types `#tbd link ABC123`. Player B's public chat shows nothing. Player A gets only private `TBD:` replies (`checking…` / success / failure), never a public echo of the code.
- Player A types `#tbd link status`: only A sees the identity/website lines; B sees nothing.
- Player A types a bare `ABC123` (no `#tbd link` prefix): ordinary public chat; B can see it.
- Non-admin `#tbd missions` is unchanged (still admin-gated after super).
- Failed link: A is told to generate a new code; B still never saw the typed line.

## twins_confirmed

- `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_AdminCommands.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_AdminCommands.c` — on disk
- `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_IdentityLink.c` — on disk
- `apps/mod/tbd-export/Scripts/Game/TBD/Backend/TBD_IdentityLink.c` — on disk
