# Mission environment and placement readers

Applies a loaded [mission](/documentation_v2/glossary.md#mission)'s authored environment to the
world: fog, wind and view distance at load, the weather timeline while the round is live, and the
placement scatter that jitters slot spawn positions.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Ingestion/
├── TBD_EnvironmentReader.c  environment fog, wind, wind direction and view distance, applied once at load
├── TBD_PlacementScatter.c   deterministic slot and group spawn offsets from placementRadius and placementShape
└── TBD_WeatherRuntime.c     weatherTimeline keyframes, each forced at its minute of the live round
```

## How it works

- `TBD_EnvironmentReader.Apply()` runs from `TBD_MissionLoader` after a valid parse. It reads
  `TBD_MissionEnvironmentStruct`, the document's `environment` block, and applies `fog`, `wind` and
  `windDirDeg` through the `BaseWeatherManagerEntity` overrides and `viewDistance` through
  `ChimeraGame.SetViewDistance`. Each number starts at the `ABSENT` sentinel (-1e6), since 0 is a
  legal fog, wind or direction, and an absent key leaves the world default. `dateTime` and
  `weatherPreset` are bound but not applied.
- `TBD_PlacementScatter.ForSlot` runs from `TBD_SpawnManager` for every slot body it spawns. A
  second `JsonLoadContext` pass reads `placementRadius` and `placementShape` of `slots[]` and of
  `orbat.*.groups[]`, once per mission id. `Scatter(center, radius, shape, seed)` is deterministic:
  radius 0 or absent returns the authored point, `square` spreads over the axis-aligned square, and
  anything else over a disk. A group offset is shared by all its members (one seed per faction and
  callsign), and a slot's own offset adds to it; the seed of a slot comes from its key. The offset
  is horizontal: the spawn manager still decides height.
- `TBD_WeatherRuntime` adds a `modded class SCR_BaseGameMode` whose `OnGameStart` arms a one-second
  self-re-arming tick in a framework world. The tick reads `weatherTimeline.keyframes[]` once per
  mission id, and while the stage is `LIVE` it applies each keyframe whose `atMinutes` has passed
  since the round went live, once: `TimeAndWeatherManagerEntity.ForceWeatherTo` with the preset,
  looping so it holds until the next keyframe, plus an optional `fog` and `windDirDeg` through the
  same overrides `TBD_EnvironmentReader` uses. Every transition logs a `[TBD][Weather]` line.

## Authority

- Server: everything. `TBD_EnvironmentReader` runs on the server load path, which
  `TBD_FrameworkManager.OnPostInit` enters only when `RplSession.Mode()` is not `RplMode.Client`;
  `TBD_WeatherRuntime`'s tick is armed only off the client (`@authority server` on `OnGameStart`);
  `TBD_PlacementScatter` runs inside the server's spawn.
- Client: nothing; the engine replicates weather to clients.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`
  (the raw JSON and the mission id); `TBD_FrameworkManager` (the game stage and the framework-world
  test); `TBD_Log`; the engine's `BaseWeatherManagerEntity`, `TimeAndWeatherManagerEntity` and
  `ChimeraGame`; the `environment`, `weatherTimeline`, slot and group definitions in
  `contracts_v2/definitions/mission.schema.json`.
- Used by: `TBD_MissionLoader`, which calls `TBD_EnvironmentReader.Apply` and binds
  `TBD_MissionEnvironmentStruct` as the document's `environment`; `TBD_SpawnManager` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`, which calls
  `TBD_PlacementScatter.ForSlot`; the game mode, through the modded `SCR_BaseGameMode`.
- Rules: numbers that may be authored as 0 keep the `ABSENT` sentinel; readers declare their own
  wire structs instead of adding fields to the loader's structs; scatter stays deterministic, so a
  slot spawns at the same offset on every respawn; sources stay ASCII, and
  `cargo xtask mod compile` checks that they compile, while whether fog, weather or scatter shows
  in a round is checked by hand.
