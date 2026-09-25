**Status:** live

# Briefing reference screenshots

Design-phase reference for the [briefing](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md):
sixteen Arma 3 captures of a community framework's briefing (mission `wog_180_new_dawn_14`, in
Russian with English readings below), the pre-mission orientation the TBD briefing started from.
They are not an implementation source.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/reference_screenshots/
├── briefing_conditions_rules.png                     "Условия": win conditions and restrictions
├── briefing_enemy_assets.png                         "Enemy vehicles": 15 types, 22 units
├── briefing_friendly_assets.png                      "Vehicles": the side's vehicles and counts
├── briefing_friendly_squad_list.png                  "Squads": the side's ORBAT with trait tags
├── briefing_lore.png                                 "Вводная": the situation narrative
├── briefing_marker_log.png                           "Markers log": who placed which marker
├── briefing_menu.png                                 the shell: system tabs and the topic tree
├── briefing_mission_parameters.png                   "Mission parameters": global settings
├── briefing_personal_squad_roster_and_inventory.png  "My Squad": each member's loadout
├── briefing_player_list.png                          "Players": roster and network diagnostics
├── briefing_squad_frequencies.png                    "Frequencies": the radio plan
├── briefing_task_parameters.png                      "Tasks parameters": capture rules
├── briefing_tasks.png                                "Задачи": the tasks with map links
├── briefing_uniforms.png                             "Атака": the attackers' uniforms
├── briefing_uniforms_2.png                           "Оборона": the defenders' uniforms
└── briefing_vehicle_inventory.png                    "Vehicle Inventory": cargo per vehicle
```

## How it works

The captures show a three-tier overlay on the 2D map, under a header with a back chevron and the
mission id:

1. System tabs: Map, Briefing, Players, SWT Markers, CBA (addon settings) and Radio (the radio
   addon's dialog).
2. A topic tree in three groups: the two sides' uniforms; tasks, conditions and lore; then
   frequencies, vehicles, vehicle inventory, enemy vehicles, squads, the player's own squad, task
   parameters and mission parameters.
3. The chosen topic's page, stamped with the in-game date and time.

What the pages carry:

- Tasks: "capture two zones", each zone a link that pans the map to it, and a 60-minute limit.
  Conditions: attackers win by taking both zones and defenders by holding one; only engineers and
  crew man armour; no fortifications within 50 m of a flagpole; capture opens 5 minutes after the
  start; a field hospital deploys no closer than 300 m; 3 minutes of freeze time.
- Uniforms: full-length renders per faction with the camouflage, headgear and webbing that tell
  the sides apart.
- Frequencies: the long-range command net (76.2 MHz) and one short-range net per squad, each with
  three auxiliary channels, commanders in amber and squads in blue.
- Vehicles and enemy vehicles: silhouette, designation and count (`BTR-80- 4`, `T-72B- 1`),
  covering trucks, armour, a helicopter, static weapons and artillery; vehicle inventory: cargo
  items with counts per vehicle.
- Squads: platoons and squads with each seat's holder or `[AI]` and trait tags (`| Med`,
  `| Eng`); my squad: each member's weapon, magazines, items and wearables.
- Task parameters: capture radius, minimum attackers, the defender block and the attacker ratio,
  hold time, and permanent capture. Mission parameters: view distances, preparation time,
  duration, a casualty threshold, thermals and night vision off, weather.
- Players: each connected player with a faction pip, and ping, bandwidth and desync readings;
  markers log: time, channel, author and text of each placed marker.

The built briefing keeps the map-first overlay, the nested navigation, the map links, the radio
plan, the vehicle manifests, the uniforms, the ORBAT and the parameters; the
[briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
lists what it replaces and drops.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the
  screen the captures informed.

## Boundaries

- Depends on: nothing.
- Used by: the briefing specification's Design section and the visual references README.
- Rules: captures are kept as taken; no screenshot of the built UI belongs here.
