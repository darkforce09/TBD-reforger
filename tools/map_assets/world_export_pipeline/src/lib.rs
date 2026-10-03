//! The world-export pipeline: a terrain's committed map data from a Workbench world export.
//!
//! **Role:** stages a world export ([`export_preparation`]), builds the object chunks, prefab
//! catalogue, census, density tiles, forest regions and road network from it and the game's
//! archives ([`chunk_partitioner`], [`catalog_emit`], [`binary_emit`], [`roads_emit`]),
//! reclassifies a committed catalogue ([`reclassify`]), and runs the gates that prove the
//! committed artifacts ([`mathematical_verification`]). [`entrypoint`] is the `world` binary.
//! It also drives `cargo xtask map export-terrain` ([`export_terrain_driver`]) and writes a
//! terrain's map tile index ([`map_tile_index`]).
//! **Position:** tier 6 of `tools/map_assets`, over the world format, terrain and world object
//! crates, `enfusion_pak`, `repository_layout`, `process_runner` and `verification_core`. The
//! `world` binary of `developer_tools` calls [`entrypoint`]; xtask's `map` dispatch calls the
//! export driver and the tile index writer; the map raster pipeline reads
//! [`json_number_formatting`], [`topo`] and [`enfusion_texture_decoder`]; the map verification
//! reads [`binary_emit`], [`forest_contours`], [`polygon_geometry`] and [`vegetation_density`].
//! **Signals & state:** none held; each stage reads its inputs, writes its outputs and returns.
//! **Invariants:** every JSON artifact and its binary twin decode to the same rows; every number is
//! written as JavaScript writes it, so a rebuild is byte-comparable with the committed file; no
//! stage writes an empty set over committed data; every failure is an [`Error`] and only the
//! binary decides the exit code.

/// The `TBDC` binary twin written beside every chunk `.json.gz`.
pub mod binary_emit;
/// The rkyv twins of the three catalogue JSONs (`objects/prefabs.rkyv`,
/// `objects/type-inventory.rkyv`, `objects/forest-regions.rkyv`), written beside them by
/// `build_world_objects`.
pub mod catalog_emit;
mod census_counts;
pub mod chunk_partitioner;
pub mod classify;
mod command_line;
mod empty_write_refusal;
pub mod enfusion_texture_decoder;
mod error;
pub mod export_locations;
pub mod export_preparation;
/// `cargo xtask map export-terrain`: the phase gate, the staged export check and the object and
/// road builds, each a `world` child process.
pub mod export_terrain_driver;
pub mod forest_contours;
/// Chaikin smoothing of the Path B forest rings, between `forest::trace_rings` and the
/// `forest-regions.json.gz` emit.
pub mod forest_smoothing;
pub mod json_number_formatting;
/// `cargo xtask map tile-index`: the `index.json` of a terrain's map tile pyramid.
pub mod map_tile_index;
pub mod mathematical_verification;
pub mod polygon_geometry;
pub mod prelude;
pub mod reclassify;
/// The `roads/road_network.rkyv` twin written beside `objects/roads.json.gz`.
pub mod roads_emit;
pub mod topo;
pub mod vegetation_density;

pub use command_line::entrypoint;
pub use error::{Error, Result};
