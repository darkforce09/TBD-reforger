# Objective runner

The game mode component that runs the mission's objectives once a second while the round is live:
it samples who stands on which objective, advances each objective through its kind behaviour,
puts completions in chat and sends each player their objective board.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/Runtime/
├── TBD_ObjectiveHudPublisher.c  renders each player's board and capture bar, sends it only on change
├── TBD_ObjectiveProgression.c   advances each usable objective by one tick through its kind behaviour
└── TBD_ObjectivesComponent.c    TBD_ObjectivesComponent: the 1 Hz server tick, presence, delivery
```

## How it works

`TBD_ObjectivesComponent` sits on `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`. Its
`OnPostInit` builds its two helpers and, off a client, arms one repeating 1000 ms `CallLater`
tick; `OnDelete` cancels it, clears `TBD_ObjectiveRegistry` and the shared
`TBD_ObjectiveKindBehaviour` instances, and hides every HUD. Each tick:

1. Until `TBD_ObjectiveRegistry.Build()` succeeds, nothing else runs; the first success logs the
   armed counts (`TBD_ObjectiveRegistry.UsableCountsSummary`, in the lookup's order), or that
   the mission has no usable objective, once per world.
2. Outside `LIVE` the HUDs are hidden and nothing advances; progress is kept. On the first `LIVE`
   tick each usable objective's behaviour runs `OnEnterLive`: a destroy objective arms its target
   search and a hold skips the announcement rungs above its length.
3. `SamplePresence` walks the connected players once and counts each living body with a life left
   (`TBD_SpawnManager.IsPlayerDead`, `TBD_CharacterUtil.IsDead`) and a side
   (`TBD_PlayerFaction.Of`) into every usable objective whose zone and height band contain it.
4. `TBD_ObjectiveProgression.Advance` hands each usable objective to
   `TBD_ObjectiveKindBehaviour.For(kind).Advance`: a capture freezes on a contest, tears down
   another side's progress or ownership at the neutralize rate, then builds toward ownership; a
   hold clock runs, pauses or resets by its rules; a destroy objective re-counts its targets. The
   behaviour returns the `CAPTURED`, `DESTROYED` or `HELD` line on the completing tick and the
   progression queues it; progress, contests, hold rungs and neutralisations are logged on
   `[TBD][Obj]`.
5. The completion lines go to every player's chat through `TBD_PlayerChat`, then
   `TBD_ObjectiveHudPublisher.Replicate` renders each player's snapshot (row glyph, title, status
   and task text, and the capture bar of the objective they stand in whose behaviour claims it)
   and sends it only when its length-prefixed signature differs from the last one that player was
   sent. The inert, complete and contested glyphs are generic; the rest comes from the behaviour's
   `HudIcon`.
6. A met objective end trigger is logged once, with a banner; `TBD_FrameworkManager` ends the round.

A client's pull (`SCR_PlayerController.TBD_RequestObjectiveHud`) reaches `PushHudTo`, which sends
the board ungated.

## Authority

- Server: everything. `OnPostInit` arms the tick only when `TBD_Authority.IsClient()` is false
  (`@authority server`); the publisher's pushes and the chat lines leave from the server tick.
- Client: nothing runs; the component exists and tears down without a tick.
- Owner: each player's snapshot is built from their own side and sent to them alone.
- RPCs: none declared here; `SCR_PlayerController.TBD_PushObjectiveHud` carries the snapshot over
  the Reliable Owner RPC that `mod/tbd-framework/Scripts/Game/TBD/UI/Hud/TBD_ObjectiveHud.c`
  declares.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_ObjectiveRegistry`, `TBD_Objective` and `TBD_ObjectiveText` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Engine/`; the kind behaviours
  (`TBD_ObjectiveKindBehaviour` and its subclasses) in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/Types/`;
  `TBD_ZoneVolume` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`;
  `TBD_FrameworkManager` (the stage); `TBD_SpawnManager`; `TBD_PlayerChat`, `TBD_PlayerFaction`,
  `TBD_CharacterUtil`, `TBD_AnnounceOnce` and `TBD_Log` in
  `mod/tbd-framework/Scripts/Game/TBD/Core/`; `SCR_PlayerController.TBD_PushObjectiveHud`.
- Used by: `mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches the component;
  `TBD_ObjectiveHud.c` in `mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`, which calls
  `GetInstance` and `PushHudTo`; `TBD_FrameworkManager`, which lists the component in its roll call.
- Rules: nothing advances outside `LIVE`; progress is cleared only with the world; rates never
  scale with headcount; the 1 Hz push is sent only on a changed snapshot and a player with no
  record always gets a full board; the component class name is referenced from the prefab and
  never changes.
