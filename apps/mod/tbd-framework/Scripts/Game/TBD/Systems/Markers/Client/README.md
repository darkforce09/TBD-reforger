# Mission map markers on the client

The client half of mission map markers: asking the server for this player's markers, and putting
them on the in-game map as the engine's own placed markers.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/Client/
├── TBD_MarkerApplier.c    builds one placed marker per row, inserts it locally, and removes it again
├── TBD_MarkerClient.c     the pull loop: poll until served, re-ask on map open, take each answer
└── TBD_StyledMapMarker.c  a placed marker that applies an authored size and opacity to its widget
```

## How it works

`TBD_MarkerClient.Start` subscribes to map open, arms a 5 s poll and asks at once through
`SCR_PlayerController.TBD_RequestMarkers`. The poll stops once the server answers with `served`
true; each map open asks again, at most every 3 s. `TBD_MarkerClient.Accept` takes each answer:

- Unserved: clears the map; when the player was served before, the poll is re-armed.
- Served: `TBD_MarkerStyleCodec.UnpackFromX` reads the style trailer, `TBD_MarkerApplier.Clear`
  removes the previous markers and `TBD_MarkerApplier.ApplyRows` inserts the new ones, each a
  `PLACED_CUSTOM` marker with its icon (`TBD_MarkerIcons.Resolve`), nearest palette colour,
  rotation and caption. A row with a size other than 100 or an alpha other than 255 becomes a
  `TBD_StyledMapMarker`, which resizes its images one frame after the widget exists.

An area shape has no fill surface on a placed marker: it warns once per world or mission and draws
the icon. The client logs one `applied` line only when the mission, side or count changes.

## Authority

- Server: none.
- Client: all three classes (`@authority client`); started by `TBD_MarkerComponent` on a machine
  with a workspace, and shut down in its `OnDelete`.
- Owner: `TBD_MarkerClient.Accept` runs on the requesting client (`@authority owner`), reached
  through the modded `SCR_PlayerController` in the parent folder.
- RPCs: none declared here; see [the Markers README](../README.md#authority).
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MarkerService.CH_MARKERS`, `TBD_MarkerStyleCodec` and `TBD_MarkerIcons` in
  [the parent folder](../README.md); `TBD_Log`; the engine's `SCR_MapMarkerManagerComponent`,
  `SCR_MapMarkerBase`, `SCR_MapMarkerEntryPlaced` and `SCR_MapEntity`.
- Used by: `TBD_MarkerComponent` and the modded `SCR_PlayerController` in the parent folder;
  `TBD_MarkerIcons` (`FindMarkerManager`); `TBD_TaskHud` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/` (`TBD_MarkerClient.FindMarkerManager`).
- Rules: markers are inserted with `isLocal` true and never get a marker id, so they never
  replicate and removing them never asks the server; the player cannot remove a mission marker;
  lines added stay ASCII.
