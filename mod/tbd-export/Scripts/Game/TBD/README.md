# Export addon game scripts

The export addon's [EnfScript](/documentation/glossary/a_to_f.md#enfscript) that compiles into the game
rather than into [Workbench](/documentation/glossary/n_to_z.md#workbench): the runtime road network
export and the ballistics oracle's simulation run, which run inside a playing export world.

## Contents

```text
mod/tbd-export/Scripts/Game/TBD/
└── Export/  the runtime road network exporter and the ballistics oracle simulation run: game mode components and helpers
```

## How it works

The folder holds one subsystem, `Export/`. The export game mode prefab
carries `TBD_RoadExportComponent`; playing the export
[mission header](/documentation/glossary/g_to_m.md#mission-header) starts it, and it writes the road
files to `$profile:TBD_Export/everon/roads/`. It also carries
`TBD_BallisticsOracleSimulationComponent`, which samples the engine's projectile simulation into
`$profile:TBD_BallisticsOracle/<generation id>/` when the Workbench ballistics oracle has recorded
a generation.

## Authority

- Server: the road export and the ballistics oracle simulation run, which return on a client
  (`RplSession.Mode()` is `RplMode.Client`).
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: vanilla game classes only; nothing from `tbd-framework` or from the addon's
  Workbench scripts.
- Used by: the export game mode prefab in `mod/tbd-export/Prefabs/Systems/`.
- Rules: game scripts here never call a Workbench class; `cargo xtask mod compile` compiles only the
  framework addon, so they compile only when Workbench or a game loads `tbd-export`.

## Related documentation

- [Map export](/documentation/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  the runtime road export among the addon's exporters, and the files it shares with the
  Workbench road layer.
