# Group AI defaults

Applies each group's authored combat mode, formation and speed to its live AI group once the round
goes live. It runs on the server once a second.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/AI/GroupState/
├── TBD_GroupState.c            TBD_GroupState: the group-state pass and applying the defaults
└── TBD_GroupStateWireStruct.c  the orbat.*.groups[] AI attribute wire structs of the pass
```

## How it works

`TBD_GroupState.Tick` parses each group's `combatMode`, `behaviour`, `formation` and `speedMode`
once per mission id into `TBD_GroupStateDocStruct` and keeps the groups that author any of them.
Once the stage is `LIVE` and the slot bodies exist, it finds each group's live `SCR_AIGroup`
through its slot bodies' AI agents and applies the defaults once:

| Attribute | Applied as |
|---|---|
| `combatMode` | blue and green hold fire, white returns fire, yellow and red fire at will (`SCR_AIGroupUtilityComponent.SetCombatMode`) |
| `formation` | the nearest of Wedge (wedge, vee, diamond), Line (line, echelon left and right), Column (column, file) and StaggeredColumn (`AIFormationComponent`) |
| `speedMode`, else `behaviour` | a movement speed setting with the `DEFAULT` origin, so a waypoint's own `WAYPOINT` setting wins |

An unknown token, or a group without the component, is one WARNING and keeps the engine default.

## Authority

- Server: everything; `Tick` runs from `TBD_RuntimeHeartbeat`'s server tick.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionJsonPass`, `TBD_MissionLoader` (mission id, slots),
  `TBD_FrameworkManager`, `TBD_SpawnManager`, `TBD_AIWireEnums`, `TBD_Log`, `TBD_AnnounceOnce`;
  `$defs/group` in `contracts/definitions/mission.schema.json`; the groups
  [`TBD_WaypointRuntime`](../Waypoints/README.md) arms.
- Used by: `TBD_RuntimeHeartbeat` (`Clear`, `Tick`, `TICK_MS`).
- Rules: nothing here spawns groups, enables AI or rewrites waypoints; absent attributes leave the
  engine default; `cargo xtask mod compile` checks that the scripts compile, and a group holding
  fire or walking in wedge is checked by hand on a dedicated server.
