**Status:** live

# Objective capture HUD

The objective board a player sees during live play: a panel in the top-left corner that lists each
objective of the [mission](/documentation_v2/glossary/g_to_m.md#mission) from the player's side, and a
capture bar while the player stands in a capture zone. The server composes every word; the client
only paints it.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/`](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/README.md)
  (`TBD_ObjectiveHud.c`, the panel and its RPC pair) and
  [`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/README.md)
  (`TBD_ObjectivesComponent.c`, which ticks the objectives and builds each player's board;
  `TBD_Objective.c`, the status texts).
- Layout: [`apps/mod/tbd-framework/UI/layouts/Hud/`](/apps/mod/tbd-framework/UI/layouts/Hud/README.md),
  `TBD_ObjectiveHud.layout`, whose README gives the geometry and every widget the handler binds.
- Entry: `TBD_ObjectivesComponent.Deliver`, run by the component's 1 s server tick while the round
  is `LIVE`.
- Related features: the [end screen](/documentation_v2/mod/tbd-framework/UI/end_screen/end_screen_specification.md),
  which an objective end trigger opens; the task markers on the map, which `TBD_TaskHud` in the
  same folder draws.

## Behaviour

### What the board shows

1. The panel is titled "OBJECTIVES". Each objective is one row, `[<glyph>] <title>`, with a detail
   line, both resolved for the viewer's faction: the title and the task text follow the
   objective's per-side framing (attacker or defender) where the mission authors one.
2. The glyph: `o` neutral, `+` held by the viewer's side, `-` held by another side, `!`
   contested, `v` complete, `#` a destroy target, `H` a hold, `.` inactive. A `!` row takes the
   danger state, `+` and `v` the active state, `-` the taken state.
3. The detail line for a capture objective: "neutral", "OURS" or "held by <faction key>", then
   " -- CONTESTED" while contested, or " -- <n>% <faction key>" while a capture is part done. A
   destroy objective reads "intact <destroyed>/<required>", then "DESTROYED"; a hold reads
   "hold <n>s left", with " (PAUSED)" while paused, then "HELD"; an inactive objective reads
   "inactive". The side's task text follows after " | ".
4. The capture bar appears only while the player stands alive in a capture objective. It shows
   `<title>  <percent>%` and a fill scaled to the percentage; when the player stands in more than
   one, a contested one wins.

### Capture rules the board reflects

1. A zone is contested when players of more than one side stand in it alive; a contest freezes
   progress in both directions.
2. Taking a zone another side holds is two stages: neutralise it, then capture it; progress decays
   when nobody acts on it.
3. The completion of an objective also goes to every player as one chat line: "TBD: <name> has
   been CAPTURED by <faction>.", "… has been DESTROYED." or "… has been HELD to the clock by
   <faction>.". Progress and contests are never chatted.

### Delivery

1. Every second the server builds each connected player's board and sends it only when it differs
   from the last board that player was sent (`ReplicateHud` compares a length-prefixed signature).
   A player the server has not yet sent a board to, a new connection included, gets the whole
   board.
2. The first board opens the panel; a repaint happens only when the board's signature changes.
3. When the round leaves `LIVE`, the server sends every player an empty board with `show` 0, which
   closes the panel and resets what each player is known to hold.

### Known discrepancies

- The server offers a pull for a client that wants its board (`TBD_ObjectivesComponent.PushHudTo`,
  `SCR_PlayerController.TBD_RequestObjectiveHud` in `TBD_ObjectiveHud.c`) — but no script calls
  the request, so a client only ever receives pushed boards.

## Data

The HUD makes no HTTP call. Its wire, on the modded `SCR_PlayerController` (the HUD README's
[Authority](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/README.md#authority) lists both RPCs):

- `TBD_RpcDo_ObjectiveHud`, Reliable, Owner: three string arrays (glyphs, titles, details), the
  bar's label, percentage and visibility, and `show`. The server fills it from
  `TBD_ObjectiveRegistry`'s objectives and the viewer's faction, resolved from the [slot](/documentation_v2/glossary/n_to_z.md#slot)
  `TBD_SpawnManager` assigned; a client holds no mission document and computes nothing.
- On a listen host, the host's own board is applied in place without an RPC.

## Design

- As built: a 360 x 320 px glass panel (`SURFACE_GLASS`) 24 px from the top-left corner with the
  title, a `TBD_ListBox` of rows and a capture bar: a `SURFACE_CONTAINER_HIGH` track and an
  `ACTION` fill 328 px wide at 100 %. Texts draw in the engine's default font. The
  [layouts README](/apps/mod/tbd-framework/UI/layouts/Hud/README.md) gives the geometry.
- Design target: the specification's wireframe; the folder has no mockup set. It matches the built
  panel's position, title, `[glyph]` rows, status texts and `<zone>  <percent>%` bar label.
- Differences from the target:
  - the target's glyph set has no `v` complete and no `.` inactive row;
  - the target's contested bar turns amber or red and flashes, with a "[!] CONTESTED" tag beside
    it; the built bar keeps its fill colour and only the row marks the contest;
  - the target's panel dims while the player aims down sights; the built panel does not.
- No open ticket covers these differences.

## Open work

- [T-946.55 — Objective HUD replicates to every player at 1 Hz](/documentation_v2/tickets/specs/t936_mission_logic.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-946_55_plan.md)): sends a board only when it
  changes; `ReplicateHud` already does, so the ticket's status is behind the code.
- [T-212 — Typed per-side objectives with attributes](/documentation_v2/tickets/specs/t212_typed_objectives.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-212_plan.md)): objectives become typed, placed,
  per-side entities, which changes the titles and task texts each side's board carries.
- [T-1089 — Remove or wire uncalled mod script methods and components](/.ai/tickets/T-1089.toml)
  (idea, no plan): wires or removes `TBD_RequestObjectiveHud`, the uncalled pull.

## Decisions

- The board is composed on the server for each player: clients hold no mission document, and each
  side sees its own framing and ownership words.
- The board replaced a per-tick private chat pump: chat keeps only the completion lines, so the
  log is not buried in progress messages.
- A board is sent only on a difference, and a miss always sends: an idle round costs one board per
  player, and no player is starved of a real change.
