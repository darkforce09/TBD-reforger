**Status:** live

# Template: decisions entry

**When to use:** one decision in a feature's `decisions.md` log, such as the Mission Creator's
`documentation_v2/website/frontend/apps/editor/decisions.md`: a choice that shapes the feature and
would otherwise be argued again. An entry says what was decided, why, what follows from it, and
which earlier entry it replaces. Live documents carry no dates, decisions entries excepted. The
[README standard](/documentation_v2/standards/readme_standard.md) holds the
writing rules the log shares with other documents.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. Each entry is a
`###` heading inside the log, with its four parts in this order.

````markdown
### <YYYY-MM-DD> — <the decision, as a short statement>

**Context:** <the problem, the constraints, and the options on the table when it was decided>

**Decision:** <what was decided, precisely enough to check the code against it>

**Consequences:** <what follows: what the code must do, what it rules out, the costs accepted, and
the gate or test that holds the decision in place>

**Supersedes:** <a link to the entry this one replaces, with its date and title; or "none">
````

The date is the day the decision was taken. When a decision changes, a new entry records the new
one and names the old entry under Supersedes, so the log shows which decision holds. Context
describes the situation at that date; Decision and Consequences describe the code as it stands, and
when the code no longer matches an entry, a new entry records what holds and supersedes it.

## Worked sample

Written from the Mission Creator log's entry for the Everon elevation raster, checked
against the exporter, the terrain manifest, the anchor file and the CI gate as they stand. The
sample sits in a fenced block, so no gate reads its paths.

````markdown
### 2026-06-29 — The terrain height map is sampled from the engine, not exported

**Context:** The Mission Creator needs the ground elevation under every placed entity, for its Z
value and for the hillshade layer. Workbench's own height-map export did not work on the packed
Everon terrain, so the elevation had to come from somewhere the exporter could reach.

**Decision:** The tbd-export Workbench plugin `TBD_MapExportDEM`
(`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/DEM/TBD_MapExportDEM.c`) samples
`WorldEditorAPI.GetTerrainSurfaceY` across the world grid and encodes each height as a linear
16-bit value between the terrain's lowest and highest points. Everon's committed result is a
6400 × 6400 16-bit PNG, `dem/everon-dem-16bit.png`, and its manifest records the source as
`mod-getsurfacey-resample` (`assets_v2/terrains/everon/manifest.json`).

**Consequences:** The raster agrees with what the engine reports at any point, so it is checked
against points probed in Workbench: `cargo xtask ci verify-terrain-strict` validates the manifest
and requires at least 10 anchors, each within the `thresholdM` of
`assets_v2/terrains/everon/anchors/verification.json` (1 m; the file holds 11). A re-export needs
Workbench with the tbd-export addon loaded. Sixteen bits over Everon's height range give a step of
about 9 mm.

**Supersedes:** none.
````
