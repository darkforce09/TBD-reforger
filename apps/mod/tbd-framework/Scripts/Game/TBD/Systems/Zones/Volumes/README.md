# Zone volumes

Reads the objective half of each zone's `zoneRules` (height bounds, capture counts, advantage and
starting owner) and answers the objective system's questions about who is inside a zone's volume,
who captures, who contests and who holds.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/Volumes/
├── TBD_ZoneContestResolver.c  the acting side of a capture under attackerCount, defenderCount, advantagePercent
├── TBD_ZoneVolume.c           the objective-facing queries: height band, acting side, contest, hold, owner
└── TBD_ZoneVolumeBounds.c     the per-zone volume keys copied off the loaded mission, with their defaults
```

## How it works

`TBD_ObjectiveRegistry` calls `TBD_ZoneVolumeBounds.Clear` and `Read` when it builds; `Read`
copies `attackerCount`, `defenderCount`, `advantagePercent`, `minHeight`,
`maxHeight` and `startingOwner` off every loaded zone, keeping the loader's ABSENT sentinels, and logs
any zone whose `minHeight` is above its `maxHeight` (that volume contains nobody).

- Height: `ContainsAgl` measures height above the ground at the body's own XZ
  (`origin.y - GetSurfaceY(x, z)`), not above sea level and not at the zone centre; an absent bound
  is open. `ContainsOrigin` adds the zone's XZ shape.
- Counts: an absent `attackerCount` or `defenderCount` reads as 1 (anyone present); an authored 0
  stays 0 (no count gate, or nobody contests and nobody is needed to hold).
- Capture: `ResolveActingFaction` hands contestable objectives to
  `TBD_ZoneContestResolver.ResolveContestable` (every side meeting `attackerCount` may act, any
  meeting `defenderCount` contests) and the others to `ResolveByWeight` (the single largest side
  acts, and must pass `acting * 100 >= others * (100 + advantagePercent)` when the percent is
  authored). With no volume keys the answer equals presence-only capture.
- Hold: `EnemyContestsHold` and `HolderPresent` apply `defenderCount` to the enemy and the holder.
- Starting owner: `ApplyStartingOwner` starts a capture objective held (full progress) by a declared
  faction the objective may be owned by; anything else is logged and left neutral.

## Authority

- Server: everything. The objective system calls these classes on the server only; `TBD_ZoneVolume`
  carries `@authority server`.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Zone` beside this folder; `TBD_MissionLoader` and `TBD_MissionZoneRulesStruct`
  under `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/`; `TBD_Objective` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`; `TBD_DeclaredFactions` and
  `TBD_Log` under `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; `#/$defs/zoneRules` in
  `contracts/definitions/mission.schema.json`.
- Used by: `TBD_ObjectiveRegistry` and `TBD_ObjectivesComponent` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`; `TBD_EntityQuery` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/World/`.
- Rules: absent keys leave presence-only capture and holding unchanged; a contested or tied capture
  resolves to no side; `cargo xtask mod compile` checks that the scripts compile.
