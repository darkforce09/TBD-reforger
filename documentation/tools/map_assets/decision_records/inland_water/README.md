**Status:** live

# Inland water decision records

The source choices of the water masks: where the ocean and inland water come from, how the inland
mask is refined, and what the terrain's topology file does and does not carry.

## Contents

```text
documentation/tools/map_assets/decision_records/inland_water/
├── refine_spike.json        the two-tier refinement of the inland mask and its results
├── source_spike.json        the topology file's road network and its subtraction from the mask
└── water_source_spike.json  the ocean and inland mask candidates and the chosen pair
```

## Code

- [Inland water lane](/tools/map_assets/map_raster_pipeline/src/inland_water/) — the water source
  analysis that writes and reads these records.
