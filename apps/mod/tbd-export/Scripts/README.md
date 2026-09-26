# Export addon scripts

The export addon's [EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript), in the two modules
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) compiles separately: the game module, which
holds the runtime road network export, and the
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) module, which holds every exporter that
runs inside the editor.

## Contents

```text
apps/mod/tbd-export/Scripts/
├── Game/           the game module: the runtime road network exporter
└── WorkbenchGame/  the Workbench module: map, equipment and vehicle, and item registry exporters
```

## How it works

Enfusion compiles `Game/` into the game, a server and Workbench's play mode, and `WorkbenchGame/`
into Workbench alone. The road network exporter is the one export that needs a running world: the
export game mode prefab carries its component, and it writes the road files once the export
[mission header](/documentation_v2/glossary/g_to_m.md#mission-header) is playing. Every other exporter
reads the loaded world, prefabs and configs from inside the editor and runs from a Workbench menu
entry or a Net API call. Both modules write to the Workbench profile, mostly under
`$profile:TBD_Export/`.

## Authority

- Server: the road export in `Game/`; it returns without running on a client.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.
- Workbench: everything in `WorkbenchGame/`, which runs in the editor only.

## Boundaries

- Depends on: vanilla game and Workbench classes only; nothing from `tbd-framework`, which the
  addon does not load (`apps/mod/tbd-export/addon.gproj`).
- Used by: the export game mode prefab in `apps/mod/tbd-export/Prefabs/Systems/`, which carries the
  road export component; people and tools through the Workbench menu and the Net API, as the
  module READMEs list.
- Rules: nothing in `Game/` names a class from `WorkbenchGame/`, which a game never compiles; the
  addon holds no copy of a framework class. `cargo xtask mod compile` compiles the framework addon
  alone (`tools_v2/xtask/src/commands/mod_ops/compile/execution.rs`), so these scripts compile only
  when Workbench or a game loads `tbd-export`.

## Related documentation

- [Export addon script documentation](/documentation_v2/mod/tbd-export/Scripts/README.md) — the
  deeper documents of these scripts.
- [Map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  every map layer and the runtime road export.
