# Chunk Partitioner

World object building separates preparation and classification in `object_partitioning.rs`, chunk emission in `build_world_objects_opt.rs`, and density/road processing. The parent module owns shared row types and public entrypoints.

Source modules: `build_world_objects_opt.rs`, `may_clear_density_dir.rs`, `object_partitioning.rs`, `redensify_from_committed.rs`.
