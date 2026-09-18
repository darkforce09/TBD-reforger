# World Export Pipeline

World export processing includes source classification, chunk partitioning and emission, road networks, vegetation density, forest contours and smoothing, DEM conversion, and mathematical verification. `export_preparation/` groups census, artifact validation, profile copying, DEM input, and aerial-cell cataloging.

Source modules: `binary_emit.rs`, `catalog_emit.rs`, `chunk_partitioner.rs`, `classify.rs`, `cli.rs`, `enfusion_texture_decoder.rs`, `export_preparation.rs`, `forest_contours.rs`, `forest_smoothing.rs`, `json_number_formatting.rs`, `mathematical_verification.rs`, `mod.rs`, `polygon_geometry.rs`, `reclassify.rs`, `roads_emit.rs`, `topo.rs`, `vegetation_density.rs`.
