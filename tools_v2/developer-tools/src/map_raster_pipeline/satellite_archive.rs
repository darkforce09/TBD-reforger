//! T-165.9 — unified satellite bundle (tbd-sat): the verifier (port of
//! `verify-unified-satellite.mjs`, dep-free byte parse) and the builder (port of
//! `build-unified-satellite.mjs` — Lanczos cascade mips, tile crop, VP8L via image-webp).
//! Plus the tile-pyramid verifier (port of `verify-tile-pyramid.mjs`).
//!
//! T-935.10 — the container gained a **version 2** (`satellite_archive_container`): a 32-byte `TbdsHeader`
//! plus an rkyv `TbdSatIndexV2` instead of v1's hand-packed JSON table. Both writers consume the
//! same encoded block vector, so a v1 and a v2 bundle built from one source have **byte-identical
//! payloads** and differ only in their index — which makes "renders identically at every mip" a
//! property of the code rather than a hope. v1 is retained behind `--container-version 1`:
//! `everon-sat.tbd-sat` is committed in that shape until T-935.13 regenerates it.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::{Value, json};

use super::image_operations;
use super::satellite_archive_container::{
    BundleSummary, TileBuf, mip_dims, tbds_v2_bytes, tbds_v2_index, verify_bundle_v2,
};
use crate::browser_testing::server::repo_root;
use crate::timestamp_formatting::iso_from_system_time;
use crate::world_export_pipeline::json_number_formatting::js_num;

/* ─────────────────────────── verify-unified-satellite ─────────────────────────── */

/* ─────────────────────────── verify-tile-pyramid ─────────────────────────── */

/* ─────────────────────────── build-unified-satellite ─────────────────────────── */

#[path = "satellite_archive/map_assets_root.rs"]
mod map_assets_root;
pub use map_assets_root::verify_tile_pyramid;
pub use map_assets_root::verify_unified_satellite;

#[path = "satellite_archive/build_unified_satellite.rs"]
mod build_unified_satellite;
pub use build_unified_satellite::build_unified_satellite;

#[cfg(test)]
pub(crate) use build_unified_satellite::build_tbds_v1_bytes;
