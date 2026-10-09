//! The unified satellite container (`.tbd-sat`): the verifier (a dependency-free byte parse), the
//! builder (Lanczos mip cascade, tile crop, VP8L through `image-webp`) and the tile pyramid verifier.
//!
//! **Role:** declares the satellite container builder and verifiers and re-exports their entries.
//! **Position:** a lane of the crate; the command line calls the subcommands it re-exports.
//! **Signals & state:** none.
//! **Invariants:** a version 1 and a version 2 container built from one source differ only in
//! their index.
//!
//! The container has two versions: version 2 (`satellite_archive_container`) frames a 32-byte
//! `TbdsHeader` and an rkyv `TbdSatIndexV2`, version 1 a hand-packed JSON table. Both writers consume the
//! same encoded block vector, so a v1 and a v2 bundle built from one source have **byte-identical
//! payloads** and differ only in their index — which makes "renders identically at every mip" a
//! property of the code rather than a hope. v1 is retained behind `--container-version 1`:
//! `everon-sat.tbd-sat` is committed in that shape until a Workbench export regenerates it.

use std::path::{Path, PathBuf};

use crate::error::{Result, ResultExt};
use serde_json::{Value, json};

use super::image_operations;
use super::satellite_archive_container::{
    BundleSummary, TileBuffer, mip_dimensions, tbds_v2_bytes, tbds_v2_index, verify_bundle_v2,
};
use ::repository_root::find_repository_root;
use time_source::iso_from_system_time;
use world_export_pipeline::json_number_formatting::js_num;

/* ─────────────────────────── verify-unified-satellite ─────────────────────────── */

/* ─────────────────────────── verify-tile-pyramid ─────────────────────────── */

/* ─────────────────────────── build-unified-satellite ─────────────────────────── */

mod satellite_and_pyramid_verification;
pub(crate) use satellite_and_pyramid_verification::verify_tile_pyramid;
pub(crate) use satellite_and_pyramid_verification::verify_unified_satellite;

mod build_unified_satellite;
pub(crate) use build_unified_satellite::build_unified_satellite;

#[cfg(test)]
pub(crate) use build_unified_satellite::build_tbds_v1_bytes;
