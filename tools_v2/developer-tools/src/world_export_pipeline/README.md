# Macro World Export Pipeline (`developer-tools/src/world_export_pipeline`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Macro-scale geospatial compilation pipeline: 512m chunk spatial partitioning, road networks, DEM elevations, and mathematical invariant gates.

Eradicates cryptic abbreviations (`edds.rs`, `jsval.rs`, `aux.rs`) in compliance with **Law 4 (Zero Context Needed)**.

---

## Submodules

- **`chunk_partitioner/`** (split from `build.rs`): Partitions hundreds of thousands of entity rows into 512m `TBDC` chunk containers.
- **`prefab_catalog/`** (split from `catalog_emit.rs`): Collates prefab archetypes into zero-copy binary catalogs.
- **`prefab_classifier.rs`**: Resolves prefab resource names into kinds and classes.
- **`prefab_reclassifier/`** (split from `reclassify.rs`): Evaluates classification drift against committed catalogs.
- **`road_network_export/`**: Decodes `.topo` binary network data and exports road graphs to `.rkyv`.
- **`forest_smoothing/`** (split from `forest_smooth.rs`): Runs Chaikin smoothing algorithms over forest canopy rings.
- **`vegetation_density/`** (formerly `density.rs`): Calculates TBDD corner vegetation density grids.
- **`dem_elevation_import.rs`** (split from `aux.rs`): Decodes 16-bit raw elevation arrays into DEM PNGs.
- **`export_validation.rs`** (split from `aux.rs`): Audits export directory structure and integrity.
- **`object_census.rs`** (split from `aux.rs`): Tally counters for entity census verification.
- **`texture_decoder.rs`** (formerly `edds.rs`): Decompresses LZ4 chunks and decodes BC7 block-compressed textures.
- **`json_number_formatting.rs`** (formerly `jsval.rs`): Enforces ECMAScript `JSON.stringify` numeric formatting rules.
- **`mathematical_gates/`** (split from `gates.rs`): Enforces G1–G12 spatial invariants and phase gates.
