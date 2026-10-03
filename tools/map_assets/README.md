# Map asset crates

The offline pipelines that build and check the served map data: the blueprint compiler for building
interiors, the world export and raster pipelines that turn a Workbench export into terrain,
satellite and cartographic assets, and the verification of the assets they write.

## Contents

```text
tools/map_assets/
├── blueprint_compiler/  building blueprints from voxel dumps and game models, occlusion sidecars, the blueprint archive
├── map_asset_verification/  `map_asset_verification`: the gates over a terrain's committed map assets and the map golden fixtures, and the world line-of-sight probe
├── map_raster_pipeline/  `map_raster_pipeline`: a terrain's satellite container, tile pyramids, cartographic render, labels, water archives and the glyph atlas
└── world_export_pipeline/  `world_export_pipeline`: a terrain's chunks, catalogue, census, density, regions, roads and elevation from a Workbench export, and their gates
```
