**Status:** live

# Tactical marker palette

The map markers a player sees in game. The [mod](/documentation_v2/glossary.md#mod) puts each
side's briefing markers, authored with the [mission](/documentation_v2/glossary.md#mission) in the
[Mission Creator](/documentation_v2/glossary.md#mission-creator), on that side's in-game map, and
the task markers beside them. The palette the specification designs, a toolbar with which leaders
draw channel-scoped markers during play, is not built: the mod adds no marker UI of its own.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/README.md):
  `TBD_MarkerData.c`, whose `TBD_MarkerService` decides which markers a player may see and packs
  them; `TBD_MarkerClient.c`, the client's request loop and map insertion; `TBD_MarkerIcons.c`,
  authored icon names to the game's marker icons.
- Task markers: `TBD_TaskHud` in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/README.md).
- Entry: `TBD_MarkerComponent` on `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`, which
  starts the client 2.5 s after init on a machine with a workspace.
- Layout: none; markers are the game's own static map markers, drawn by its map UI.
- Related features: the [briefing](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md),
  whose Markers mode reads the same map; the
  [objective capture HUD](/documentation_v2/mod/tbd-framework/UI/objective_capture_hud/objective_capture_hud_specification.md),
  the live objective board.

## Behaviour

### Briefing markers

1. Once started, the client asks the server for its markers every 5 s until it is served, and
   again each time the player opens the map, at most once every 3 s.
2. The server reads the asker's side from the [slot](/documentation_v2/glossary.md#slot)
   `TBD_SpawnManager` assigned, never from the request, and answers with only that side's
   `briefings.<faction>.markers` rows. A player with no slot, or a server with no mission, gets an
   unserved answer and keeps asking; a served answer with no rows stops the loop.
3. At most 64 markers go to one player, each label cut to 64 characters; a cut is logged.
4. The client inserts each marker locally as a `PLACED_CUSTOM` static marker: the authored icon,
   resolved through the game's icon names and an alias table and falling back to a dot, the label
   as its text, and the authored size, rotation, shape, brush, colour and alpha.
5. Markers inserted locally are never re-broadcast by the game's marker sync, so no side sees
   another's.

### Task markers

`TBD_TaskHud` draws each assigned task that has a position as a static marker with the task's
icon and title. Every player gets the same tasks; a task that leaves `assigned` disappears.

### Placing markers during play

The mod adds no way to place, draw, edit or erase a marker. It neither extends nor scopes the
game's own map-marker tools.

### Known discrepancies

- The client loop starts wherever a workspace exists (`TBD_MarkerComponent.c`) — but a headless
  dedicated server has a workspace too, so it runs the loop there as well.
- An account without the game's user-generated-content privilege sees no static markers, these
  included; the game's marker system enforces that.

## Data

The feature makes no HTTP call. Its wire, on the modded `SCR_PlayerController` (the markers
README's [Authority](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/README.md#authority)
lists both RPCs):

- `TBD_RpcAsk_Markers`, Reliable, Server, with no argument; `TBD_RpcDo_Markers`, Reliable, Owner:
  parallel arrays of positions, icons and labels, the faction key, the mission id and `served`.
  The six style columns ride as a trailer on the positions array, because an RPC takes at most
  eight parameters; a mission whose markers are all default sends no trailer.
- The rows follow `mission.schema.json#/$defs/marker` in `contracts_v2/definitions/`.

## Design

- As built: the game's own map markers and map UI; nothing on screen belongs to the mod.
- Design target: the specification's wireframe; the folder has no mockup set. A floating toolbar
  docked on the map edge, for leaders, of which nothing is built:
  - a header with the active channel and channel tabs: `[S]` side, `[C]` command (platoon HQ and
    squad leaders), `[G]` squad, `[V]` vehicle occupants, `[A]` admin and all sides;
  - tools: dot, arrow (click and drag), polyline for phase lines and boundaries, NATO symbol, text
    label and eraser, with undo (Ctrl+Z), "Clear My Markers" and a leader-only "Clear Channel";
  - a NATO sub-palette for hostile and friendly infantry, armour, mechanised, recon, anti-tank,
    anti-air, mortar and HQ, and points (objective, point of interest, observation post, rally
    point, medical, ambush);
  - solid, dashed and dotted lines in 2 or 4 px, and blue, red, green, yellow, orange and
    white/black swatches with a meaning each (friendly, enemy, allied, unconfirmed, control
    measures);
  - server validation of each placement by role, replication of a channel's markers only to its
    subscribers, and an audit log of every creation, edit and deletion with time and callsign.
- The one part the built markers meet is the side discipline: no side's markers reach another.
- No open ticket covers the palette.

## Open work

- [T-1083 — Keep the marker client poll off dedicated servers](/.ai/tickets/T-1083.toml) (idea,
  no plan): the client loop starts only where a player sees the map, not on a headless server.
- [T-831 — Per-side marker authoring audit then explicit UI](/documentation_v2/tickets/specs/t831_per_side_markers_audit.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-831_plan.md)): the Mission Creator authors
  markers per side explicitly, which changes the rows each side receives.

## Decisions

- The server picks a player's side from its own slot assignment and sends only that side's rows:
  two sides may hold opposite orders at the same place, and filtering on the client would hand
  each side the other's plan.
- Markers are inserted locally as the game's static markers: the game's map UI draws them with no
  mod layout, and its marker sync never re-broadcasts them.
- The wire uses parallel arrays rather than a delimited string: an empty icon or label, which the
  schema allows, stays unambiguous.
