# Export addon game module

The export addon's game script module: the [EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript)
that the engine compiles into the game, a server and
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench)'s play mode. It holds the runtime road network
export, the one export that needs a running world.

## Contents

```text
apps/mod/tbd-export/Scripts/Game/
└── TBD/  the TBD game scripts: the runtime road network exporter
```

## How it works

Enfusion compiles each addon's `Scripts/Game/` into the game module. For this addon that is the
road export component in `TBD/Export/`, which the export game mode carries and which writes the road
files once the export [mission header](/documentation_v2/glossary/g_to_m.md#mission-header) is playing.
The addon's other exports live in `Scripts/WorkbenchGame/`, compiled into Workbench only.

## Authority

- Server: the road export; it returns without running on a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: vanilla game classes only.
- Used by: the export game mode prefab in `apps/mod/tbd-export/Prefabs/Systems/`.
- Rules: no class here depends on `Scripts/WorkbenchGame/`, which a game never compiles;
  `cargo xtask mod compile` compiles only the framework addon
  (`tools_v2/xtask/src/commands/mod_ops/compile/execution.rs`), so these scripts compile only when
  Workbench or a game loads `tbd-export`.

## Related documentation

- [Map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  the runtime road export among the addon's exporters, and the files it shares with the
  Workbench road layer.
