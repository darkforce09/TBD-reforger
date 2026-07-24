# Slot-Body Materialization — session handoff (updated 2026-07-25, Fable 5)

**Tip:** `a18eeb2e` (was `f4b25440`). **Read with:** `CLAUDE.md` · verify log
`.ai/artifacts/spawn_determinism_verify_log.md` (addendum at the end is authoritative for
what changed and why) · this file.

**Operator decisions (locked, do not re-litigate):** bodies created at MISSION LOAD from
compiled JSON (not baked into the world); PS-style lobby experience; re-equip every spawn;
dynamic per-side slot counts (128 = player cap only).

## Current state — the deploy path (CHANGED 2026-07-25)

Mission load materializes one numbered body per compiled slot (kit prefab at the JSON
transform, AI disabled once, Arsenal loadout applied) before the lobby. **Deploy is now
vanilla's POSSESS spawn request** — `SCR_PossessSpawnData.FromEntity(body)` +
`SCR_PossessSpawnRequestComponent.RequestRespawn(data)` — NOT raw `SetInitialMainEntity`.

**Why this matters more than anything else in this file:** `SetInitialMainEntity`
possesses the body and even gives it a live `PlayerCamera`, but it is *not a spawn* — the
vanilla finalize never runs, so the client is never told it spawned and sits on the
loading screen forever. The possess request performs the same takeover *through* the
pipeline: it creates no entity (census stays `characters == bodies`, double-spawn stays
fixed) and runs the finalize the client waits on. `SetInitialMainEntity` remains only as a
logged fallback. If a future change ever reverts to it, the loading screen comes back.

Vanilla's player registration is stood down on framework worlds
(`TBD_SCR_RespawnSystemComponent` + the `TBD_SCR_MenuSpawnLogic` hooks), which is what
killed the faction churn. Everything is guarded on `TBD_FrameworkManager.IsFrameworkWorld()`
so the mod stays inert on plain vanilla worlds.

## Closed this session

1. `set.Remove()` by index — compile-confirmed fixed (0 errors, VM exceptions gone).
2. Faction cycling — root-caused and fixed (13 switch lines → 0). Handoff Lead 1 was
   **falsified**: the churn starts ~9 ms after materialization, before the first VM
   exception and before the bind. Lead 2 was right.
3. `EngineFactionKey` — `indfor`→`FIA`, `civ`→`CIV`, + body-faction fallback +
   `UpdatePlayerFaction_S`.
4. (Not in the old handoff) Client pinned on the loading screen — fixed by the possess
   route above.

## OPEN — next session starts here

- **Determinism gate: 2/5, INCOMPLETE.** Both completed runs byte-identical
  (`6abcbdd84e02`, 21 lines), census 1/1/1, zero errors, zero churn. Stopped at operator
  request. Close it with `bash scripts/mod/tbd-spawn-determinism.sh 5`.
- **Death → redeploy never verified live.** Code is in (`RedeployAfterDeath`,
  `m_iRedeployDelayMs` default 5000, gated on `m_bAutoDeploy`). Needs a grenade-at-feet
  check: fresh dressed body at the slot after ~5 s, corpse stays.
- **Dedicated / JIP pass** — playerId reuse, real audit, reconnect reclaim. PIE cannot
  exercise these. Also the fallback route's client-side spawn invoke is server-side only
  (PS RPCs theirs to the client); moot on the possess path, real on the fallback.
- **T-068.13 lobby / slot picker** — `ClaimSlot` is the ready backend; `m_bAutoDeploy`
  turns off the PIE auto-wave when the picker lands.

## Landmines (measured — do not relearn)

- **`api_search` returns signatures, not bodies.** There is no vanilla `.c` source on disk
  (it lives in `addons/*/data*.pak`, FORM format, not zip). This is the single biggest
  diagnostic tax in this codebase: today's loading-screen hunt cost 4 Workbench restarts
  and 3 operator round-trips that one `grep` of the vanilla spawn pipeline would have
  answered. **Getting vanilla sources greppable is the highest-leverage tooling task
  available.**
- **Workbench is a hard serial resource** — one instance, one port (5775), one machine.
  Mod iteration cannot be parallelized; batch several hypotheses per boot behind
  attributes instead of one lever per restart. Restart is 90–120 s.
- `wb_reload` never recompiles — restart Workbench for every compile
  (`pkill -f "WorkbenchSteamD[i]ag"`, Steam relaunches; game-dead boots happen, probe
  `wb_open_resource` and cycle; helper: the restart/smoke scripts pattern in
  `scripts/mod/tbd-spawn-determinism.sh`).
- Loader statics survive re-play inside one WB process → restart per gate run.
- "Can't initialize the game" is usually a Game-module **compile error** — read the boot
  log's `Compiling Game scripts` section first.
- Enforce `set`/`array.Remove` is **by index** — use `map<K,bool>` / `RemoveItem`.
- VM exceptions carry no `[TBD]` tag (harness greps them separately).
- `wb_script_editor` is 0-based.
- Backend for tests: `make db-up && make api`; mission
  `6d291619-8182-4164-866d-4e165a5516af` v0.1.3 (`$profile TBD_BackendConfig.json`).
  With the backend down, roster/mission-list 404 and the run settles `roster
  settled=failed` — deterministic, which is why the gate still passes without it.

## Reference frameworks (design mirror only)

**CRF** — `apps/mod/crf_framework` (gitignored, local). Load-bearing today:
- `Scripts/Game/Systems/VanillaOverrides/.../CRF_SCR_PossessSpawnHandlerComponent.c` — the
  file that revealed the possess pipeline; also shows the faction finalize
  (`SetAffiliatedFaction` + `UpdatePlayerFaction_S` + `OnPlayerFactionSet_S`).
- `.../Managers/Gamemode/CRF_GamemodeManager.c:54-114` (`InitilizePlayer`) — pre-spawned
  character → DisableAI → faction → assign → notify, the shape ours mirrors.
- `.../Helpers/CRF_PlayerHelper.c:30-43` — why they use `SetInitialMainEntity` over
  `RequestSpawn` (the `AssignEntity_S` guard cancels finalization on re-init).
- `.../PlayerController/CRF_PlayerControllerManager.c:61-136` — the **client-side** half:
  `CloseAllMenus`, camera cleanup, and a client-side `GetOnPlayerSpawned().Invoke`. We
  have no equivalent; it is the model if the fallback path ever needs one.

**PlayableSelector** — clone at `scratchpad/PlayableSelector` (re-clone:
github.com/JiraF4/PlayableSelector). **NO LICENSE — mirror design, never copy code.**
Vanilla component-instance GUIDs appearing in their `.et` files are vanilla facts and are
fine to reuse. Their controller-prefab component disables are what
`Prefabs/Systems/TBD_PlayerController.et` mirrors — but note we deliberately keep
`SCR_RespawnComponent` and `SCR_PossessSpawnRequestComponent` ENABLED (PS disables both;
we need the possess route) and disable only the two body-creating request components.
