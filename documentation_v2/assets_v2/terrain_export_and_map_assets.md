**Status:** live

# Terrain export and map assets

How a game terrain becomes the map the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)
draws: the [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) exports, the developer-tools
pipelines that turn them into committed datasets under `assets_v2/terrains/<terrain>/`, the gates
that prove them, and how the [API](/documentation_v2/glossary/a_to_f.md#api) serves them. Developers and
AI agents read it before exporting a terrain again or adding a new one.

## Where it lives

- Code: the Workbench export plugins in
  [`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/`](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/README.md);
  the world export pipeline in
  [`tools_v2/developer-tools/src/world_export_pipeline/`](/tools_v2/developer-tools/src/world_export_pipeline/README.md),
  the raster pipeline in
  [`tools_v2/developer-tools/src/map_raster_pipeline/`](/tools_v2/developer-tools/src/map_raster_pipeline/README.md),
  the blueprint compiler in
  [`tools_v2/developer-tools/src/blueprint/`](/tools_v2/developer-tools/src/blueprint/README.md)
  and the gates in
  [`tools_v2/developer-tools/src/map_verification/`](/tools_v2/developer-tools/src/map_verification/README.md);
  the datasets in [`assets_v2/terrains/`](/assets_v2/terrains/README.md) and the glyph set in
  [`assets_v2/glyphs/`](/assets_v2/glyphs/README.md).
- Entry: `cargo xtask map export-terrain <terrain> [--phase <phase>]`
  ([map command group](/tools_v2/xtask/src/commands/map/README.md)); the `world` and `map`
  binaries of `developer-tools`.
- Related features: the [map raster pipeline](/documentation_v2/tools_v2/developer-tools/map_raster_pipeline.md);
  the [uploaded terrain volume](/documentation_v2/assets_v2/uploaded_terrain_volume.md), the
  designed second tier; the [map streaming](/documentation_v2/website/map-engine/map_streaming.md)
  that loads the datasets in the browser.

## Behaviour

### The world objects export

```text
Workbench: Plugins > TBD > "Export TBD World Objects (full)"
  ─▶ $profile:TBD_WorldExport_full.jsonl, then TBD_WorldExport_full_meta.json (written last)
world copy-export-profile --terrain <T> --full --profile <dir>
  ─▶ assets_v2/scratch/<T>/export/raw-entities.jsonl
cargo xtask map export-terrain <T> --phase <P>
  ├─ world phase-gate    refuses a phase above the registry's importPhaseMax
  ├─ world build-objects chunks, catalogue, density, regions, census; patches the manifest
  └─ world build-roads   the road network from the game's .topo file
world verify-phase --terrain <T>
  ─▶ then raise importPhaseMax in assets_v2/terrains/terrain-registry.json by hand
```

1. With the terrain world open in Workbench and every layer loaded, the full-world export walks
   512 m cells and writes one JSON line per entity to the Workbench profile; its metadata file is
   written last and marks completion. The plugin runs from the menu or through
   `cargo xtask mcp call wb_execute_action`.
2. `world copy-export-profile` stages the export in the gitignored `assets_v2/scratch/<terrain>/`.
   `cargo xtask map export-terrain` stops with exit 2 and prints these operator steps when the
   staged export is missing.
3. `export-terrain` runs the phase gate, then `build-objects` and `build-roads`, and prints the
   `verify-phase` command. Phases run from `P1_buildings` through `P5_props` (the gate knows ten,
   to `P10_full`); a phase adds one class of objects.
4. `world verify-phase` proves the committed artifacts: per-phase gates (building anchors, tree
   and density checks, fences), roads and size, and E6, which rebuilds twice into scratch folders
   and requires both builds to equal the committed files byte for byte. After it passes, a person
   raises the terrain's `importPhaseMax` in the registry.

Each JSON artifact is written first and its binary twin is built by reading that JSON back
through the map engine's own loader, so both forms decode to the same rows, and every number is
spelled the same way on every run, so a rebuild compares byte for byte. Object classification
follows `contracts_v2/rules/prefab-classify.json`, whose render keys name the glyphs in
`assets_v2/glyphs/manifest.json`.

### The other exports

| Asset | Source | Tool | Gate |
|---|---|---|---|
| elevation model `dem/<terrain>-dem-16bit.png` | the Workbench DEM plugin's surface-height grid | `world raw-u16-dem-png` | `cargo xtask schema terrain-alignment` against the anchors |
| surface anchors `anchors/verification.json` | Workbench height probes at named points, by hand | none | `cargo xtask schema terrain-alignment` |
| satellite container and pyramids, Map view pyramid | the game's paks and the Workbench satellite export | `map` (the [raster pipeline](/documentation_v2/tools_v2/developer-tools/map_raster_pipeline.md)) | `map verify-unified`, `verify-pyramid` |
| `locations.json`, `height-labels.json` | the raw entity export, the elevation model | `map export-locations`, `export-height-labels` | `cargo xtask schema locations`, `height-labels`, `town-labels` |
| `road-names.json` | a curated route list, by hand | `map labels-rkyv` packs it with the two label files | `cargo xtask schema road-names` |
| water archives | the Workbench water export | `map water` | none of its own |
| building geometry `prefabs/` | the game's `.xob` models and Workbench voxel dumps | `cargo xtask map bvh-batch`, `blueprint-from-voxels`, `ingest-blueprints` | `cargo xtask verify blas-manifest` |
| glyph atlas | hand-drawn SVGs | `map build-glyph-atlas` | `cargo xtask schema map-glyphs` |

### Serving

The API mounts `assets_v2/terrains/` at `/map-assets` and `assets_v2/glyphs/` at
`/map-assets/glyphs`, below the rate limiter; the Mission Creator boots
`/map-assets/<terrain>/manifest.json` for the terrain its [mission](/documentation_v2/glossary/g_to_m.md#mission) names, and
loads every other file by the path the manifest gives. A terrain may ship a subset of files, and the map engine degrades
on what the manifest lists. The [assets README](/assets_v2/README.md#how-it-works) and the
[terrains README](/assets_v2/terrains/README.md#how-it-works) give the mounts and the rules.

### Storage

The JSON files and the forest-density tiles are plain git blobs; every other binary is in Git
LFS, and an LFS pattern in `.gitattributes` must exist before the first file of its kind is
committed. Scratch exports and tile pyramids are gitignored. `cargo xtask deploy website` leaves
`assets_v2/terrains/` out of its rsync, so each host keeps its own copy, checked by the deploy's
asset preflight.

### Known discrepancies

- `world validate-exports` gunzips every chunk index path
  (`tools_v2/developer-tools/src/world_export_pipeline/export_preparation/export_validation/artifact_integrity.rs:74-75`)
  — Everon's committed chunk index lists raw `.bin` paths, while `build-objects` writes `.json.gz`
  paths (`object_partitioning.rs:413`), so the check cannot pass on the committed data.
- `validate-exports` compares density tiles with raw tree counts (`artifact_integrity.rs:182-183`)
  — `build-objects` and `redensify` write the box-blurred canopy grid
  (`build_world_objects_opt.rs:72-73`).
- `build-objects --patch-manifest`, which `export-terrain` always passes, expects an `objects`
  block (`build_world_objects_opt.rs:308-310`) — the manifest schema makes it optional and Arland's
  manifest has none.
- The terrain registry lists Arland as `queued` with no phase shipped
  (`assets_v2/terrains/terrain-registry.json`) — the manifest gate hard-codes both terrains' sizes
  and heights (`tools_v2/developer-tools/src/map_verification/terrain_manifest.rs:229-246`), and CI
  checks only Everon's manifest.

## Data

- `assets_v2/terrains/terrain-registry.json`: one entry per terrain with its world bounds,
  manifest path, Workbench world, status and phases; read by tools only
  (`contracts_v2/definitions/terrain-registry.schema.json`).
- `assets_v2/terrains/<terrain>/manifest.json`: the index of every asset the browser loads
  (`contracts_v2/definitions/terrain-manifest.schema.json`); the
  [Everon README](/assets_v2/terrains/everon/README.md) lists each folder and file.
- `assets_v2/scratch/<terrain>/`: the gitignored staging area of every export.

## Design

A terrain is addressed only through its manifest, so the browser never guesses a path and a
dataset can grow phase by phase. Every artifact is rebuilt deterministically from a staged export,
and the gates prove the committed files by rebuilding them rather than by trusting the builder's
own checks. The pipelines are written for Everon first; Arland is registered and waits for its
export.

## Open work

- [T-1123 — Decide whether chunk index paths name the .bin or .json.gz](/.ai/tickets/T-1123.toml)
  and [T-1122 — Fix world validate-exports failing on the committed Everon chunks](/.ai/tickets/T-1122.toml)
  (idea, no plan): one authoritative chunk path form, which the builder, the committed index and
  `validate-exports` all follow.
- [T-1124 — Fix validate-exports density check ignoring the canopy blur](/.ai/tickets/T-1124.toml)
  (idea, no plan): the density check compares like with like.
- [T-1125 — Stop world export library code exiting the process mid-gate](/.ai/tickets/T-1125.toml)
  (idea, no plan): a failed rebuild inside `verify-phase` returns an error instead of ending the
  run.
- [T-1127 — Widen world verify-phase density checks and the E2c source scan](/.ai/tickets/T-1127.toml)
  (idea, no plan): later phases recheck density.
- [T-1074 — Fix world build-objects panicking on a manifest without objects](/.ai/tickets/T-1074.toml)
  and [T-1076 — Derive terrain checks and create dialog from the terrain registry](/.ai/tickets/T-1076.toml)
  (idea, no plan): a second terrain exports and checks without code changes.
- [T-1075 — Remove orphan BLAS files and fix stale terrain manifest docs](/.ai/tickets/T-1075.toml)
  (idea, no plan): every committed BLAS file is listed, and the manifest schema's texts match the
  loader.
- [T-1148 — Fix map export files overwriting each other and cell-edge duplicates](/.ai/tickets/T-1148.toml)
  and [T-1101 — Fix map export road classes, spline transforms and DEM result](/.ai/tickets/T-1101.toml)
  (idea, no plan): the Workbench exports stop overwriting each other, recording entities twice or
  misclassifying roads.
- [T-1117 — Stream map export-terrain world output while each step runs](/.ai/tickets/T-1117.toml)
  (idea, no plan): long builds show progress as they run.
- [T-935 — Map binary storage — hybrid rkyv + POD](/.ai/tickets/T-935.toml) (queued,
  [plan](/documentation_v2/tickets/plans/t-935_plan.md)) and its child
  [T-935.15 — Delete chunking: one container, spatial index, range fetch](/.ai/tickets/T-935.15.toml)
  (queued, [plan](/documentation_v2/tickets/plans/t-935_15_plan.md)): the object chunks give way to
  one container with a spatial index fetched by HTTP ranges.
- [T-121 — Terrain DEM export automation](/.ai/tickets/T-121.toml) (deferred, no plan): Arland's
  re-export and a game-mode height fallback.

## Decisions

- The browser reads only what a manifest names: a partial dataset works, and no reader hard-codes
  a file list.
- The committed artifacts must rebuild byte for byte: a change in the pipeline shows up as a diff,
  never as silent drift.
- Terrains stay out of the website deploy: each host keeps its own large, LFS-backed copy.
- The engine's own surface probe is the height authority: the elevation model is checked against
  anchors the engine reported, so placed objects cannot float or sink unnoticed.
