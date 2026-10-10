**Status:** live

# Aerial orthophoto decision records

The seam analysis of a terrain's satellite supertexture: how visible the tile seams are, measured
per seam, and the thresholds the analysis judged them against.

## Contents

```text
documentation/tools/map_assets/decision_records/aerial_orthophoto/
└── seam_analysis.json  per-seam gradient measurements, their summary and the anchor spot checks
```

## Code

- [Aerial orthophoto lane](/tools/map_assets/map_raster_pipeline/src/aerial_orthophoto/) — the
  supertexture seam analysis that writes the record.
