# Location export

The place layers of the map export: the named towns and villages of the world open in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench), read from its location compositions, and the
landmark anchors in `Anchors/`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Locations/
├── Anchors/                     landmark structures with ground and apex heights, to `anchors/`
├── TBD_LocationsExportPlugin.c  the standalone plugin that runs the named location export
└── TBD_MapExportLocations.c     `TBD_MapExportLocations`: named places, written to `locations.json`
```

## How it works

`TBD_MapExportLocations.Export` takes the shared context and config from
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`, queries the whole world in one box
(−1,000 m to 4,000 m in height), and keeps each entity whose prefab lies under
`Prefabs/World/Locations/` but not under its `Urban/`, `Natural/` or `Aquatic/` folders, plus
`StPhilippe_StPhilippe_01.et`. The prefab's file name becomes the display name: underscores become
spaces, and four names are spelled out by hand (`EntreDeux` is "Entre Deux", `Le_Moule` is
"Le Moule", `Villeneuf` is "Villeneuve", `StPhilippe_StPhilippe_01` is "Saint Philippe"). The id is
the lowercased name with spaces as hyphens, and the importance comes from a table of nine Everon
towns (Montignac 0.85 down to Highstone 0.48), 0.55 for any other.

It writes `$profile:TBD_Export/<map>/locations/locations.json`, an array of
`{id, name, x, y, importance}` rows in which `x` and `y` are the entity's world x and z in metres,
and `locations_meta.json` (`written`, `source` `TBD_MapExportLocations`, `worldSizeM`).

`TBD_LocationsExportPlugin` builds its own config and context and runs the export; `Configure`
opens the config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench lists it
in no menu.
`TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/` runs the
location export when `m_bExportLocations` is set and the anchors when `m_bExportAnchors` is, and its
attribute is commented out too.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig`, `TBD_MapExportPaths` and
  `TBD_MapExportJson` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`; the engine's
  `BaseWorld.QueryEntitiesByAABB`.
- Used by: `TBD_MapExportPlugin` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Plugins/`.
  No committed tool reads the file: the committed `assets_v2/terrains/everon/locations.json` comes
  from `map export-locations` in `tools_v2/developer-tools/src/map_raster_pipeline/map_labels/`,
  which reads the staged full world-object export, not this output.
- Rules: a place's name and importance are decided in the script, keyed on Everon's prefab names;
  lines added stay ASCII. `cargo xtask mod compile` compiles only the framework addon, so these
  scripts compile only when Workbench loads `tbd-export`.

## Related documentation

- [Everon terrain dataset](/assets_v2/terrains/everon/README.md) — the committed `locations.json`
  and the tools that build it.
