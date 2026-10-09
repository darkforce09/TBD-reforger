**Status:** live

# T-1092 — Progress checkpoint

The resume file for [T-1092](/documentation/tickets/specs/t1092_mod_script_modularisation.md).
To resume: read this file, then continue the roster of the
[plan](/documentation/tickets/plans/t-1092_plan.md) from the first row that is not `done`.
Statuses: `pending`, `running`, `done — awaiting commit`, `done`, `blocked`.
Ids are the plan's slice ids, except that the plan's two-digit phase 3 slices are written
"P3 slice 10" to "P3 slice 14" here: `cargo xtask ticket check --strict` rejects a phase digit,
hyphen and two digits in live documentation, the spelling of the retired priority backlog ids.

## Frozen class names

Referenced from `.et`, `.layout`, `.conf` or `.ent` files in `mod/tbd-framework`; no slice
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

Another session holds uncommitted work in `mod/tbd-export`, `tools/xtask`
(`commands/mod_ops`, `commands/deploy`, `commands/generate`, `Cargo.toml`), `tools/developer_tools`,
`crates/api/api_server`, `crates/frontend/shell/frontend_application`, `contracts`, the untracked `equipment` folder of `assets`, `CLAUDE.md`, `.gitignore` and `Cargo.lock`.
Commits of this program stage by pathspec only.

## Roster

| Id | Status | Commit | Result |
|---|---|---|---|
| P0 | done | d5cb82b97 | Tickets T-1092.1–.6, spec, plan, checkpoint |
| P1-1 | done | 87d831968 | `.rs`+`.c` walk, `MOD_SCRIPT_ROOTS` + const assert on mod pins; 14 tests (filter `file_length_tests`, not `node_free`). Slip: `hcargo fmt -p xtask` reformatted the other session's uncommitted `commands/mod_ops/mod.rs` and `equipment_vehicle_export/publication.rs` (format only, left in place). |
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
| P3 slice 10 | done | c7afddd02 | Briefing Service/, Catalog/, UI/Pages, UI/Navigation, map launcher; wire bytes unchanged; self-check armed from Serialise now runs. |
| P3 slice 11 | done | e198b3e64 | Lobby Service/, Catalog/, PreSlot/, UI/Roster, UI/Kit; deployment-authorization folded into `ApplyDeploy`; ticket ids removed from one attribute desc and one log line. |
| P3 slice 12 | done | 58163e9ec | Spectator Host/ (6) and Controller/ (4); Players and PostGame one type per file; DebriefScreen uses `TBD_WireCodec` separators. |
| P3 slice 13 | done | 8210e3784 | Admin (subcommands, audit, snapshot via codec), Admin/UI sections; MissionSelector Catalog/ and inspector cards; browser RPCs on the Admin player controller. |
| P3 slice 14 | done | 8e6fd61eb | UI Common Dropdown/Inputs/Layout/SessionChrome, Core Theme/Screens/Controls; HUD RPCs to `UI/Hud/SCR_PlayerController.c`. Framework: 0 findings in 370 scripts, max 477 lines, compile 0, world-boot PASS. |
| P3-C | done | 4518a8665 | Forwarders removed, stale references fixed across scripts and docs, dead `TBD_Objective` members dropped, T-1219 filed; order line matches baseline. P3-C2 (this commit) filed T-1220..T-1228 for the bugs the checkpoint had missed. |
| P4-1 | done | a055c173d, f6f350b51 | tbd-framework pinned in `file-length` and `enfusion-comments`; `verify-coding-standards` and `ci.yml` language-gates run the comment gate; `task_definitions.rs` map steps split to `map_asset_steps.rs`; checkpoint moved here (plans/ takes only `t-<id>_plan.md`); CLAUDE.md law 7 and section 3 committed separately; the law 8 sentence ("machine-checked by cargo xtask verify enfusion-comments") sits inside the other session's uncommitted law 8 rewrite and lands with it. ci-local exit 1 only on the other session's work (editorconfig in untracked assets/equipment, api_v2 rustfmt, 12 equipment route tags, tbd-export and website README coverage). |
| OP-1 | done (waived) | | Operator 2026-09-26: playtest not relevant in pre-alpha; closed. |
| P5-1 | done | 3499895f8, c5843279a | ModifyEntity/ split (6 files), 18 handlers documented, 38 JsonApiStruct request/response classes renamed `...Wire` (ECM-6; handler names, action names and JSON keys unchanged; broker calls handlers only); tbd-emcp pinned in both gates; CLAUDE.md law 7 names it. Operator Workbench compile: clean (2026-09-26). Operator accepted the `...Wire` rename. |
| P6-1 … P6-6 | blocked | | Operator is working on tbd-export now and will say when P6 may start. |
| P6-C | pending | | |

## Amendments

Additions to a launch prompt beyond concrete values, by slice id.

| Slice | Addition |
|---|---|
| P3 slices 11…14 (wave C) | Brief file gains a "Bugs" section (operator 2026-09-26: bugs are noted with file:line, never fixed) and the wave B helpers (`EndRound`, `CountSurvivors`, `TBD_WarnOnce`, `TBD_AnnounceOnce`, `TBD_Rounding`). Slice notes carry each folder's stale-comment leftovers; P3 slice 13 switches `TBD_MissionDeploymentRelay` off the `JsonEscape` forwarder. |
| P3 slices 6…10 (wave B) | Brief file gains wave A lessons: name split files after their primary type (ECM-9; companion structs live with their owner, enums get their own file); run `readme-coverage`/`link-check` with `--with-untracked`. |
| P3-1…P3-5 | Delivered as one scratch brief file (B0 + CARD + SPLIT RULES + writer steps, verbatim) plus a "Parallel wave rules" block: judge compile by own-file errors only; git mv is fine, no other git add/reset. Slice notes add: heartbeat owns Tick calls (keep static Tick signatures); P3-3 adds `TBD_ZoneRegistry.FindById` and `TBD_TriggerRuntime.HasFired` for other slices; P3-5 must not touch `TBD_DebriefScoreboard.c`. |
| P2-2 | Also update tools/checks/mod_script_checks/src/enfusion_comments/network_authority_rule.rs:25 so `TBD_Authority.IsClient()`/`IsServer()` calls count as context-dependent (the P2-1 replacement hid 82 sites), with a test. |
| P1-3 onward | B0 gains: "Never run `hcargo fmt -p <package>` (it reformats the other session's files); check with `hcargo fmt -p xtask -- --check` and format only your own files." |
| P1-2 | Concurrency note: P1-1 edits `language_bans/` and node_free_tests.rs at the same time; the other session's uncommitted `tools/xtask` edits are reported, not fixed, if they break the build. |

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
- Run `hcargo xtask verify link-check --with-untracked --path <f>`; without the flag, new
  untracked files fail.
- Shared helpers now also include `TBD_ZoneRegistry.FindById`, `TBD_TriggerRuntime.HasFired(id,
  out bool unknownId)` and `TBD_TriggerRuntime.FindById`.
- A stale comment in your folders that names a moved file or member is yours to fix.
```

## Pause point

Session paused 2026-09-26 after wave B (operator: session budget); resumed the same day with wave C. Wave C landed; next is P3-C. Tree at pause: `mod compile` 0, `mod world-boot` PASS, 2074 comment findings in 78 of 324 framework scripts.

## Comment gate baseline

Filled by P1-2 and P1-3.

| Tree | When | ECM-1 | ECM-2 | ECM-3 | ECM-4 | ECM-5 | ECM-6 | ECM-7 | ECM-8 | ECM-9 | Total |
|---|---|---|---|---|---|---|---|---|---|---|---|
| tbd-framework (172 files) | after P1-2 | 2890 | 187 | 1179 | 2429 | 49 | 80 | 19 | 4105 | 46 | 10984 |
| tbd-framework | after P1-3 | 126 | 187 | 1179 | 2312 | 49 | 80 | 19 | 1425 | 43 | 5420 |
| tbd-framework | after wave A | 117 | 138 | 935 | 1538 | 36 | 37 | 19 | 773 | 31 | 3624 |
| tbd-framework (324 files) | after wave B | 100 | 78 | 586 | 929 | 20 | 7 | 19 | 320 | 15 | 2074 |
| tbd-framework (370 files) | after wave C | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| tbd-emcp (19 files) | after P1-3 | 0 | | | 158 | | | | 0 | | 458 |

## Shared helper index

Filled by P2-1: path | class.method signature | replaces (file:method) | parameter notes.

Paths are under `mod/tbd-framework/Scripts/Game/TBD/`. All helpers are static.

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

Left by P3 slices; P3-C removed every one by pointing its callers at the owner.

- `TBD_GameRuntimeHttp.JsonEscape` -> `TBD_BackendText.JsonEscape` (DeploymentRequestQueue; MissionDeploymentRelay already switched): removed.
- `TBD_MissionLoader.IsSquadLeader` -> `TBD_MissionOrbatQuery.IsSquadLeader` (SlotBodyDressing): removed.
- `TBD_BriefingService.SelfCheckWire` -> `TBD_BriefingWireSelfCheck.Run` (FrameworkRollCall): removed.
- `TBD_MissionFlow.AllowsJoinAtStage`/`.JipPolicyName` -> `TBD_JipPolicy.AllowsJoinAtStage`/`.Name` (SpawnJoinAudit): removed.
- `TBD_UITheme.ChipInk`/`PanelFill`/`PanelBorder`/`FactionRowFill`/`FactionRowBorder`/`FactionRowInk` -> `TBD_UITintColours` (23 call sites in 12 Session files): removed.
- `TBD_ZoneVolume.Clear`/`.Read` -> `TBD_ZoneVolumeBounds` (ObjectiveRegistry): removed.
- Kept: the one-line `TBD_SpawnManager` members that hand off to its helpers (`ClaimSlot`, `ReleaseSlot`, `BuildSlotRoster`, `MaterializeSlotBodies`, `GetSlotBody`, `AdminRespawn`, ...) are the component's permanent API: Lobby, Admin, Briefing and fleet code reach the spawn system only through `TBD_SpawnManager.GetInstance()`, and the helpers are owned by `ref` and not reachable from outside.

## Leftovers

Cleared by P3-C: stale names and paths in the framework scripts, READMEs, the named docs
(pin catalogue rows, AI README sample in the template, two-client playtest runbooks, staging boot
table, slice workflow, end-screen, lobby, safe-start and debrief specs, glossary, requirements.json,
remaining milestones, Eden gap analysis, the two map-engine READMEs, `mission.schema.json`
descriptions), plus `documentation/standards/documentation_standards.md` (its Lobby examples named
a deleted file); ticket ids in four runtime log strings and one attribute desc; dead
`TBD_Objective` members (`HasEnemyPresent`, `ResolveActingFaction`, `LogKey`,
`m_sPendingInsideMessage`).

Remaining, outside P3-C's paths:

- `TBD_MissionLoader.GetSpawnZoneForFaction` has no callers; kept (the loader's static read API keeps its names), ticketed as T-1219.
- Stale member names in files no slice owns: `.world-boot-warning-baseline:41` (`TBD_FrameworkManager.ArmRoundClock`), `crates/mission/mission_compiler/src/game_document/tests/cases_4.rs` lines 256, 264 and 268 (assert messages naming `OnEnterBriefing`, `ArmRoundClock`, the JIP door on `TBD_SpawnManager`), `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/env.rs:154` (`TBD_FrameworkManager.ArmRoundClock`), `crates/mission/mission_model/src/spawn_modules/spawns.rs:11` (`SpawnManager's EngineFactionKey`, now `TBD_SlotBodyMaterializer`).
- All slices: run `hcargo fmt --check -p xtask` or format only owned files; never plain `fmt -p xtask` while another session has xtask edits.

## Ticket batch

Bugs noticed but not fixed, collected by P3-C and P6-C.

- P5-1 (to file at P6-C): `mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_ExecuteAction.c` lines 82 and 87 discard `Trim()` results; `ModifyEntity/EMCP_WB_ModifyEntityPropertyActions.c:31,59` setProperty/clearProperty run outside an entity action (not one undo step); `EMCP_WB_Layers.c:19` `visible`/`subScene` unused, no setVisible action; `EMCP_WB_SelectEntity.c:139` "select" only clears the selection yet answers ok; `EMCP_WB_EditorControl.c:97` saveAs runs Save.

- P2-1: `Systems/AI/TBD_WaypointRuntime.c:481` leaks a non-group entity: no ticket; resolved by P2-1's `TBD_AIGroupFactory.SpawnGroup`, which deletes a non-group spawn (WaypointRuntime calls it).
- P2-1: `Session/Briefing/TBD_BriefingService.c:445` no null check on `doc`: no ticket; resolved by P2-1's `TBD_MissionFactionNames.DisplayName` (guards null), and `Build` returns before any read when `doc` is null.
- P3-C: END reason and winner lost for extraction, VIP and trigger endings, and the false "round will NOT end" banner: still reproduce at `Gamemode/Stages/WinConditions/TBD_WinConditionEvaluator.c:139`, `Systems/Zones/Triggers/TBD_TriggerFlowEffects.c:85` (`SetStage(END)` without `EndRound`) and `Gamemode/Objectives/Engine/Runtime/TBD_ObjectivesComponent.c:303`: existing T-1082 (its paths predate the splits).
- P3-C: session top bar count icon loads key `group`: still at `UI/Common/SessionChrome/TBD_SessionTopBar.c:104`: existing T-1098.
- P3-C: `Systems/Mission/Loaders/Validation/TBD_MissionStructureChecks.c:239` warns that `GetSpawnZoneForFaction` cannot place from a spawn zone, but nothing calls it: T-1219.
- P3-C2: occupant found by exact float equality on X/Z at `Systems/Zones/PlayArea/TBD_PlayAreaVehicleAxis.c:232`: T-1220.
- P3-C2: `environment` warned as unconsumed though `TBD_EnvironmentReader` applies fog, wind and view distance at `Systems/Mission/Loaders/Validation/TBD_MissionUnconsumedKeyCheck.c:39`: T-1221.
- P3-C2: `ResolveWinner` duplicates `TBD_FactionElimination.CountSurvivors` at `API/Results/TBD_ResultsPayload.c:25`: T-1222.
- P3-C2: `AcquireRow` reuses a pooled row whatever its container at `Session/Lobby/UI/TBD_LobbyFactionPanel.c:189`: T-1223.
- P3-C2: `GetFirstKey` reads `m_Catalog` without a null check at `Session/Lobby/UI/TBD_LobbyFactionPanel.c:134`: T-1223.
- P3-C2: a seat switch raises the change event twice at `Session/Lobby/Catalog/TBD_LobbyCatalog.c:181`: T-1224.
- P3-C2: `Reconcile` keeps issuing hosts after `StandDown` at `Session/Spectator/Host/TBD_SpectatorHostLifecycle.c:58`: T-1225.
- P3-C2: log line says F6 opens the selector (the key is F9) at `Session/MissionSelector/SCR_PlayerController.c:64`: covered by T-1084.
- P3-C2: browser RPC refusals check the admin list directly and skip the admin audit trail at `Session/MissionSelector/SCR_PlayerController.c:130`: T-1226.
- P3-C2: `GetTerrain` and `FindTerrain` are duplicates at `Session/MissionSelector/Catalog/TBD_MissionCatalog.c:56`: T-1227.
- P3-C2: overlay host shown before the menu is created at `UI/Common/Dropdown/TBD_DropdownComponent.c:220`: T-1228.
- P3-C2: debrief board format omits the deaths field at `Gamemode/Orchestrator/Stage/TBD_EndBanner.c:116`: resolved: both doc comments (`TBD_EndBanner.c:116`, `TBD_FrameworkManager.c:47`) now read `kills\tdeaths\tfaction\trole\tname`, matching `TBD_DebriefScreen.PackRows`.
