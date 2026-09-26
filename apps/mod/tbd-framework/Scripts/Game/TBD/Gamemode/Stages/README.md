# Round stages, safe start and authored win rules

The stage vocabulary of a TBD round, the safe start that keeps everyone unhurt from the lobby until
the round goes live, and the evaluator that ends the round on a
[mission](/documentation_v2/glossary/g_to_m.md#mission)'s authored `extraction` or `vip` win rule.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/
├── TBD_GameStage.c              TBD_EGameStage: the seven round stages, LOADING to DEBRIEF
├── TBD_SafestartManager.c       game mode component: damage off, rounds deleted, countdown to LIVE
└── TBD_WinConditionEvaluator.c  winConditions.mode: ends the round on extraction or a VIP's fate
```

## How it works

`TBD_EGameStage` is the one stage enum every folder reads; `TBD_FrameworkManager` in
`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/` owns the current stage and calls
`TBD_SafestartManager.OnStageChanged` on every transition.

### Safe start

`TBD_SafestartManager` is a `SCR_BaseGameModeComponent` on
`apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`. Its shield arms on `LOBBY`, `BRIEFING`
and `SAFE_START` and lifts on any other stage:

1. Arm: every player body, every AI agent's body and every unpossessed
   [slot](/documentation_v2/glossary/n_to_z.md#slot) body from `TBD_SpawnManager` gets
   `EnableDamageHandling(false)`, weapon safety on, and shot and grenade handlers that delete the
   projectile the moment it exists. A `TBD_SafestartHold` per body records the damage-handling value
   found before the first change, and a re-sweep every `SWEEP_MS` (3 s) covers bodies that appear
   later.
2. Countdown: on `SAFE_START` only, `m_iSecondsRemaining` counts down once a second from the
   configured length (`DEFAULT_COUNTDOWN_SECONDS` 300, accepted range 5 to 3600, set by the
   mission's `flow.safeStartSeconds` or `#tbd safestart <seconds>` through `AdminSetSeconds`). Chat
   milestones go to everyone; at zero, or on `#tbd safestart go`, `GoLive` asks the stage machine
   for `LIVE`.
3. Lift: the armed flag clears first, so a handler that fails to unregister is inert; then each
   held body gets back the value it had, verified by read-back. A body that is still unrestored
   starts a watchdog that retries every 5 s and logs at ERROR until the set is empty.

On the clients the replicated countdown drives `SCR_PopUpNotification` banners
("SAFESTART — WEAPONS COLD", "SAFESTART OVER — WEAPONS LIVE"); `FormatClock` and the milestone
tables are shared with the round clock. The shield covers characters only, cannot holster a
weapon, and leaves melee, falls and vehicle impacts to the damage switch alone.

### Authored win rule

`winConditions.mode` names one of five modes. `attrition`, `objective` and `timeout` end through
`TBD_FrameworkManager` and `TBD_ObjectiveRegistry`; this evaluator only warns once, at the first
live tick, when the mission's `endOn` list cannot satisfy them. `extraction` ends the round when
every living player of one side stands in the `extractionZoneId` zone (a side with no living player
does not count); `vip` ends it when the `vipSlotId` player dies (`vip_down`; the one other fielded
side wins) or, with an extraction zone, reaches it (`vip_extracted`). A `modded class
SCR_BaseGameMode` arms a self-re-arming 2 s tick (`TICK_MS`) in a framework world, and the
`s_bEnded` latch ends the round exactly once per world. The evaluator reads its keys with its own
`JsonLoadContext` pass over `TBD_MissionLoader.GetRawJson()` into `TBD_WinConditionsStruct`.

## Authority

- Server: the shield, the sweeps, the countdown, the lift and the watchdog
  (`TBD_SafestartManager.OnPostInit` stops on `RplMode.Client`, and every mutator carries
  `@authority server`), and the win rule (`OnGameStart` returns on `RplMode.Client`; `Tick` carries
  `@authority server`).
- Client: `OnCountdownReplicated` (`@authority client`) shows the safe start pop-ups; the server
  calls the same helper from `SetCountdown`, since the hook never fires on the authority.
- Owner: nothing.
- RPCs: none.
- Replicated properties: `TBD_SafestartManager.m_iSecondsRemaining`, the countdown in seconds or
  `NOT_RUNNING` (-1), with the hook `OnCountdownReplicated`
  (`@replicated m_iSecondsRemaining`).

## Boundaries

- Depends on: `TBD_FrameworkManager` (the stage and `SetStage`); `TBD_SpawnManager` (slot bodies,
  claimed and living players per side), `TBD_MissionLoader` (the raw JSON, slots, factions and
  `HasEndTrigger`) and `TBD_ZoneRegistry` (the extraction zone) under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`; `TBD_Log` and `TBD_PlayerChat`; the engine's
  `SCR_CharacterDamageManagerComponent`, `CharacterControllerComponent` and
  `SCR_PopUpNotification`; the `winConditions` definition in
  `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_FrameworkManager`, which drives safe start, borrows `FormatClock` and refuses
  `SAFE_START` on a world without the component; `TBD_AdminService` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/` (`#tbd safestart status|go|<seconds>`);
  `TBD_EGameStage`, read across `API/`, `Session/`, `Systems/` and `UI/` under
  `apps/mod/tbd-framework/Scripts/Game/TBD/`; and
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches `TBD_SafestartManager`.
- Rules: the lift clears the armed flag before touching any body and hands each body back the value
  it had, never `true` by default; `TBD_EGameStage` order is the admin `#tbd stage next` order, so a
  new stage goes where it runs; the win latch clears in `OnGameStart`, because statics outlive a
  world; lines added to a script stay ASCII, and `cargo xtask mod compile` checks that the scripts
  compile, while whether damage stays off is checked in a round.

## Related documentation

- [Safe start HUD specification](/documentation_v2/mod/tbd-framework/UI/safe_start_hud/safe_start_hud_specification.md)
  — the safe start notices as built and their design target
- [End screen specification](/documentation_v2/mod/tbd-framework/UI/end_screen/end_screen_specification.md)
  — the END banner that names the win rule's endings
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — one life and the
  [event](/documentation_v2/glossary/a_to_f.md#event) loop the stages follow
