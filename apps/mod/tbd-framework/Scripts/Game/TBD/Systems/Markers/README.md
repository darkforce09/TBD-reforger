# Mission map markers

Puts a [mission](/documentation_v2/glossary.md#mission)'s briefing markers on the in-game map: the
server sends each player only their own side's markers, and each client draws them as the engine's
own placed markers.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/
├── TBD_MarkerClient.c      the client's pull loop and map insertion, and the styled marker class
├── TBD_MarkerComponent.c   game mode component that starts the client and reports the marker manager
├── TBD_MarkerController.c  the request and reply RPCs on the player controller
├── TBD_MarkerData.c        TBD_MarkerService: which markers a player may see, and the wire and style codec
└── TBD_MarkerIcons.c       authored icon names to the engine's placed-marker icon indices
```

## How it works

```text
TBD_MarkerComponent.OnPostInit (a machine with a workspace) ──2.5 s──> TBD_MarkerClient.Start
TBD_MarkerClient: every 5 s until served, and on each map open (at most every 3 s)
  └─> SCR_PlayerController.TBD_RequestMarkers ──TBD_RpcAsk_Markers──> server
        TBD_MarkerService.BuildForPlayer(playerId): side = TBD_SpawnManager.GetAssignedSlot
        └─> TBD_MarkerStyleCodec.PackIntoX ──TBD_RpcDo_Markers──> owner client
              TBD_MarkerClient.Accept ──> SCR_MapMarkerManagerComponent.InsertStaticMarker (local)
```

- Side discipline: `BuildForPlayer` takes only a player id and reads the side from the server's
  slot assignment, so the request carries no faction a client could forge, and only that side's
  `briefings.<faction>.markers` rows leave the server. A player with no slot, or a server with no
  mission, gets an answer with `served` false and keeps asking; a served answer with zero rows
  stops the poll. At most `MAX_MARKERS` (64) rows are sent, labels cut to 64 characters, and a cut
  is logged.
- The wire: parallel arrays (`xs`, `zs`, `icons`, `labels`), so an empty label, which the schema
  allows, stays unambiguous. `Rpc()` takes at most eight parameters, so the six style columns (size,
  rotation, shape, brush, colour, alpha) travel as a six-int trailer on `xs`, and a mission whose
  markers are all default sends no trailer.
- Drawing: markers are `PLACED_CUSTOM` entries inserted local to the client, so the vanilla marker
  sync never re-broadcasts them; a styled row becomes a `TBD_StyledMapMarker`, which reapplies the
  authored size and alpha. `TBD_MarkerIcons.Resolve` tries the running game's own icon quad names
  first, then its alias table over `SCR_EScenarioFrameworkMarkerCustom`, and falls back to `DOT`
  for an unknown name.
- `TBD_MarkerComponent` also logs once, on every machine, whether `SCR_MapMarkerManagerComponent`
  and its `PLACED_CUSTOM` config are present on the game mode.
- Limitation of the engine's marker system: an account without the user-generated-content
  privilege sees no static markers, these included.

## Authority

- Server: `TBD_MarkerService` (`@authority server` on `BuildForPlayer` and `Build`) and
  `TBD_RpcAsk_Markers` (`@authority server`). On a listen host, `TBD_RequestMarkers` builds and
  accepts the payload in place without an RPC.
- Client: `TBD_MarkerClient`, started by `TBD_MarkerComponent` on any machine where
  `GetGame().GetWorkspace()` is not null (`TBD_MarkerClient.Start` carries `@authority client`).
- Owner: `TBD_RpcDo_Markers` runs on the requesting client only (`@authority owner`).
- RPCs, on the modded `SCR_PlayerController`:
  - `TBD_RpcAsk_Markers`: Reliable, Server (`@rpc Reliable Server`); takes no argument.
  - `TBD_RpcDo_Markers`: Reliable, Owner (`@rpc Reliable Owner`); the rows, the faction key, the
    mission id and `served`.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` (`TBD_MissionMarkerStruct` rows and the briefings) and
  `TBD_SpawnManager` (the caller's slot) under `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`;
  `TBD_Log`; the engine's `SCR_MapMarkerManagerComponent`, `SCR_MapMarkerBase`,
  `SCR_MapMarkerEntryPlaced` and `SCR_MapEntity`; the `marker` definition in
  `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_TaskHud` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`
  (`TBD_MarkerClient.FindMarkerManager`, `TBD_MarkerIcons.Resolve`, `MARKER_COLOR`);
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which attaches `TBD_MarkerComponent`.
- Rules: a side's markers never reach another side, and a request carries no faction; the reply goes
  to the owner only, and the client inserts markers locally; the marker RPC stays within the
  eight-parameter limit; lines added stay ASCII, and `cargo xtask mod compile` checks that the
  scripts compile, while which glyph an icon draws is checked in game.

## Related documentation

- [Tactical marker palette specification](/documentation_v2/mod/tbd-framework/UI/tactical_marker_palette/tactical_marker_palette_specification.md)
  — the design for player-placed tactical markers on the same map
- [Map symbology](/documentation_v2/design_system/map_symbology.md) — the marker symbols and colours
  in game beside the Mission Creator's, and where the two disagree.
