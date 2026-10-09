//! The map raster pipeline: a terrain's image and label assets from the game and its exports.
//!
//! **Role:** stitches the aerial orthophoto from the game's archives (`aerial_orthophoto`), builds
//! the unified satellite container and verifies it and its tile pyramids (`satellite_archive`,
//! `satellite_archive_container`), renders the cartographic Map view and its land cover
//! (`cartographic_rendering`), classifies and tints the inland water (`inland_water`), writes the
//! water and label archives (`inland_water_archive`, `map_label_archives`), exports the label
//! sets (`map_labels`) and builds the world-glyph atlas (`glyph_atlas`). [`entrypoint`] is the `map`
//! binary.
//! **Position:** tier 7 of `tools/map_assets`, over `world_export_pipeline` (number spelling, the
//! `.topo` and texture decoders), the world format, terrain and place-name crates, `enfusion_pak`,
//! `repository_layout` and `process_runner`. The `map` binary of `developer_tools` calls
//! [`entrypoint`]; `cargo xtask ci` runs that binary as a child process, so no image codec enters
//! xtask's dependency closure.
//! **Signals & state:** none held; each subcommand reads its inputs, writes its outputs and returns
//! its exit code.
//! **Invariants:** no subcommand writes an empty result over a committed asset; an archive is read
//! back through its validating reader before it is written; the same inputs give the same bytes;
//! every failure is an [`Error`] and only the binary decides the exit code.

mod aerial_orthophoto;
mod cartographic_rendering;
mod command_line;
mod decision_record_locations;
mod empty_write_refusal;
mod error;
mod glyph_atlas;
mod image_operations;
mod inland_water;
/// `water/water_vectors.rkyv` + `water/bathymetry.tbd-bath` from the Workbench inland-
/// water staging export. Separate from `inland_water`, which is the *image* lane (inland-water
/// classifier + ortho tint) and shares nothing with it but the word: this module reads the staging
/// rasters and writes binaries. Same split as `map_labels` versus `map_label_archives`.
mod inland_water_archive;
/// `locations/map_labels.rkyv`, the binary twin of `locations.json` +
/// `height-labels.json` + `road-names.json`. Separate from `map_labels`, which *produces* two of
/// those three JSON files: this module only ever reads them.
mod map_label_archives;
mod map_labels;
pub mod prelude;
mod satellite_archive;
/// The `TBDS` v2 satellite container (32-byte header + rkyv `TbdSatIndexV2`), the writer, the
/// reader and the tile geometry; `satellite_archive` keeps the call sites.
mod satellite_archive_container;

pub use command_line::entrypoint;
pub use error::{Error, Result};
