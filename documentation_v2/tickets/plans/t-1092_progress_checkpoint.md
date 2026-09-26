**Status:** live

# T-1092 — Progress checkpoint

The resume file for [T-1092](/documentation_v2/tickets/specs/t1092_mod_script_modularisation.md).
To resume: read this file, then continue the roster of the
[plan](/documentation_v2/tickets/plans/t-1092_plan.md) from the first row that is not `done`.
Statuses: `pending`, `running`, `done — awaiting commit`, `done`, `blocked`.

## Frozen class names

Referenced from `.et`, `.layout`, `.conf` or `.ent` files in `apps/mod/tbd-framework`; no slice
renames them. Moving their files is fine.

`TBD_AdminScreen` `TBD_BriefingScreen` `TBD_ChipComponent` `TBD_DebriefScreen`
`TBD_DropdownComponent` `TBD_EndScreen` `TBD_FrameworkManager` `TBD_KeyValueRowComponent`
`TBD_ListBox` `TBD_ListBoxRow` `TBD_LoadoutEquipComponent` `TBD_LobbyComponent`
`TBD_LobbyFactionRowComponent` `TBD_LobbyScreen` `TBD_LobbySlotRowComponent`
`TBD_LobbySquadCardComponent` `TBD_MarkerComponent` `TBD_MissionCardComponent`
`TBD_MissionSelectorScreen` `TBD_NavItemComponent` `TBD_NumberedCardComponent` `TBD_ObjectiveHud`
`TBD_ObjectivesComponent` `TBD_PanelComponent` `TBD_PlayAreaComponent` `TBD_PreSlotComponent`
`TBD_PrimaryNavItemComponent` `TBD_RadioComponent` `TBD_SafestartManager`
`TBD_SearchBoxComponent` `TBD_SectionComponent` `TBD_SessionBottomBar` `TBD_SessionTopBar`
`TBD_ShellScreen` `TBD_SpawnManager` `TBD_SpectatorComponent` `TBD_SpectatorScreen`
`TBD_TabStripComponent` `TBD_TerrainRowComponent` `TBD_TopicNavItemComponent` `TBD_UIButton`

Also frozen: every `ChimeraMenuPreset` member name (configs reference them), every `EMCP_` class
name in tbd-emcp (the MCP broker calls them by name).

## Environment at program start

Another session holds uncommitted work in `apps/mod/tbd-export`, `tools_v2/xtask`
(`commands/mod_ops`, `commands/deploy`, `commands/generate`, `Cargo.toml`), `tools_v2/developer-tools`,
`apps/website`, `contracts_v2`, `assets_v2/equipment`, `CLAUDE.md`, `.gitignore` and `Cargo.lock`.
Commits of this program stage by pathspec only.

## Roster

| Id | Status | Commit | Result |
|---|---|---|---|
| P0 | done | d5cb82b97 | Tickets T-1092.1–.6, spec, plan, checkpoint |
| P1-1 | done | 87d831968 | `.rs`+`.c` walk, `MOD_SCRIPT_ROOTS` + const assert on apps/mod pins; 14 tests (filter `file_length_tests`, not `node_free`). Slip: `hcargo fmt -p xtask` reformatted the other session's uncommitted `commands/mod_ops/mod.rs` and `equipment_vehicle_export/publication.rs` (format only, left in place). |
| P1-2 | done | 244e62813 | Gate built (13 rule modules, shared `enfusion_script_lexer.rs`), 17 tests; roots unpinned. |
| P1-3 | done | 57339f19e | 177 files, -3103 separator/banner lines, 67 ticket tags, 117 field docs to trailing; compile 0. |
| P2-1 | done | (this commit) | 19 helpers, 82 `TBD_Authority` replacements, presets in `UI/Core/ChimeraMenuPreset.c` (file named for its type); compile 0. |
| P2-2 | done | (this commit) | `Heartbeat/TBD_RuntimeHeartbeat.c` (1000 ms, WinCondition on even beats) replaces 8 drivers; `TBD_DebriefScoreboard.Fill`; world-boot 0, order matches baseline; ECM rule sees `TBD_Authority`. |
| P3-1 | done | 49f0180ad | Spawning: manager 287 lines over Manager/Slots/Identity/Deploy/Lives/Vehicles/Dynamic/Deployment/VanillaBridge/Client; deployment-authorization modded class folded; 3 dead members removed; world-boot PASS. `TBD_DeployExecutor.DeployPlayerInternal` public (bypasses ONE LIFE; banner restricts callers). |
| P3-2 | done | b84cf0220 | Loaders/Mission + Loaders/Validation + Data/Document; validator gear count now `CountGear` (counts attachments); `TBD_VariantConfigStruct` -> `TBD_VariantConfigWire`. |
| P3-3 | done | 5da30bce6 | Zones: Triggers/ (12), Registry/, Volumes/, PlayArea/; adds `TBD_ZoneRegistry.FindById`, `TBD_TriggerRuntime.FindById`/`HasFired(id, out unknownId)`; dead-body cast now `ChimeraCharacter`. |
| P3-4 | done | 6270ea673 | Objectives: Model/, Registry/, Runtime/, Tasks/ (19 files). |
| P3-5 | done | 0a4831cb0 | API: Http/, RuntimeSession/, Identity/, Results/, FleetCommands one type per file; `TBD_BackendConfigStruct` -> `TBD_BackendConfigFile`; new `TBD_RuntimeSessionLifecycle`. |
| P3-6 | done | 5c53cf2bb | Data/Vehicles (5 files); `TBD_MissionParamSelectionStruct` -> `...Wire`; GadgetFlags 471 lines (over 400 target). |
| P3-7 | done | f8f8701c4 | Loadouts/Application (5 phases), Preview/, AI/Waypoints, AI/GroupState; export DTOs to `TBD_LoadoutExportStruct`. |
| P3-8 | done | 03d9d83be | Audio (5), Markers (+Client/), Radio comment pass; modded PCs named `SCR_PlayerController.c` per folder. |
| P3-9 | done | 0bf022275 | FrameworkManager 398 over Flow/ + Stage/; Safestart/, WinConditions/; `TBD_GameStage.c` -> `TBD_EGameStage.c`; Safestart OnDelete now cancels its timers. |
| P3-10 | done | c7afddd02 | Briefing Service/, Catalog/, UI/Pages, UI/Navigation, map launcher; wire bytes unchanged; self-check armed from Serialise now runs. |
| P3-11 | running | | |
| P3-12 | running | | |
| P3-13 | running | | |
| P3-14 | running | | |
| P3-C | pending | | |
| P4-1 | pending | | |
| OP-1 | pending | | operator playtest |
| P5-1 | pending | | |
| P6-1 … P6-6 | blocked | | waits for the other session's tbd-export commit |
| P6-C | pending | | |

## Amendments

Additions to a launch prompt beyond concrete values, by slice id.

| Slice | Addition |
|---|---|
| P3-11…P3-14 | Brief file gains a "Bugs" section (operator 2026-09-26: bugs are noted with file:line, never fixed) and the wave B helpers (`EndRound`, `CountSurvivors`, `TBD_WarnOnce`, `TBD_AnnounceOnce`, `TBD_Rounding`). Slice notes carry each folder's stale-comment leftovers; P3-13 switches `TBD_MissionDeploymentRelay` off the `JsonEscape` forwarder. |
| P3-6…P3-10 | Brief file gains wave A lessons: name split files after their primary type (ECM-9; companion structs live with their owner, enums get their own file); run `readme-coverage`/`link-check` with `--with-untracked`. |
| P3-1…P3-5 | Delivered as one scratch brief file (B0 + CARD + SPLIT RULES + writer steps, verbatim) plus a "Parallel wave rules" block: judge compile by own-file errors only; git mv is fine, no other git add/reset. Slice notes add: heartbeat owns Tick calls (keep static Tick signatures); P3-3 adds `TBD_ZoneRegistry.FindById` and `TBD_TriggerRuntime.HasFired` for other slices; P3-5 must not touch `TBD_DebriefScoreboard.c`. |
| P2-2 | Also update `tools_v2/xtask/src/verifications/mod_scripts/enfusion_comments/network_authority_rule.rs:25` so `TBD_Authority.IsClient()`/`IsServer()` calls count as context-dependent (the P2-1 replacement hid 82 sites), with a test. |
| P1-3 onward | B0 gains: "Never run `hcargo fmt -p <package>` (it reformats the other session's files); check with `hcargo fmt -p xtask -- --check` and format only your own files." |
| P1-2 | Concurrency note: P1-1 edits `language_bans/` and `node_free_tests.rs` at the same time; the other session's uncommitted `tools_v2/xtask` edits are reported, not fixed, if they break the build. |

## Writer brief file

Wave launches pass one file built from the plan's B0, CARD, SPLIT RULES and writer steps blocks (verbatim; B0 adds "Never run `hcargo fmt -p <package>`") plus these two sections, then a short per-slice parameter prompt. Rebuild it in the scratchpad on resume.

```text
### Parallel wave rules
- Other slices run at the same time on other folders. Judge `mod compile` only by error lines that
  name your files; ignore errors elsewhere (the orchestrator compiles after the wave).
- `git mv` stages renames in the shared index; that is fine. Never `git add` anything else, never
  `git reset`, and never touch another slice's folders.
- Record forwarders you leave in your report (the orchestrator adds them to the checkpoint).

### Lessons from wave A
- ECM-9: every file is named after its primary type. Map names like `...Structs` or `...Types`
  are intents, not file names: put a companion struct with its owner, give an enum its own file
  (`TBD_EXxx.c`), and name the file after the class it declares.
- Run `hcargo xtask verify readme-coverage --with-untracked --path <f>` and
  `hcargo xtask verify link-check --with-untracked --path <f>`; without the flag, new untracked
  files fail.
- Shared helpers now also include `TBD_ZoneRegistry.FindById`, `TBD_TriggerRuntime.HasFired(id,
  out bool unknownId)` and `TBD_TriggerRuntime.FindById`.
- A stale comment in your folders that names a moved file or member is yours to fix.
```

## Pause point

Session paused 2026-09-26 after wave B (operator: session budget); resumed the same day with wave C. Next: launch wave C (P3-11 Lobby, P3-12 Spectator+Players+PostGame, P3-13 Admin+MissionSelector, P3-14 UI+Core) with the plan's slice parameters, then P3-C. Tree at pause: `mod compile` 0, `mod world-boot` PASS, 2074 comment findings in 78 of 324 framework scripts.

## Comment gate baseline

Filled by P1-2 and P1-3.

| Tree | When | ECM-1 | ECM-2 | ECM-3 | ECM-4 | ECM-5 | ECM-6 | ECM-7 | ECM-8 | ECM-9 | Total |
|---|---|---|---|---|---|---|---|---|---|---|---|
| tbd-framework (172 files) | after P1-2 | 2890 | 187 | 1179 | 2429 | 49 | 80 | 19 | 4105 | 46 | 10984 |
| tbd-framework | after P1-3 | 126 | 187 | 1179 | 2312 | 49 | 80 | 19 | 1425 | 43 | 5420 |
| tbd-framework | after wave A | 117 | 138 | 935 | 1538 | 36 | 37 | 19 | 773 | 31 | 3624 |
| tbd-framework (324 files) | after wave B | 100 | 78 | 586 | 929 | 20 | 7 | 19 | 320 | 15 | 2074 |
| tbd-emcp (19 files) | after P1-3 | 0 | | | 158 | | | | 0 | | 458 |

## Shared helper index

Filled by P2-1: path | class.method signature | replaces (file:method) | parameter notes.

Paths are under `apps/mod/tbd-framework/Scripts/Game/TBD/`. All helpers are static.

| Path | Signature | Replaces | Parameter notes |
|---|---|---|---|
| `Core/Characters/TBD_CharacterUtil.c` | `TBD_CharacterUtil.IsDead(IEntity body, bool missingCountsAsDead = false)` | `TBD_ObjectivesComponent`, `TBD_PlayAreaComponent`, `TBD_TriggerRuntime`: `IsBodyDead`; `TBD_SpawnManager.IsBodyDead` | SpawnManager passes `true` (null, non-character, no controller read dead). Casts `ChimeraCharacter` (the three presence copies cast `SCR_ChimeraCharacter`; every Reforger character is one). |
| `Core/TBD_PlayerChat.c` | `TBD_PlayerChat.Broadcast(string tag, string text)` returns reached count | `TBD_FrameworkManager.Broadcast` (tag `Flow`), `TBD_SafestartManager.Broadcast` (tag `Safestart`), `TBD_FleetPlayerActions.Broadcast` send | Client returns 0 without sending. Empty `tag` writes no `[TBD][<tag>] broadcast:` line (the fleet path, which logs its own `Kv`). |
| `Core/Players/TBD_PlayerFaction.c` | `TBD_PlayerFaction.Of(TBD_SpawnManager spawn, int playerId)` | `TBD_ObjectivesComponent.ResolveFaction`, `TBD_PlayAreaComponent.ResolveFaction` | none |
| `Systems/Mission/Data/TBD_MissionFactionNames.c` | `TBD_MissionFactionNames.DisplayName(TBD_MissionDocumentStruct doc, string factionKey)` | `TBD_BriefingService.ResolveFactionName`, `TBD_LobbyService.ResolveFactionName` | Returns raw text; call sites write `TBD_WireCodec.Sanitise(TBD_MissionFactionNames.DisplayName(doc, key))` for identical bytes. Null `doc` guarded (Briefing did not guard). |
| `Core/Wire/TBD_WireCodec.c` | `FIELD_SEP`, `LINE_SEP`, `FIELD_MARK`; `Field(value, bool sanitise = true)`, `Unmark`, `IsSet`, `Flag`, `Sanitise`, `Join(lines, int maxLines, string channel, string clipWarningFormat)`, `Record1..Record4(kind, ..., bool sanitise = true)`, `Record(int fields, kind, a, b, c, d, e)` | `TBD_BriefingService`, `TBD_LobbyService`, `TBD_AdminSnapshotService`: `Field`, `Unmark`, `IsSet`, `Flag`, `Sanitise`, `Join`, `Record*`, the three constants | Admin passes `sanitise = false` to `Field`/`Record1..4`. `Join` takes each file's `MAX_PAYLOAD_LINES` (400 briefing and admin, 600 lobby), its channel and its own clip message (`%1` = limit). `Record` is Lobby's. `TBD_AdminSnapshotService.RecordPlayer` stays (admin-specific). `TBD_DebriefScreen` also declares `LINE_SEP`/`FIELD_SEP`. |
| `API/Http/TBD_BackendText.c` | `JsonEscape(value)`, `DescribeBackend(string noneText = "none")`, `UtcNowIso8601()`, `Pad2(int)` | `TBD_ResultsReporter`: all four; `TBD_IdentityLink`: `JsonEscape`, `DescribeBackend`; `TBD_RuntimeStatusReadings.Pad2`; `TBD_GameRuntimeHttp.JsonEscape` | ResultsReporter passes `"(none)"`. `TBD_GameRuntimeHttp.DescribeBackend` is a different semantic (machine credential, uses its protected `IsCredentialUsable`) and stays there. |
| `Core/Time/TBD_ClockText.c` | `FormatClock(int)`, `IsCountdownChatMilestone(int)`, `IsCountdownPopupMilestone(int)`, `IsRoundClockMilestone(int)` | `TBD_SafestartManager.FormatClock`, `.IsChatMilestone`, `.IsPopupMilestone`; `TBD_FrameworkManager.IsRoundClockMilestone` | none |
| `Core/TBD_Authority.c` | `TBD_Authority.IsClient()`, `TBD_Authority.IsServer()` | every `RplSession.Mode() == RplMode.Client` (79 sites, replaced here) and `!= RplMode.Client` (3 sites: AdminCommands, SpectatorComponent, SpawnManagerDeploymentAuthorization) | `IsServer` = not client (dedicated, listen, single-player). `RplMode.Dedicated`/`None` checks left as they are. |
| `Systems/Mission/Ingestion/TBD_MissionJsonPass.c` | `JsonLoadContext LoadRoot(out TBD_EMissionJsonPassOutcome outcome, string renameKeyFrom = "", string renameKeyTo = "")`; enum `LOADED`, `NO_DOCUMENT`, `NOT_JSON` | the `GetRawJson` + `LoadFromString` head of `ReadWire`/`EnsureParsed`/`Parse` in AudioEmitter, TaskStateMachine, WeatherRuntime, DynamicSpawner, TriggerRuntime, PlacementScatter, WaypointRuntime, GroupState, RadioPlan, ObjectiveRules, ObjectiveRegistry, WinConditionEvaluator, MissionParams, VehicleState, EntityState, GadgetFlags | Caller still does `ctx.ReadValue("", doc)` into its own struct (typed read; a `Managed` root is not proven). AudioEmitter passes `"event", "cueEvent"`. Callers keep their own log lines per outcome. |
| `Systems/Mission/Data/TBD_MissionVariants.c` | `IsActive(string variantId, array<string> activeIds, bool selectionInForce = true, bool missingSetKeepsRow = true)`; `IsRowIncluded(variantId, declared, active, collection, rowName)` | `TBD_ObjectiveRegistry.VariantActive`, `TBD_TriggerRuntime.IsVariantActive`, `TBD_MissionLoader.IsVariantRowIncluded` | ObjectiveRegistry: `IsActive(id, TBD_MissionLoader.GetActiveVariantIds())`. TriggerRuntime: `IsActive(id, s_aSelectedVariants, s_bVariantSelectionInForce, false)`. Loader: `IsRowIncluded` (same WARNING text). |
| `Systems/Loadouts/TBD_LoadoutInventoryUtil.c` | `PrefabOf`, `CountGear(TBD_SlotGearStruct)`, `AreasForLabel`, `IsRootedOn`, `WeaponStorageOf`, `WeaponStorageHas` | `TBD_LoadoutEquipHelper`: all six; `TBD_LoadoutPreviewDresser.PrefabOf`; `TBD_GadgetFlags.PrefabOf` | none |
| `Systems/AI/TBD_AIGroupFactory.c` | `SCR_AIGroup SpawnGroup(ResourceName prefab, vector origin, out TBD_EAIGroupSpawnFailure failure)`; `AdoptMemberFaction(notnull SCR_AIGroup, IEntity member)` | `TBD_DynamicSpawner.SpawnGroup`, `TBD_WaypointRuntime.SpawnGroup` | Callers log per failure (`PREFAB_UNLOADABLE`, `NOT_A_GROUP`) on their channel; DynamicSpawner keeps `ApplyFaction`, WaypointRuntime calls `AdoptMemberFaction`. A non-group entity is always deleted (WaypointRuntime leaked it; Group_Base is always a group). |
| `Systems/AI/TBD_AIWireEnums.c` | `SPEED_*`, `BEHAVIOUR_*`; `SpeedFromSpeedMode(speedMode, out speed)`, `SpeedCeilingFromBehaviour(behaviour, out speed)`, `SpeedFromWire(speedMode, behaviour, out speed)` | `TBD_GroupState.SpeedFromWire` and its `SPEED_*`/`BH_*`; `TBD_WaypointRuntime.SpeedFromWire` and its `SPEED_*`/`BH_*` | The two semantics: explicit `speedMode` speed, and `behaviour` speed ceiling; `SpeedFromWire` = speedMode wins. `BH_` constants are renamed `BEHAVIOUR_`. |
| `Core/World/TBD_EntityQuery.c` | `FirstVehicleNear(x, z, halfWidthM, halfHeightM)`, `CollectPrefabInBox(prefab, mins, maxs, outHits)`, `CollectPrefabInZone(prefab, zone, halfHeightM, bool checkHeightBand, outHits)` | `TBD_VehicleState`/`TBD_SpawnManager` `FindBody`+`OnQuery`; `TBD_WaypointRuntime` `OnVehicleQuery`; `TBD_MissionVehicleStruct` `OnCensusEntity`; `TBD_ObjectiveRegistry.CountLiveTargets`+`OnQueryEntity`; `TBD_TriggerRuntime` delete +`OnDeleteQueryEntity` | ObjectiveRegistry: `checkHeightBand = true` (ContainsOrigin), then counts alive over the hits itself. TriggerRuntime: `false` (`TBD_Zone.Contains`). Census: `CollectPrefabInBox`, use the count. |
| `Systems/Mission/Loaders/TBD_MissionLoader.c` | `TBD_MissionLoader.GetMissionId()` | `CurrentMissionId` in BriefingController, GadgetFlags, AudioEmitter, TaskStateMachine, WeatherRuntime, PlacementScatter, DynamicSpawner, GroupState, WaypointRuntime, TriggerRuntime | Reads `GetMission()` (not gated on `IsValid`), same as every copy. |
| `Core/Factions/TBD_DeclaredFactions.c` | `TBD_DeclaredFactions.Exists(string key)` | `TBD_ZoneVolume.FactionExists`, `TBD_ObjectiveRegistry.FactionExists` | none |
| `Core/Math/TBD_Rounding.c` | `TBD_Rounding.RoundToInt(float)` | `TBD_TaskStateMachine.RoundToInt`, `TBD_MarkerData.RoundToInt` | none |
| `Core/Logging/TBD_WarnOnce.c` | `TBD_WarnOnce.Warn(string channel, string key, string message, int maxKeysPerChannel = -1)` | `TBD_UIIcons.WarnOnce` (channel `ui`), `TBD_BriefingService.WarnOnce` (key `faction|field`, bound `MAX_WARN_STATES` 64), `TBD_LoadoutPreviewDresser.WarnOnce` (channel `lobby`, message prefixed `kit preview: `) | Writes `TBD_Log.Warn`, byte-identical to the `Print` copies. Keys are scoped per channel; the bound clears only that channel. |
| `Core/Logging/TBD_AnnounceOnce.c` | `Claim(key)`, `Rearm(key)`, `Event(channel, key, message)`, `Kv(channel, key, eventName, keyValues)` | `AnnounceOnce`/`AnnounceEmptyOnce` flags in TriggerRuntime, WinConditionEvaluator, TaskStateMachine, WeatherRuntime, AudioEmitter, DynamicSpawner, ObjectivesComponent, PlayAreaComponent (+ WaypointRuntime, GroupState `s_bAnnounced`) | Each `s_bAnnounced = false` reset becomes `Rearm(key)`. Per-instance component flags need a `Rearm` in the component's init. Multi-line announcers use `Claim` then log as today. |
| `UI/Core/ChimeraMenuPreset.c` | `modded enum ChimeraMenuPreset { TBD_UIShell, TBD_UIMissionSelector, TBD_UIBriefing, TBD_UILobby, TBD_UIAdmin, TBD_Spectator }` | the six `modded enum ChimeraMenuPreset` blocks (moved here) | Named `ChimeraMenuPreset.c`, not `TBD_MenuPresets.c`: ECM-9 requires the file stem to name its type. |

## Tick-order baseline

Recorded by P2-2 from `hcargo xtask mod world-boot --keep-logs` (`TBD_WORLDBOOT_SETTLE=8`) with temporary `Print`s after each driver's `super.OnGameStart()` and before each static `Tick`; the `Print`s were removed before the heartbeat change.

Driver hooks before the merge: each of the 8 `modded class SCR_BaseGameMode` blocks overrides only `OnGameStart`, calls `super.OnGameStart()` first, clears its runtime, returns on a client (Task instead arms its HUD tick there) or outside a framework world, sets its own armed flag, and arms a one-shot self-re-arming `CallLater` guarded by `GetGame().GetGameMode() != this`.

| Driver | Armed flag | Interval | Client side |
| --- | --- | --- | --- |
| Task | `m_bTBD_TaskTickArmed` | 1000 ms | `TBD_TaskHud.RequestLocal` every 1000 ms |
| WinCondition | `m_bTBD_WinConditionTickArmed` | 2000 ms, stops once `HasEnded()` | none |
| GroupState | `m_bTBD_GroupStateTickArmed` | 1000 ms | none |
| Waypoint | `m_bTBD_WaypointTickArmed` | 1000 ms | none |
| Audio | `m_bTBD_AudioTickArmed` | 1000 ms | none |
| Weather | `m_bTBD_WeatherTickArmed` | 1000 ms | none |
| DynamicSpawner | `m_bTBD_SpawnTickArmed` | 1000 ms | none |
| Trigger | `m_bTBD_TriggerTickArmed` | 1000 ms | none |

Order derived from the log:

- `OnGameStart` (clears): Task, WinCondition, GroupState, Waypoint, Audio, Weather, DynamicSpawner, Trigger.
- Odd beats: Task, GroupState, Waypoint, Audio, Weather, DynamicSpawner, Trigger.
- Even beats: WinCondition, then the odd-beat order.

Evidence (`console.log`):

```text
13:32:35.997 SCRIPT       : TBD_TICKORDER start Task
13:32:35.997 SCRIPT       : TBD_TICKORDER start WinCondition
13:32:35.997 SCRIPT       : TBD_TICKORDER start GroupState
13:32:35.997 SCRIPT       : TBD_TICKORDER start Waypoint
13:32:35.997 SCRIPT       : TBD_TICKORDER start Audio
13:32:35.997 SCRIPT       : TBD_TICKORDER start Weather
13:32:35.997 SCRIPT       : TBD_TICKORDER start DynamicSpawner
13:32:35.997 SCRIPT       : TBD_TICKORDER start Trigger
13:32:36.853 SCRIPT       : TBD_TICKORDER tick Task TBD_TaskStateMachine.Tick
13:32:36.853 SCRIPT       : TBD_TICKORDER tick GroupState TBD_GroupState.Tick
13:32:36.853 SCRIPT       : TBD_TICKORDER tick Waypoint TBD_WaypointRuntime.Tick
13:32:36.853 SCRIPT       : TBD_TICKORDER tick Audio TBD_AudioEmitter.Tick
13:32:36.853 SCRIPT       : TBD_TICKORDER tick Weather TBD_WeatherRuntime.Tick
13:32:36.853 SCRIPT       : TBD_TICKORDER tick DynamicSpawner TBD_DynamicSpawner.Tick
13:32:36.853 SCRIPT       : TBD_TICKORDER tick Trigger TBD_TriggerRuntime.Tick
13:32:37.853 SCRIPT       : TBD_TICKORDER tick WinCondition TBD_WinConditionEvaluator.Tick
13:32:37.853 SCRIPT       : TBD_TICKORDER tick Task TBD_TaskStateMachine.Tick
13:32:37.853 SCRIPT       : TBD_TICKORDER tick GroupState TBD_GroupState.Tick
```

The same two patterns repeat at 38.853 (odd) and 39.852 (even). The heartbeat's arm-time line must read `order=WinCondition/2,Task,GroupState,Waypoint,Audio,Weather,DynamicSpawner,Trigger` on the server.

## Forwarders

Left by P3 slices for P3-C to remove: forwarder | owner file | external callers.

- `TBD_GameRuntimeHttp.JsonEscape` -> `TBD_BackendText` | API/Http | Spawning/Deployment/TBD_DeploymentRequestQueue, MissionSelector/TBD_MissionDeploymentRelay
- `TBD_MissionLoader.IsSquadLeader` -> `TBD_MissionOrbatQuery` | Loaders/Mission | Spawning
- `TBD_BriefingService.SelfCheckWire` -> `TBD_BriefingWireSelfCheck.Run` | Briefing/Service | Gamemode/Orchestrator/TBD_FrameworkManager
- `TBD_MissionFlow.AllowsJoinAtStage`/`.JipPolicyName` -> `TBD_JipPolicy` | Gamemode/Orchestrator/Flow | Spawning/Identity/TBD_SpawnJoinAudit
- `TBD_ZoneVolume.Clear`/`.Read` -> `TBD_ZoneVolumeBounds` | Zones/Volumes | internal

## Leftovers

Per slice, for the closing runs.

- All slices: run `hcargo fmt --check -p xtask` or format only owned files (`rustfmt` via `hcargo fmt -- <file>` is unsafe on module files); never plain `fmt -p xtask` while another session has xtask edits.
- P1-3: box-drawing diagrams in 14 UI panel files; residual non-ASCII (x, <=, e-acute, bullet, section sign, check mark, emoji) e.g. `TBD_MissionSelectorScreen.c:4`, `TBD_LobbyScreen.c:4`; titles `TBD_UITheme.c:113`, `TBD_SpectatorCamera.c:49`. 165 above-line field docs left (over 120 columns). Owners fix via ECM-1/ECM-4/ECM-8.
- Wave A stale comments/paths (P3-C unless the folder owner fixes them first): `TBD_WinConditionEvaluator.c:44`, `TBD_WaypointRuntime.c:13`, `TBD_FrameworkManager.c:796` (`ResolveWinner` now on `TBD_ResultsPayload`), `TBD_MissionVehicleStruct.c:165`, `TBD_MissionLoader` `SpawnMissionEntities` comments, Admin/Lobby DeploymentAuthorization files, `TBD_SpectatorHost`, `TBD_LobbyStage`, `TBD_MissionSlotStruct`; docs `documentation_v2/refactor_pin_catalogue.md:158,179,187`, `map-engine/.../extensions/modules/README.md`, runbooks `mod_slice_workflow`, `game_server_staging/boot_and_log_verification`, `two_client_playtest/*`, `end_screen_specification.md`, `eden_gap_analysis.md`, `remaining_milestones.md`, `mission.schema.json:88,705`; Core/Characters and Core/Players READMEs "Used by: none"; dead code `TBD_Objective.c:57,145,161,257`; `GetSpawnZoneForFaction` has no callers.
- Wave B stale paths: `documentation_v2/glossary/n_to_z.md:117`, `apps/website/map-engine/src/data/scenario/extensions/objectives/win_conditions/README.md:40`, `documentation_v2/mod/tbd-framework/mod_design.md:72`, `apps/mod/tbd-framework/UI/layouts/Session/Lobby/README.md:91` (-> `UI/Pages/TBD_BriefingOrbatPage.c`), `UI/Mock/README.md:28` (BriefingCatalog path), `two_client_playtest/README.md:99` (`TBD_LoadoutEquipHelper.c`), `mission.schema.json` (`TBD_MarkerClient.SetRotation`). Stale member names: `TickWinConditions` (ObjectiveRegistry, ObjectivesComponent, MissionWinConditionChecks), `ArmRoundClock`/`ArmFactionEliminated`, `ApplyMissionFlow` (MissionFlowStruct), `ApplyEndScreens` (EndScreen), Safestart members in SpectatorHost/SpectatorHostEntity, `TBD_BriefingService.MAX_PAYLOAD_LINES`/`FIELD_MARK` (Lobby, Admin services), `TBD_BriefingController.c` (MissionBrowser, LobbyController, Spectator), `TBD_MissionBrowser.c:19`, `TBD_AdminService.c:47`, `TBD_SpectatorComponent.c:15-16`. Runtime log string with `T-941.7` in `TBD_RadioComponent.c`.
- P2-2: `TBD_TriggerRuntime.c:14-23` header rationale (P3-3); `documentation_v2/standards/templates/readme_mod_scripts.md:84` (P3-C).

## Ticket batch

Bugs noticed but not fixed, collected by P3-C and P6-C.

- P2-1: `Systems/AI/TBD_WaypointRuntime.c:481` leaks a non-group entity; `Session/Briefing/TBD_BriefingService.c:445` no null check on `doc`.
