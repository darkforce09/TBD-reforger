**Status:** live

# Map assets documentation

The documents on the map data in `assets/`: how a game terrain is exported into the committed
datasets the platform serves, and the designed tier for terrains uploaded at runtime. Developers
and AI agents read them below the [assets README](/assets/README.md) and its child READMEs,
which say what each folder and file holds.

## Contents

```text
documentation/assets/
├── terrain_export_and_map_assets.md  Workbench exports, the world and raster pipelines, gates, serving
└── uploaded_terrain_volume.md        the designed volume for uploaded terrains: layout, ingest gates, rules
```

## How it works

Start with [terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md),
the end-to-end flow from a [Workbench](/documentation/glossary/n_to_z.md#workbench) world to a manifest the browser boots; it links the
[map raster pipeline](/documentation/tools/developer_tools/map_raster_pipeline.md) for the
image, label and water lanes. The [uploaded terrain volume](/documentation/assets/uploaded_terrain_volume.md)
describes a tier no code implements yet. Both follow the
[feature doc template](/documentation/standards/templates/feature_doc.md).

| Doc | Covers | Code |
|---|---|---|
| [Terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md) | the world objects export, the other exports, gates, serving, storage | [`assets/terrains/`](/assets/terrains/README.md), [`world_export_pipeline/`](/tools/developer_tools/src/world_export_pipeline/README.md) |
| [Uploaded terrain volume](/documentation/assets/uploaded_terrain_volume.md) | the second tier's layout, ingest gates and operating rules | [`assets/storage_spec/`](/assets/storage_spec/README.md) |

## Code

- [Map assets](/assets/) — the terrains, glyphs and storage specification these documents
  cover.
- [Terrains](/assets/terrains/) — the built-in datasets and the terrain registry.
- [Storage specification](/assets/storage_spec/) — the folder of the uploaded tier's design.

## Boundaries

- Depends on: the datasets under `assets/`, the developer_tools pipelines and gates, the xtask
  map commands and the API's `/map-assets` mount, which every claim is checked against; the
  feature doc template; the ticket registry for open work.
- Used by: the READMEs of `assets/` and `assets/storage_spec/`, which link these documents
  under Related documentation; the developer tools documentation.
- Rules: a document describes the committed data and code, and a disagreement goes under Known
  discrepancies with both places; the design of the uploaded tier lives in its document, and the
  `storage_spec/` README stays a short pointer to it.

## Related documentation

- [Map streaming](/documentation/legacy/map_engine/map_streaming.md) — how the browser loads
  what the manifests name.
- [Local development](/documentation/runbooks/local_development.md) — pulling the LFS map assets
  and serving them to the app.
