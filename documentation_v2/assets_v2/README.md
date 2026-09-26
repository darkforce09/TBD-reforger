**Status:** live

# Map assets documentation

The documents on the map data in `assets_v2/`: how a game terrain is exported into the committed
datasets the platform serves, and the designed tier for terrains uploaded at runtime. Developers
and AI agents read them below the [assets README](/assets_v2/README.md) and its child READMEs,
which say what each folder and file holds.

## Contents

```text
documentation_v2/assets_v2/
├── terrain_export_and_map_assets.md  Workbench exports, the world and raster pipelines, gates, serving
└── uploaded_terrain_volume.md        the designed volume for uploaded terrains: layout, ingest gates, rules
```

## How it works

Start with [terrain export and map assets](/documentation_v2/assets_v2/terrain_export_and_map_assets.md),
the end-to-end flow from a [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) world to a manifest the browser boots; it links the
[map raster pipeline](/documentation_v2/tools_v2/developer-tools/map_raster_pipeline.md) for the
image, label and water lanes. The [uploaded terrain volume](/documentation_v2/assets_v2/uploaded_terrain_volume.md)
describes a tier no code implements yet. Both follow the
[feature doc template](/documentation_v2/standards/templates/feature_doc.md).

| Doc | Covers | Code |
|---|---|---|
| [Terrain export and map assets](/documentation_v2/assets_v2/terrain_export_and_map_assets.md) | the world objects export, the other exports, gates, serving, storage | [`assets_v2/terrains/`](/assets_v2/terrains/README.md), [`world_export_pipeline/`](/tools_v2/developer-tools/src/world_export_pipeline/README.md) |
| [Uploaded terrain volume](/documentation_v2/assets_v2/uploaded_terrain_volume.md) | the second tier's layout, ingest gates and operating rules | [`assets_v2/storage_spec/`](/assets_v2/storage_spec/README.md) |

## Code

- [Map assets](/assets_v2/) — the terrains, glyphs and storage specification these documents
  cover.
- [Terrains](/assets_v2/terrains/) — the built-in datasets and the terrain registry.
- [Storage specification](/assets_v2/storage_spec/) — the folder of the uploaded tier's design.

## Boundaries

- Depends on: the datasets under `assets_v2/`, the developer-tools pipelines and gates, the xtask
  map commands and the API's `/map-assets` mount, which every claim is checked against; the
  feature doc template; the ticket registry for open work.
- Used by: the READMEs of `assets_v2/` and `assets_v2/storage_spec/`, which link these documents
  under Related documentation; the developer tools documentation.
- Rules: a document describes the committed data and code, and a disagreement goes under Known
  discrepancies with both places; the design of the uploaded tier lives in its document, and the
  `storage_spec/` README stays a short pointer to it.

## Related documentation

- [Map streaming](/documentation_v2/website/map-engine/map_streaming.md) — how the browser loads
  what the manifests name.
- [Local development](/documentation_v2/runbooks/local_development.md) — pulling the LFS map assets
  and serving them to the app.
