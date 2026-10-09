//! `map water`: the two water binaries, from the Workbench staging export.
//!
//! **Role:** the staging and output names of `map water` and the in-memory mip level; declares
//! the reductions and the emitter.
//! **Position:** a lane of the crate; the command line calls `emit_water_archive`.
//! **Signals & state:** none held; see below for the streaming.
//! **Invariants:** the same staging inputs give the same bytes.
//!
//! * `water/water_vectors.rkyv` — a `WaterVectorsArchive` built from the export scratch's
//!   `water/TBD_InlandWaterExport_vectors.json` (lake rings, river centrelines, pond
//!   rings), each carrying the still-water surface height a ring of 2D points cannot say.
//! * `water/bathymetry.tbd-bath` — the `TBDB` mip pyramid (spec §3.3) built from the two ASCII
//!   rasters `TBD_InlandWaterExport_{depth,mask}.txt`.
//!
//! Separate from `super::inland_water`, which is the *image* lane (`analyze-water-sources` /
//! `composite-water-ortho`): that module tints an ortho, this one emits binaries. Same split as
//! `super::map_labels` versus `super::map_label_archives`.
//!
//! # The rasters are never held in memory
//!
//! Everon's two `.txt` files are 327,680,000 bytes **each** and decode to a 12800×12800 grid —
//! 491 MB of level-0 payload on its own. `write_bathymetry` therefore reads each file one line at
//! a time into a reused buffer and writes level 0 straight through to the output as it goes, while
//! folding the *next* level (a quarter of the size) into memory in the same pass. Every level after
//! that is folded from the one before it, so peak residency is level 1, not level 0. That is why
//! the depth and mask blocks come from two separate files in two separate passes: the format wants
//! all of a level's depths before any of its mask, and buffering one to interleave the other is the
//! 164 MB this design exists to avoid.
//!
//! # The mask is a boolean here, and the class byte is not lost — it was never carried
//!
//! `TBD_MapExportWater.c` writes a class per texel (`0` land, `1` ocean, `2` pond/lake, `3` river).
//! `TBDB`'s mask is specified as `1 = water`, and the mip reduction is "any water", which a
//! multi-valued class cannot express (what is the union of a river and a lake?). So the class is
//! collapsed to a boolean on the way in. Callers that need the class read the *vectors* archive,
//! which keeps lakes, rivers and ponds apart by construction.
//!
//! # Determinism
//!
//! Same staging inputs → same bytes, on any host: the header is a `Pod` written little-endian, the
//! payload is a fixed walk of the grid, the padding is zeros, and the fold is integer `max` / `or`.
//! `bytes_are_deterministic_across_runs` re-runs the whole emitter and compares.

use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::error::{Result, ResultExt, bail};
use serde_json::Value;

use water_bodies::vectors::downsample_index;
use world_file_formats::archives::codec::access_checked;
use world_file_formats::archives::codec::to_bytes;
use world_file_formats::archives::version::ARCHIVE_SCHEMA_VERSION;
use world_file_formats::archives::water::WaterBody;
use world_file_formats::archives::water::WaterLine;
use world_file_formats::archives::water::WaterVectorsArchive;
use world_file_formats::containers::header::ContainerHeader;
use world_file_formats::containers::tbdb::TbdbHeader;

use ::repository_root::find_repository_root;

/// Where the Workbench inland-water export lands, relative to the export scratch: under
/// `assets/scratch/<terrain>/` for a terrain id, under `<dir>/scratch/` for a directory
/// argument (see [`scratch_dir`]). Gitignored either way.
pub(crate) const STAGING_WATER: &str = "water";
/// Staging source: the vector export (lakes + rivers + inland bodies).
pub(crate) const VECTORS_JSON: &str = "TBD_InlandWaterExport_vectors.json";
/// Staging source: grid dimensions and the depth quantisation.
pub(crate) const META_JSON: &str = "TBD_InlandWaterExport_meta.json";
/// Staging source: the per-texel water class raster (ASCII, one row per line).
pub(crate) const MASK_TXT: &str = "TBD_InlandWaterExport_mask.txt";
/// Staging source: the per-texel depth raster, in the meta's depth unit.
pub(crate) const DEPTH_TXT: &str = "TBD_InlandWaterExport_depth.txt";
/// Terrain-relative output: the rkyv vector archive (spec §4).
pub(crate) const WATER_VECTORS_RKYV: &str = "water/water_vectors.rkyv";
/// Terrain-relative output: the `TBDB` pyramid (spec §3.3).
pub(crate) const BATHYMETRY_TBDB: &str = "water/bathymetry.tbd-bath";

/// Output buffer for the ~655 MB everon pyramid. One `write` syscall per megabyte instead of one
/// per texel.
const WRITE_BUF: usize = 1 << 20;

/* ─────────────────────────────── the two mip reductions ─────────────────────────────── */

/* ──────────────────────────────────── staging metadata ──────────────────────────────── */

/// What the staging meta says about the two rasters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RasterMetadata {
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Metres per stored depth unit (`depthScaleToMeters`; everon exports decimetres → 0.1).
    pub(crate) depth_scale: f32,
}

/* ─────────────────────────────────────── vectors ────────────────────────────────────── */

/* ────────────────────────────────────── bathymetry ──────────────────────────────────── */

/// One level of the pyramid held in memory, while the next is folded out of it.
struct Level {
    width: usize,
    height: usize,
    depth: Vec<u16>,
    mask: Vec<u8>,
}

impl Level {
    /// Dimensions of the level below this one, `max(1, n >> 1)` per side.
    fn next_dims(&self) -> (usize, usize) {
        ((self.width / 2).max(1), (self.height / 2).max(1))
    }

    /// Fold this level into the next through the two reductions.
    fn fold(&self) -> Level {
        let (nw, nh) = self.next_dims();
        let mut out = Level {
            width: nw,
            height: nh,
            depth: vec![0; nw * nh],
            mask: vec![0; nw * nh],
        };
        for z in 0..self.height {
            #[allow(clippy::cast_possible_truncation)]
            let dz = downsample_index(z as u32, nh as u32) as usize;
            for x in 0..self.width {
                #[allow(clippy::cast_possible_truncation)]
                let dx = downsample_index(x as u32, nw as u32) as usize;
                let (src, dst) = (z * self.width + x, dz * nw + dx);
                out.depth[dst] = reduce_depth_keeping_deepest(out.depth[dst], self.depth[src]);
                out.mask[dst] = reduce_mask_keeping_any_water(out.mask[dst], self.mask[src]);
            }
        }
        out
    }

    fn write_to(&self, out: &mut impl Write) -> Result<usize> {
        let mut row = Vec::with_capacity(self.width * 2);
        for chunk in self.depth.chunks(self.width) {
            row.clear();
            for d in chunk {
                row.extend_from_slice(&d.to_le_bytes());
            }
            out.write_all(&row)?;
        }
        out.write_all(&self.mask)?;
        pad_to_four_bytes(out, self.depth.len() * 2 + self.mask.len())
    }
}

/* ─────────────────────────────────────── the CLI ────────────────────────────────────── */

#[cfg(test)]
#[path = "tests/inland_water_archive_tests.rs"]
mod tests;

mod water_archive_codec;
pub(crate) use water_archive_codec::build_water_vectors;
pub(crate) use water_archive_codec::mip_count;
use water_archive_codec::pad_to_four_bytes;
pub(crate) use water_archive_codec::parse_meta;
use water_archive_codec::reduce_depth_keeping_deepest;
use water_archive_codec::reduce_mask_keeping_any_water;
pub(crate) use water_archive_codec::scratch_dir;
pub(crate) use water_archive_codec::terrain_dir;
pub(crate) use water_archive_codec::write_bathymetry;
pub(crate) use water_archive_codec::write_water_vectors;

mod water_archive_emission;
pub(crate) use water_archive_emission::emit_water_archive;
