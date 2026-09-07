# REPORT T-941.2 — Lobby: deploy holders on BRIEFING, reopen path

## pwd_branch

```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-941.2
slice/T-941.2
```

Command: `pwd && git branch --show-current` (first action). EnfusionMCP already 19 files; no copy.

## defect_verified_on_main

Worktree tracked `slice/T-941.2` at dispatch (`87bd9d9b0`). Defect still present before the edit. No script unit-test lane; evidence is the live sources.

| claim | path:line | command |
|---|---|---|
| Race note: close, don't lock | `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Lobby/TBD_LobbyScreen.c:211-212` (export twin same) | `sed -n '183,216p' …/TBD_LobbyScreen.c` |
| DEPLOY gating closes via ShouldStandDown | same file `:492-507` / `OnRosterChanged` `Call(DeferredClose)` | `sed -n '183,208p'` and `sed -n '488,511p'` |
| Auto-deploy fires ~250 ms into LOBBY | `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:2346-2354` + `ScheduleDeployAllConnectedPlayers` `CallLater(..., 250, false)` | `sed -n '2330,2400p'` |

Quoted (framework, pre-edit):

```
		// `TBD_SpawnManager`'s LOBBY auto-deploy wave (`m_bAutoDeploy`, still 1 — it deploys every
		// connected player ~250 ms into LOBBY), the JIP `DeployJoiner` path, and `AdminRespawn`.
…
		if (TBD_LobbyClient.ShouldStandDown())
			GetGame().GetCallqueue().Call(DeferredClose);
…

	//! Guarded: the stage watcher may have closed this screen already (LOBBY -> BRIEFING lands in
	//! the same breath as a deploy), and closing a dead menu through the stack is not something to
	//! find out about at an event.
```

```
		if (m_bAutoDeploy)
		{
			PrintFormat("[TBD][Spawn] LOBBY: auto-deploy wave ON — seating everyone in 250 ms. The slot picker will open and then close itself; set m_bAutoDeploy 0 on TBD_GameMode.et to use the picker.",
				level: LogLevel.WARNING);
			ScheduleDeployAllConnectedPlayers();
			return;
		}
…
		GetGame().GetCallqueue().CallLater(DeployAllConnectedPlayers, 250, false);
```

`TBD_GameMode.et` already sets `m_bAutoDeploy 0`, so production used the picker DEPLOY click during LOBBY. The code default and the JIP `m_bAutoDeploy` branch still seated bodies in LOBBY when the flag was on. Either path put a body on screen before BRIEFING.

## changes

| path | line | why |
|---|---|---|
| `TBD_SpawnManager.c` (framework + export) | 111-114, 301, 368 | Dated decision: wave does not fire in LOBBY. `map<int,bool> m_mDeployedHolders` tracks one deploy per player id. |
| same | 798-826 | `ClaimSlot` after LOBBY: first claim deploys; a different seat despawns + redeploys. |
| same | 1206 | Loadout settle re-kicks the holder wave on **BRIEFING**, not LOBBY. |
| same | 2262-2280 | JIP: LOBBY → picker (bodiless); post-lobby claimed holder deploys once. |
| same | 2380-2474 | `OnStageChanged`: LOBBY logs and returns; LOBBY→BRIEFING calls `ScheduleDeployClaimedHolders` (250 ms). Helpers: `DeployClaimedHolders`, `MarkHolderDeployed`, `RedeployHolderToClaimedSlot`. `m_bAutoDeploy` only seats leftover unclaimed players at that same BRIEFING beat. |
| same | 2601-2608 | `DeployPlayerInternal` returns `FAILED` during LOBBY (not RETRY, not DENIED). |
| same | disconnect / kill / watchdog | `m_mDeployedHolders.Remove(playerId)` stays in lockstep with `m_mDeployRequested`. |
| `TBD_LobbyScreen.c` (framework + export) | 187-208 | Dated decision at the old :211-212 site: do not close on a body; the guard is DEPLOY. |
| same | 482-538, 650-658 | DEPLOY disabled with a reason while pending (`Deploy in progress.`), in-world, or during LOBBY (`You deploy when briefing starts.`). |
| same | 676-698, 769-832 | Back shown after LOBBY so the overlay can dismiss. `PauseMenuUI` relabels hidden `LeaveFaction` to **Change slot** and opens the lobby (`OpenFromPause`). |

No vehicle spawn changes (T-941.8). No files outside owns.

## perturbation

Declared `m_mDeployedHolders` as `map<string, bool>` in the framework tree only (call sites still pass `int playerId`).

**red_output VERBATIM** (`cargo xtask mod compile` after the mismatch):

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
     Running `/home/Samuel/.cache/tbd-target/debug/xtask mod compile`
==> compiling tbd-framework + tbd-export (native headless server, no Workbench)

FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:2278: Cannot convert 'int' to 'string' for argument '0' in method 'Contains'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:2364: Cannot convert 'int' to 'string' for argument '0' in method 'Contains'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:2427: Cannot convert 'int' to 'string' for argument '0' in method 'Contains'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:2450: Cannot convert 'int' to 'string' for argument '0' in method 'Set'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:2465: Cannot convert 'int' to 'string' for argument '0' in method 'Remove'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:2466: Cannot convert 'int' to 'string' for argument '0' in method 'Remove'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:2505: Cannot convert 'int' to 'string' for argument '0' in method 'Contains'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:3087: Cannot convert 'int' to 'string' for argument '0' in method 'Remove'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:3254: Cannot convert 'int' to 'string' for argument '0' in method 'Remove'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:3396: Cannot convert 'int' to 'string' for argument '0' in method 'Remove'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:3631: Cannot convert 'int' to 'string' for argument '0' in method 'Remove'
Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c:368: Types 'map<int,bool>' and 'map<string,bool>' are unrelated
------------------------------------------------------------
12 error(s) in TBD sources, 2 cascaded into vanilla.
Cascade (fix the TBD errors first; these usually vanish):
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:452: Can't find class Tuple2
  Scripts/Game/UI/Menu/ContentBrowser/Common/SCR_ScenarioUICommon.c:517: Can't find class Tuple2
```

**restored_green:** restored `map<int, bool>`, `touch` the framework file, `cargo xtask mod compile`:

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11435x classes
    Compiling Game scripts took: 841.094000 ms
    0 warning(s) in TBD sources
```

Final post-cleanup compile (duplicate `Remove` / indent): same `OK: compiled clean`, 864 ms, 0 TBD warnings.

## gate_verdict_tail

Last lines of `cargo xtask platform wave gate --slice T-941.2` (waited ~30s on T-654's gate lock, then):

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

  gate verdict PASS @ 87bd9d9b082b recorded: .ai/artifacts/verdicts/T-941.2.json
SLICE GATE: PASS
```

## mod_compile_verdict

`OK: compiled clean` — 5755 files, 11435 classes, 0 TBD warnings. `cargo xtask mod world-boot` → `WORLD BOOT: PASS` (roll-call includes `SpawnManager=ok Lobby=ok`). Did not restart :3000/:8080.

## files_outside_owns

[]

## found_not_fixed

| path:line | repro |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Lobby/TBD_LobbyController.c:193, 232, 880, 983-1006` (export twin) | Comments still describe the LOBBY 250 ms auto-deploy wave and `ShouldStandDown` closing the picker. Behaviour is now in SpawnManager/LobbyScreen; this file is T-139 / not in owns. |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Lobby/TBD_LobbyData.c:134` | Same stale `m_bAutoDeploy, still 1` comment. Not in owns. |
| `TBD_LobbyScreen.c` known blocker (preset / `resourceDatabase.rdb`) | Unchanged: `TBD_UILobby` still needs a Workbench pass before the screen can open in a live client. Headless compile/boot do not register it. |

## deviations

- Pause-menu affordance reuses vanilla **LeaveFaction** (hidden on TBD because `m_bAllowFactionChange 0`) rather than a new `.layout`. No new files: owns forbids them, and SIZE allowlists are not extended.
- `m_bAutoDeploy` is no longer the production briefing path. Claimed holders always deploy on LOBBY→BRIEFING. The flag, when ON, only auto-assigns leftover unclaimed players at that same beat (PIE). Prefab still sets it 0.

## commits

(filled after `git commit` on explicit paths)

## manual_checklist

In-game (gate cannot stand in):

1. Two players claim different slots in LOBBY: **no bodies**.
2. Advance to BRIEFING: **one body each**, once (`path=briefing-holder`).
3. Double-click DEPLOY while a deploy is pending: button stays disabled with **Deploy in progress.**; still one body.
4. Pause menu → **Change slot** → pick a different open seat: old body left, new seat possessed (`path=slot-change`).

## twins_confirmed

| framework | export |
|---|---|
| `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` | `apps/mod/tbd-export/Scripts/Game/TBD/Gamemode/TBD_SpawnManager.c` |
| `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Lobby/TBD_LobbyScreen.c` | `apps/mod/tbd-export/Scripts/Game/TBD/UI/Lobby/TBD_LobbyScreen.c` |

Export kept ASCII comments/dashes. Both trees compiled in the same `mod compile` pass.
