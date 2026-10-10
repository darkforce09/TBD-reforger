**Status:** live

# Map raster pipeline decision records

The JSON records behind the source choices of three lanes of the map raster pipeline: which input
each lane reads, the measurements that chose it and the candidates it rejected. Developers who
change a lane's source read the matching record first.

## Contents

```text
documentation/tools/map_assets/decision_records/
├── aerial_orthophoto/       the seam analysis of the satellite supertexture
├── cartographic_rendering/  the landcover source choice of the cartographic render
└── inland_water/            the ocean and inland water mask sources and their refinement
```

## How it works

Each record is the JSON a pipeline analysis command writes, kept as captured: its inputs, the
parameters, the measured results and the decision. A command that runs the analysis again
rewrites its own record and leaves the others alone. The records are data, not prose; the
[map raster pipeline](/documentation/tools/map_assets/map_raster_pipeline.md) document states the
choices they support.

## Code

- [Map raster pipeline](/tools/map_assets/map_raster_pipeline/) — `decision_record_locations.rs`
  names these folders, and the analysis commands write the records.

## Boundaries

- Depends on: the analysis commands of `map_raster_pipeline`, which write the records.
- Used by: the map raster pipeline document; the cartographic render, which cites the landcover
  record in its output.
- Rules: a record changes only when its analysis runs again; a folder's path keeps its spelling or
  `decision_record_locations.rs` changes with it.

## Related documentation

- [Map raster pipeline](/documentation/tools/map_assets/map_raster_pipeline.md) — the lanes these
  records decide.
