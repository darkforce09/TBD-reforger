**Status:** live

# Play area warning

What a player is told when they leave the area of operations, or walk into another side's
protected base, while the round is live: how long they have to return and what happens if they do
not. Under one life a terminal penalty ends the player's [event](/documentation_v2/glossary/a_to_f.md#event),
so the warning must come early and repeat. The warning is private chat; the [mod](/documentation_v2/glossary/g_to_m.md#mod) draws no overlay.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/`](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/README.md)
  (`TBD_PlayAreaComponent.c`, the enforcer and every message; `TBD_ZoneRegistry.c`, the zones and
  their rules; `TBD_Zone.c`, the zone names).
- Entry: `TBD_PlayAreaComponent`'s 1 s server tick, a game mode component on
  `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et`.
- Layout: none; messages go to the player's chat through `SCR_ChatComponent.SendPrivateMessage`.
- Related features: the [safe start HUD](/documentation_v2/mod/tbd-framework/UI/safe_start_hud/safe_start_hud_specification.md),
  the phase before enforcement starts; the
  [spectator](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md), where a
  player killed by the penalty goes.

## Behaviour

### When it applies

1. Enforcement runs only while the stage is `LIVE`; leaving `LIVE` drops every running countdown.
2. A [mission](/documentation_v2/glossary/g_to_m.md#mission) with no `boundary` zone restricts nobody,
   and the server logs that once. A boundary may apply to one side or to all.
3. A `base_protection` zone names a faction; a player of any other side standing in it is in
   violation. The boundary is checked first, so its message wins when a player breaks both.
4. A player whose life is spent, or whose body is dead, is never policed; with no
   `TBD_SpawnManager` on the world, enforcement stands down for everyone.

### The countdown

1. On the first tick outside, the player reads "TBD: you are outside the play area (<zone>) --
   return within <n>s." ("inside a protected area" for a base-protection zone), with "ONE LIFE:
   you will be killed and cannot respawn." added when the zone's penalty is `kill`. The zone is
   its `label`, else its id, else its type.
2. The message repeats every `warnEverySeconds` (default 5 s) with the time left. The grace is the
   zone's `graceSeconds` (default 30 s, at most 3600 s) and starts counting on the tick after
   detection, so a player gets up to one extra second.
3. Stepping back inside ends the countdown and says "TBD: back inside the play area.". Moving into
   a different violated zone, or a new body under the same player id, restarts it.
4. When the grace runs out, the zone's `penalty` decides, once per violation:
   - `none`: nothing is said at any point, and the expiry is only logged;
   - `warn`, the default: "TBD: you are still out of the play area. Return to the AO.", then
     silence until the player returns;
   - `kill`: "TBD: you left the play area. Your one life is spent -- contact an admin.", then the
     engine's own kill through the body's damage manager; the player becomes a spectator, and only
     an admin's `#tbd respawn` brings them back.

## Data

- The zones come from the mission document's `zones[]` with `type` `boundary` or
  `base_protection` and `rules.penalty`, `rules.graceSeconds` and `rules.warnEverySeconds`, read on
  the server by `TBD_ZoneRegistry`; clients hold no mission document, so the check cannot run
  there.
- The player's side comes from the [slot](/documentation_v2/glossary/n_to_z.md#slot) `TBD_SpawnManager` assigned.
- Per-player state (`TBD_PlayAreaViolation`) exists only while the player is in violation, keyed
  by player id and checked against the body, so a recycled id cannot inherit a countdown.

## Design

- As built: private chat lines, one on detection and one every `warnEverySeconds`, and one on
  return or expiry.
- Design target: the specification's wireframe; the folder has no mockup set. It draws, and the
  built warning lacks all of it:
  - a screen-edge vignette, amber during the grace and flashing red under 5 s, leaving the centre
    clear;
  - a banner "RETURN TO MISSION AREA" with the zone ("OUT OF BOUNDS: <zone>") in the upper centre;
  - a large seconds countdown and a bar draining from full to empty;
  - the consequence line "ONE LIFE: PERMANENT ELIMINATION IN <n>s";
  - removal on the frame the player re-enters; the built check runs once a second.
- No open ticket covers these differences.

## Open work

None. Checked `.ai/tickets/` for open tickets on the play area, its boundary and its warning.

## Decisions

- The default penalty is `warn`, and `kill` is an explicit per-zone choice: under one life a kill
  for leaving the area removes a player for what is often a navigation mistake, so a hard boundary
  is the mission maker's decision, recorded in the log at load.
- No enforcement outside `LIVE`: during safe start players are still walking to their start line,
  and a countdown then would be a trap.
- One server tick walks every player rather than a timer per player: a timer cannot be cancelled
  for one player alone, and one carrying a player id would outlive a disconnect onto a recycled id.
