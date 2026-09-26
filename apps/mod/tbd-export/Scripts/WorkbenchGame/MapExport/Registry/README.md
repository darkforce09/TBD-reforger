# Prefab and weapon catalog export

Two catalog layers of the map export: a taxonomy of every prefab placed in the world open in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench), with its size and cover class, and a fixed
list of weapons and magazines. Neither is the item [registry](/documentation_v2/glossary/n_to_z.md#registry)
the platform imports, which another Workbench plugin of the addon writes.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Registry/
├── TBD_ArsenalExportPlugin.c  the standalone plugin that runs the weapon list export
├── TBD_MapExportArsenal.c     `TBD_MapExportArsenal`: writes the coded weapon and magazine list
├── TBD_MapExportPrefabs.c     `TBD_MapExportPrefabs`: one taxonomy record per placed prefab
└── TBD_PrefabsExportPlugin.c  the standalone plugin that runs the prefab taxonomy export
```

## How it works

Both exporters take the shared `TBD_MapExportContext` and `TBD_MapExportConfig` from
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/` and write into
`$profile:TBD_Export/<map>/registry/`, where `<map>` is the context's map name.

`TBD_MapExportPrefabs` sweeps the world in square cells of `m_fObjectChunkSizeM` (512 m by default)
and keeps the first entity of every distinct prefab resource name. Each record holds
`resourceName`, `className`, the local box as `lengthM`, `widthM` and `heightM` (each at least
0.5 m) with `footprintAreaM2`, and a classification decided by substrings of the class and prefab
names:

| `kind` | `class` values | `coverType` | `destructible`, `maxHealth` |
|---|---|---|---|
| `building` | `residential`, `military`, `commercial`, `industrial`, `civic`, `shed`, `generic` | `hard` | true, 1000 |
| `tree` | `conifer`, `deciduous`, `generic` | `foliage` | false, 1000 |
| `rock` | `cliff`, `boulder` | `hard` | false, 1000 |
| `prop` | `fence` or `generic` | `hard` | false, 1000 |
| `utility` | `powerline` | `soft` | false, 1000 |
| `vehicle` | `car` | `hard` | true, 2000 |

It writes `prefabs.json`, an array of those records, and `prefabs_meta.json` (`method`
`mod-prefabs-component-export`, `uniquePrefabCount`, `worldSizeM`, `dataFile`); a failed write
deletes `prefabs.json`. The health and destructibility values come from that table, not from the
prefabs' components.

`TBD_MapExportArsenal` reads nothing from the world: `PopulateStandardArsenal` codes six weapons
(M16A2, AK-74, M249, PKM, M14, SVD), each with a caliber, a mass, a prefab resource name and the ids
of compatible magazines and attachments, and eight magazines with capacity, caliber and mass. It
writes them to `arsenal.json` (`weapons` and `magazines`) with `arsenal_meta.json` (`method`
`mod-arsenal-registry-export`, the two counts, `dataFile`).

`TBD_PrefabsExportPlugin` and `TBD_ArsenalExportPlugin` each build their own config and context
and run one exporter; `Configure` opens the config dialog. Their `[WorkbenchPluginAttribute]`s are
commented out, so Workbench lists them in no menu. `TBD_MapExportPlugin` in
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` runs both, behind `m_bExportPrefabs`
and `m_bExportArsenal`, and its attribute is commented out too.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `BaseWorld.QueryEntitiesByAABB` and `FileIO`.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  No committed tool reads `prefabs.json` or `arsenal.json`: the platform's item catalog is
  `contracts_v2/catalogs/`, and the map's prefab catalogue comes from the full world-object export
  and `world build-objects`.
- Rules: the prefab taxonomy keys on prefab and class names only, and keeps one record per resource
  name; lines added stay ASCII. `cargo xtask mod compile` compiles only the framework addon, so
  these scripts compile only when Workbench loads `tbd-export`.

## Related documentation

- [Contract catalogs](/contracts_v2/catalogs/README.md) — the item registry the platform imports,
  and the plugin that exports it.
