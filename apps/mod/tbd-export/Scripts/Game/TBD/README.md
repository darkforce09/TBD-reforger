# Export addon game scripts

The export addon's [EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) that compiles into the game
rather than into [Workbench](/documentation_v2/glossary/n_to_z.md#workbench): the runtime road network
export, which runs inside a playing export world.

## Contents

```text
apps/mod/tbd-export/Scripts/Game/TBD/
└── Export/  the runtime road network exporter: a game mode component and its helpers
```

## How it works

The folder holds one subsystem, `Export/`. The export game mode prefab
carries `TBD_RoadExportComponent`; playing the export
[mission header](/documentation_v2/glossary/g_to_m.md#mission-header) starts it, and it writes the road
files to `$profile:TBD_Export/everon/roads/`.

## Authority

- Server: the road export, which returns on a client (`RplSession.Mode()` is `RplMode.Client`).
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: vanilla game classes only; nothing from `tbd-framework` or from the addon's
  Workbench scripts.
- Used by: the export game mode prefab in `apps/mod/tbd-export/Prefabs/Systems/`.
- Rules: game scripts here never call a Workbench class; `cargo xtask mod compile` compiles only the
  framework addon, so they compile only when Workbench or a game loads `tbd-export`.

## Related documentation

- [Map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  the runtime road export among the addon's exporters, and the files it shares with the
  Workbench road layer.
