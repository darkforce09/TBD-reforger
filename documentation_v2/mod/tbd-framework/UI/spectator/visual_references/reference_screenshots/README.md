**Status:** live

# Spectator reference screenshots

Design-phase reference for the [spectator](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md):
two Arma 3 captures of a community framework's spectator, with the ACE3 medical and RHS weapon
mods, the one-life spectating the TBD spectator started from. They are not an implementation
source.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/spectator/visual_references/reference_screenshots/
├── spectator_death_and_kill_info.png  a dead player's forensics, the killer's beacon, the kills
└── spectator_mines_playercount.png    the bottom rail, a vehicle tag and three mine markers
```

## How it works

Both captures show four layers over the world: the 3D view; billboards projected from world
positions (nameplates in faction colours, vehicle tags, a killer beacon, mine markers); a
forensics stream at the centre; and a bottom rail.

`spectator_death_and_kill_info.png`, watching "[LG]Yuki_Hattori" (green side) from a high free
camera over farmland, with smoke from destroyed armour on the left:

- the centre stream, headed "(ACE3):" in amber: "Total Kills: 3", then the life in order:
  "Wounded: ace_frag_huge from [HF]" (heavy fragmentation), three kills ("gHosT", "[L-13] Kolin",
  "[L-13]ProrocK", each "[FatalInjury:Death]"), three "Wounded: rhs_B_545x39_7N22_Ball from
  [XOF]Ales" hits and one "ace_frag_small HD from [HE]", and "Killer: [XOF]Ales -
  [FatalInjury:Death]";
- a beacon over the killer, "killer: [1xDB]" above "[XOF]Ales", showing where the shot came from;
- a victims billboard, "Kills: 3" over the three names;
- the bottom rail: "[LG]Yuki_Hattori", the clock `23:32`, living counts `34` and `28`, and "Kills
  to win:" `20` and `26`.

`spectator_mines_playercount.png`, watching "[L - 13] Vikhr" (red side) from above a road:

- a vehicle tag "[1] [FOX]Beast" in blue over a moving truck: the occupant count, the tag and the
  driver;
- three red triangles on the road and its verges, buried mines the living players cannot see;
- the bottom rail: the watched player on a dark red pill, the clock `35:39`, living counts `31`
  (blue) and `16` (red), and "Kills to win:" `11` (blue) and `26` (red).

A reading of the rail, not stated in the captures: in the second capture each side's kills to win
equals the enemy's living count minus five (16 - 11, 31 - 26), as though a side reduced to five
players loses. Other rail elements read as a match clock that counts up or down, and a spacer
before the kills-to-win block.

The TBD spectator keeps the free, follow and first-person camera and the idea of a roster, and
draws none of these overlays; the
[spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md)
lists every difference.

## Code

- [Spectator](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/) — the spectator the
  captures informed.

## Boundaries

- Depends on: nothing.
- Used by: the spectator specification's Design section and the visual references README.
- Rules: captures are kept as taken; no screenshot of the built UI belongs here.
