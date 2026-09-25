# Briefing

The pre-game briefing: the server builds each player's side-specific briefing from the loaded
[mission](/documentation_v2/glossary.md#mission) and sends it to that player alone, tallies who has
marked ready, and opens the Briefing screen on every client when the round enters `BRIEFING`.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/
├── TBD_BriefingCatalog.c     the Briefing screen's read surface: nets, objectives, rules, assets, plans
├── TBD_BriefingClient.c      client cache of the last briefing and ready tally, with invokers
├── TBD_BriefingController.c  modded `SCR_PlayerController`: briefing and ready RPCs, the stage handler
├── TBD_BriefingData.c        the briefing payload models: roles, groups, zones, kit lines
├── TBD_BriefingService.c     server payload builder for one player's side, and the wire format
└── UI/                       the Briefing screen: map, navigation and the ten pages
```

## How it works

```text
TBD_FrameworkManager (server) enters BRIEFING -> pushes TBD_OnStageChanged to each controller
  (a joining client reads the replicated stage once instead)
client TBD_OnStageChanged(BRIEFING) -> TBD_BriefingClient.Reset, TBD_MenuStack.Open(TBD_UIBriefing)
client TBD_BriefingClient.Request -> TBD_RpcAsk_Briefing  --RPC-->  server
server TBD_BriefingService builds the caller's side from TBD_SpawnManager and TBD_MissionLoader
       <--RPC-- TBD_RpcDo_Briefing(wire, situation, mission, execution) -> TBD_BriefingClient.Accept
client Ready & Continue -> TBD_BriefingClient.ReportReady -> TBD_RpcAsk_Ready  --RPC-->  server
server TBD_BriefingReadyRegistry records it <--RPC-- TBD_RpcDo_ReadyTally(own side's tally, accepted)
```

`TBD_OnStageChanged` is the only thing that opens or closes the screen: it opens it on `BRIEFING`
and closes it on any other stage. The server resolves the caller's side from its own state, so a
player receives only their side's briefing and their own side's ready tally. The briefing wire is
tab-separated lines of at most `MAX_PAYLOAD_LINES` (400), every field prefixed with `.`, with the
orders' situation, mission and execution texts sent as separate string arrays; its log channel is
`Briefing`.

`TBD_BriefingCatalog` is what the pages draw: faction, nets, objectives, rule groups, lore,
parameters, assets and uniforms for both sides, and plans. Its `Get()` builds it from
`TBD_BriefingMock` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/` until something calls
`Set()`, and no script does, so the pages show mock content while the payload above reaches
`TBD_BriefingClient`.

## Authority

- Server: `TBD_BriefingService` and `TBD_BriefingReadyRegistry`, and the server halves of the RPCs
  (`@authority server`); the caller is `GetPlayerId()` of the controller the request arrived on.
- Client: `TBD_BriefingClient`, `TBD_BriefingCatalog`, the stage handler (`@authority client` on the
  joining-client read) and the screen.
- Owner: `TBD_RpcDo_Briefing` and `TBD_RpcDo_ReadyTally` (`@authority owner`), on the requesting
  client only.
- RPCs, on the modded `SCR_PlayerController`, as their `@rpc` tags state: `TBD_RpcAsk_Briefing`
  and `TBD_RpcAsk_Ready` (Reliable, Server); `TBD_RpcDo_Briefing` and `TBD_RpcDo_ReadyTally`
  (Reliable, Owner).
- Replicated properties: none here; a joining client reads `TBD_FrameworkManager`'s replicated
  stage.

## Boundaries

- Depends on: `TBD_SpawnManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`;
  `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`;
  `TBD_FrameworkManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Orchestrator/`;
  `TBD_Registry` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_SessionSelection` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`; `TBD_MenuStack` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; `TBD_BriefingMock` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`.
- Used by: `TBD_FrameworkManager`, which pushes `TBD_OnStageChanged`; `TBD_DockScreen` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/` (the top-bar Briefing tab).
- Rules: a player receives only their own side's briefing and tally, resolved on the server; the
  stage handler is the one opener and closer of the screen; sources stay ASCII and
  `cargo xtask mod compile` checks that they compile.

## Related documentation

- [Briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the briefing's design target
