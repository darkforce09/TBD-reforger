# Zone registry

Turns the loaded [mission](/documentation/glossary/g_to_m.md#mission)'s `zones[]` into prepared
zones once per world and answers the in-bounds questions the play area, triggers and objectives ask.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/Registry/
├── TBD_ZoneCompiler.c  prepares one zone: shape, bounds and play-area rules with logged fallbacks
└── TBD_ZoneRegistry.c  the prepared zones, FindById, and the boundary and base-protection questions
```

## How it works

`TBD_ZoneRegistry.Build` reads `TBD_MissionLoader.GetZones`, hands each row to
`TBD_ZoneCompiler.Prepare` and logs one `built` summary (zone, circle, polygon, boundary and
base-protection counts); it returns false until zones are loaded, so callers keep waiting. Both
`TBD_PlayAreaComponent` and `TBD_ObjectiveRegistry` call it; only the first call after `Clear` does
work, and `TBD_PlayAreaComponent.OnDelete` calls `Clear`, since statics outlive a world.

`TBD_ZoneCompiler.Prepare` resolves the shape by content, not by null (`JsonLoadContext` allocates
both nested members): a polygon of at least 3 `[x, z]` pairs is flattened with its bounds, else a
circle with a positive radius; both at once take the polygon, and neither leaves the zone INERT.
`ResolveRules` starts from `graceSeconds` 30, `warnEverySeconds` 5 and `penalty` `warn`, logs every
out-of-range value and unknown penalty (which falls back to `warn`, never `kill`), logs a `kill`
zone, and binds `vehicleClasses` through `TBD_PlayAreaVehicleAxis`. Only `boundary` and
`base_protection` zones log their defects and a `zone` line; other types carry rules for other
subsystems.

The queries: `HasBoundaryFor` (any boundary applies to a side), `IsInsideBoundary` (inside any
applicable boundary, the union, or off the governing zone's vehicle-class axis),
`GoverningBoundary` (the strictest applicable boundary: highest `TBD_EZonePenalty`, then shortest
grace), `FindViolatedProtection` (another side's base-protection zone the player stands in) and
`FindById`.

## Authority

- Server: everything. The registry is built and asked only from the server-side play area,
  triggers and objective system; the classes carry `@authority server`.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Zone` beside this folder; `TBD_PlayAreaVehicleAxis` in `../PlayArea/`;
  `TBD_MissionLoader` and `TBD_MissionZoneRulesStruct` under
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/`; `TBD_Log`.
- Used by: `TBD_PlayAreaComponent`, `TBD_PlayAreaPenalties` and `TBD_PlayAreaVehicleAxis` in
  `../PlayArea/`; `TBD_TriggerCompiler` in `../Triggers/`; `TBD_ObjectiveRegistry` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`; `TBD_WinConditionEvaluator` in
  `mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/`; `TBD_DynamicSpawner` in
  `mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`.
- Rules: "no boundary applies" and "outside the boundary" stay separate verdicts; every zone leaves
  the compiler with resolved rules, never a sentinel; an unknown penalty never becomes `kill`;
  `cargo xtask mod compile` checks that the scripts compile.
