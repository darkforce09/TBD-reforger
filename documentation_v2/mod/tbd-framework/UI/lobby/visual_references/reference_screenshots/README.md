**Status:** live

# Lobby reference screenshot

Design-phase reference for the [lobby](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md):
an Arma 3 capture of the "ROLE ASSIGNMENT" screen, the community's slotting experience the TBD
lobby started from. It is not an implementation source.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/lobby/visual_references/reference_screenshots/
└── lobby_screen.png  the Arma 3 role assignment screen, BLUFOR selected, one player connected
```

## How it works

The capture shows the mission `wog_187_chollima_on_the_wing_10` on Chernarus Autumn, 187 slots
(BLUFOR 92, OPFOR 95), over a dark map:

- an amber header rail with "ROLE ASSIGNMENT" and the host's name ("Mission Maker"), then a strip
  with the mission file name, the map, a bilingual match-up description and "Listed Players: 1";
- a side rail ("Side:") with a button per side and its claimed-of-total count (`0/92 Blufor`,
  `0/95 Opfor`);
- "Roles for BLUFOR:", a scrolling list of squads named by callsign and vehicle
  ("A1-1 Company HQ - M113A3 MEV", "A1-2 M60A1"), each seat showing its role, its occupant (`AI`
  in red until a player claims it), an AI chip and a claim icon that is struck through on seats
  locked until the unit leader is claimed; `Driver | Eng` marks a trait in the role name;
- a `DISABLE AI` button that stops bots filling unclaimed seats;
- a "Players:" table sorted by ping, with a voice indicator and the host row
  "Mission Maker (Host)";
- a footer with `BACK`, `LOCK` (the host freezes slot changes) and `OK` (the host moves everyone
  to the briefing).

The built lobby keeps the side → squad → seat drill-down, the counts, the holder on a claimed
seat and a lock action; the
[lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md)
lists what it drops.

## Code

- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the screen
  the capture informed.

## Boundaries

- Depends on: nothing.
- Used by: the lobby specification's Design section and the visual references README.
- Rules: captures are kept as taken; no screenshot of the built UI belongs here.
