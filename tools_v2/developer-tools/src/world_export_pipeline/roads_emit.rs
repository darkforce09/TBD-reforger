//! T-935.6 — the rkyv twin of `objects/roads.json.gz`.
//!
//! `build-roads` writes the road network twice: the gzip-9 JSON the shipped loader still fetches,
//! and a `roads/road_network.rkyv` `RoadNetworkArchive`. Dual emission is deliberate and stays
//! until T-935.11/.13 flip the URL — deleting either write before then blinds one reader.
//!
//! # Why the emitter centrelines instead of copying the export's quad soup
//!
//! `roads.json.gz` ships road *surface* quads: alternating cross-edge point pairs, ~10× the
//! vertices of the line the map actually draws. Every page load then paid for
//! `extract_road_centerline` over all of it on
//! the main thread. The archive stores the centrelined result, so the work happens once here.
//!
//! # Why it narrows the JSON rather than the builder's own model
//!
//! Same reason `binary_emit` does: the JSON round trip is not the identity.
//! `build_roads_from_topo` holds `f64` topo vertices, writes them through
//! `round2` and `js_num`, and the loader centrelines *those*. Reading the
//! file that was just written puts the emitter on the loader's own chain
//! (`bytes_to_json` → `parse_roads_payload`), so the archive equals the JSON decode by
//! construction rather than by coincidence — including the drops: a segment the loader rejects
//! (unknown class, under two centreline vertices, a non-finite point) is absent from both.
//!
//! # Class codes
//!
//! `road_class` is a byte on the wire. The table is
//! `road_class_code` /
//! `road_class_name` in `map-engine-core` — *one*
//! table, linked by both the writer here and the reader in `world::roads`, so they cannot drift
//! into disagreeing about what a byte means. A class the table cannot code is a hard error here
//! and a hard error there; neither side invents a fallback.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use website_map_engine::io::archives::codec::access_checked;
use website_map_engine::io::archives::codec::to_bytes;
use website_map_engine::io::archives::roads::RoadNetworkArchive;
use website_map_engine::io::archives::roads::RoadSegmentArchive;
use website_map_engine::io::archives::version::ARCHIVE_SCHEMA_VERSION;
use website_map_engine::streaming::loaders::store::bytes_to_json;
use website_map_engine::world::environment::locations::route_placement::road_class_code;
use website_map_engine::world::terrain::roads::network::RoadSegment;
use website_map_engine::world::terrain::roads::network::parse_roads_payload;

use crate::browser_testing::server::repo_root;

/// The gzip-JSON road export, relative to a terrain directory.
pub const ROADS_GZ: &str = "objects/roads.json.gz";

/// The rkyv road network, relative to a terrain directory. Matches the manifest's
/// `objects.binary.roads` path (`map_engine_core::world::ObjectsBinaryBlock`), which T-935.11
/// will point the SPA at.
pub const ROAD_NETWORK_RKYV: &str = "roads/road_network.rkyv";

/// Centrelined [`RoadSegment`]s → the wire archive, in file order.
///
/// # Errors
/// When a segment's class is not in the shared class table. That is fatal rather than a skip or a
/// zero byte: the reader treats an unnameable code as corruption (`world::roads::from_archive`),
/// so writing one would produce a file that only fails at load time, on the user's machine.
pub fn to_archive(segments: &[RoadSegment]) -> Result<RoadNetworkArchive> {
    let mut out = Vec::with_capacity(segments.len());
    for (i, s) in segments.iter().enumerate() {
        let code = road_class_code(&s.road_class);
        if code == 0 {
            bail!(
                "road segment {i} (id {:?}) has class {:?}, which the wire class table cannot \
                 code — refusing to write an archive the loader would reject",
                s.id,
                s.road_class
            );
        }
        out.push(RoadSegmentArchive {
            id: s.id.clone(),
            road_class: code,
            #[allow(clippy::cast_possible_truncation)]
            width_m: s.width_m as f32,
            centerline: s
                .points
                .iter()
                .map(|p| {
                    #[allow(clippy::cast_possible_truncation)]
                    [p[0] as f32, p[1] as f32]
                })
                .collect(),
        });
    }
    Ok(RoadNetworkArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        segments: out,
    })
}

/// Serialise, **re-read through the validating reader**, then write. Returns the file size.
///
/// The read-back is not ceremony: `access_checked` is the only thing the SPA will ever use on this
/// file, so an archive that cannot survive it is a broken file whether or not `to_bytes` returned
/// `Ok`, and the emitter is the last place that can say so cheaply.
///
/// # Errors
/// When serialisation, validation or the write fails.
pub fn write_road_network_rkyv(path: &Path, archive: &RoadNetworkArchive) -> Result<usize> {
    let bytes = to_bytes(archive).map_err(|e| anyhow::anyhow!("{e}"))?;
    access_checked::<RoadNetworkArchive>(&bytes)
        .map_err(|e| anyhow::anyhow!("emitted archive fails its own validating read: {e}"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    std::fs::write(path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(bytes.len())
}

/// Read `objects/roads.json.gz` from `terrain_dir` and build the archive from it.
///
/// # Errors
/// When the JSON is missing or undecodable, or when it centrelines to nothing — an empty network
/// is refused rather than written, the same guard `build-roads` puts on the JSON itself (T-537).
pub fn build_road_network_archive(terrain_dir: &Path) -> Result<RoadNetworkArchive> {
    let src = terrain_dir.join(ROADS_GZ);
    let raw = std::fs::read(&src).with_context(|| format!("read {}", src.display()))?;
    let doc = bytes_to_json(&raw).map_err(|e| anyhow::anyhow!("{} decode: {e}", src.display()))?;
    let segments = parse_roads_payload(&doc);
    super::refuse_empty_write(
        "roads-rkyv",
        segments.is_empty(),
        "zero centrelined road segments — refusing to write an empty road_network.rkyv",
    )?;
    to_archive(&segments)
}

/// Resolve a `--terrain` value the way `build_roads_from_topo` resolves its `--out`: an explicit
/// base wins, otherwise `packages/map-assets/<terrain>`.
#[must_use]
pub fn resolve_terrain_dir(terrain: &str, out_base: Option<&Path>) -> PathBuf {
    out_base.map_or_else(
        || repo_root().join("packages/map-assets").join(terrain),
        Path::to_path_buf,
    )
}

/// Build and write `roads/road_network.rkyv` for one terrain directory. Returns the path written
/// and its size, so the caller can print both emitted paths.
///
/// # Errors
/// When the source JSON is missing/undecodable/empty, or the write fails.
pub fn emit_road_network(terrain_dir: &Path) -> Result<(PathBuf, usize)> {
    let archive = build_road_network_archive(terrain_dir)?;
    let out = terrain_dir.join(ROAD_NETWORK_RKYV);
    let bytes = write_road_network_rkyv(&out, &archive)?;
    Ok((out, bytes))
}

/// `world roads-rkyv --terrain <id> [--out <base>]` — the archive on its own, from the committed
/// JSON.
///
/// `build-roads` emits both files, but it needs the game's `.pak` VFS to decode the topo section,
/// so it cannot run in CI or on a machine without Arma installed. This entry point re-derives the
/// archive from the committed `roads.json.gz` alone, which is what makes the binary lane
/// reproducible in the repo rather than only on the export box.
///
/// # Errors
/// When the terrain directory or its road export is missing, or the emit fails.
pub fn emit_road_network_cli(terrain: &str, out_base: Option<&Path>) -> Result<u8> {
    let dir = resolve_terrain_dir(terrain, out_base);
    if !dir.is_dir() {
        eprintln!("roads-rkyv: no terrain directory at {}", dir.display());
        return Ok(1);
    }
    let (out, bytes) = emit_road_network(&dir)?;
    println!(
        "roads-rkyv: {} → {} ({bytes} bytes)",
        terrain,
        out.display()
    );
    Ok(0)
}

#[cfg(test)]
#[path = "tests/roads_emit/tests.rs"]
mod tests;
