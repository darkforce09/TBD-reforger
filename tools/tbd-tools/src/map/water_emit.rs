//! T-935.9 — `map water`: the two water binaries, from the Workbench staging export.
//!
//! * `water/water_vectors.rkyv` — a [`WaterVectorsArchive`] built from
//!   `staging/water/TBD_InlandWaterExport_vectors.json` (lake rings, river centrelines, pond
//!   rings), each carrying the still-water surface height a ring of 2D points cannot say.
//! * `water/bathymetry.tbd-bath` — the `TBDB` mip pyramid (spec §3.3) built from the two ASCII
//!   rasters `TBD_InlandWaterExport_{depth,mask}.txt`.
//!
//! Separate from [`super::water`], which is the *image* lane (`analyze-water-sources` /
//! `composite-water-ortho`): that module tints an ortho, this one emits binaries. Same split as
//! [`super::labels`] versus [`super::labels_emit`].
//!
//! # The rasters are never held in memory
//!
//! Everon's two `.txt` files are 327,680,000 bytes **each** and decode to a 12800×12800 grid —
//! 491 MB of level-0 payload on its own. [`write_bathymetry`] therefore reads each file one line at
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

use anyhow::{Context, Result, bail};
use serde_json::Value;

use map_engine_core::world::binary::archives::{
    ARCHIVE_SCHEMA_VERSION, WaterBody, WaterLine, WaterVectorsArchive,
};
use map_engine_core::world::binary::chunk_container::{ContainerHeader, TbdbHeader};
use map_engine_core::world::binary::{access_checked, to_bytes};
use map_engine_core::world::downsample_index;

use crate::serve::repo_root;

/// Terrain-relative staging directory the Workbench export lands in (gitignored).
pub const STAGING_WATER: &str = "staging/water";
/// Staging source: the vector export (lakes + rivers + inland bodies).
pub const VECTORS_JSON: &str = "TBD_InlandWaterExport_vectors.json";
/// Staging source: grid dimensions and the depth quantisation.
pub const META_JSON: &str = "TBD_InlandWaterExport_meta.json";
/// Staging source: the per-texel water class raster (ASCII, one row per line).
pub const MASK_TXT: &str = "TBD_InlandWaterExport_mask.txt";
/// Staging source: the per-texel depth raster, in the meta's depth unit.
pub const DEPTH_TXT: &str = "TBD_InlandWaterExport_depth.txt";
/// Terrain-relative output: the rkyv vector archive (spec §4).
pub const WATER_VECTORS_RKYV: &str = "water/water_vectors.rkyv";
/// Terrain-relative output: the `TBDB` pyramid (spec §3.3).
pub const BATHYMETRY_TBDB: &str = "water/bathymetry.tbd-bath";

/// Output buffer for the ~655 MB everon pyramid. One `write` syscall per megabyte instead of one
/// per texel.
const WRITE_BUF: usize = 1 << 20;

/* ─────────────────────────────── the two mip reductions ─────────────────────────────── */

/// Spec §3.3: a coarse texel keeps the **deepest** depth of the block it covers.
///
/// Conservative in the same direction as [`reduce_mask`] — a coarse level never reports shallower
/// water than the fine one it summarises.
fn reduce_depth(acc: u16, v: u16) -> u16 {
    acc.max(v)
}

/// Spec §3.3: a coarse texel is water if **any** texel of the block it covers is.
///
/// This is the polarity the whole pyramid rests on. Invert it and every level above 0 marks the
/// lakes dry and the fields wet, which a placement guard reads as permission — the
/// `every_mip_level_agrees_with_level_zero` pin exists for exactly this line.
fn reduce_mask(acc: u8, v: u8) -> u8 {
    acc | u8::from(v != 0)
}

/* ──────────────────────────────────── staging metadata ──────────────────────────────── */

/// What the staging meta says about the two rasters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterMeta {
    pub width: u32,
    pub height: u32,
    /// Metres per stored depth unit (`depthScaleToMeters`; everon exports decimetres → 0.1).
    pub depth_scale: f32,
}

/// Read `TBD_InlandWaterExport_meta.json`.
///
/// # Errors
/// Malformed JSON, a missing or non-positive `widthPx` / `heightPx`, or a non-finite,
/// non-positive `depthScaleToMeters`. Every one of those would produce a container whose header
/// lies about the grid it contains.
pub fn parse_meta(text: &str) -> Result<RasterMeta> {
    let v: Value = serde_json::from_str(text).context("parse water meta json")?;
    let dim = |k: &str| -> Result<u32> {
        let n = v.get(k).and_then(Value::as_u64).unwrap_or(0);
        if n == 0 || n > u64::from(u32::MAX) {
            bail!("water meta: {k} must be a positive u32, got {:?}", v.get(k));
        }
        Ok(n as u32)
    };
    let scale = v
        .get("depthScaleToMeters")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if !(scale.is_finite() && scale > 0.0) {
        bail!("water meta: depthScaleToMeters must be finite and positive, got {scale}");
    }
    Ok(RasterMeta {
        width: dim("widthPx")?,
        height: dim("heightPx")?,
        #[allow(clippy::cast_possible_truncation)]
        depth_scale: scale as f32,
    })
}

/// Levels in the pyramid: keep halving until the longer side is one texel, inclusive of both ends.
///
/// `1 + floor(log2(max(w, h)))`, which is `32 - leading_zeros`. 12800 → 14 levels, the coarsest
/// 1×1; 4 → 3; 1 → 1.
#[must_use]
pub fn mip_count(width: u32, height: u32) -> u16 {
    let longest = width.max(height).max(1);
    // At most 32, so the cast is exact.
    #[allow(clippy::cast_possible_truncation)]
    let n = (32 - longest.leading_zeros()) as u16;
    n
}

/* ─────────────────────────────────────── vectors ────────────────────────────────────── */

/// `[x, y, z]` → `[x, z]`, the horizontal plane the ring lives in. `y` is carried once per body as
/// `surface_y`, not per point.
fn xz(p: &Value, what: &str) -> Result<[f32; 2]> {
    let a = p
        .as_array()
        .filter(|a| a.len() == 3)
        .with_context(|| format!("{what}: expected a [x, y, z] point, got {p}"))?;
    let n = |i: usize| -> Result<f32> {
        let v = a[i]
            .as_f64()
            .with_context(|| format!("{what}: component {i} is not a number"))?;
        if !v.is_finite() {
            bail!("{what}: component {i} is {v}");
        }
        #[allow(clippy::cast_possible_truncation)]
        Ok(v as f32)
    };
    Ok([n(0)?, n(2)?])
}

fn f32_field(v: &Value, key: &str, what: &str) -> Result<f32> {
    let n = v
        .get(key)
        .and_then(Value::as_f64)
        .with_context(|| format!("{what}: missing numeric `{key}`"))?;
    if !n.is_finite() {
        bail!("{what}: `{key}` is {n}");
    }
    #[allow(clippy::cast_possible_truncation)]
    Ok(n as f32)
}

fn id_field(v: &Value, what: &str) -> Result<String> {
    Ok(v.get("id")
        .and_then(Value::as_str)
        .with_context(|| format!("{what}: missing string `id`"))?
        .to_string())
}

/// A closed ring: lakes call it `polygon`, ponds call it `perimeter`
/// (`TBD_MapExportPonds.c:569`). The archive's contract is "first point is not repeated", so a
/// producer that closed the ring has its duplicate dropped.
fn ring_of(body: &Value, what: &str) -> Result<Vec<[f32; 2]>> {
    let raw = body
        .get("polygon")
        .or_else(|| body.get("perimeter"))
        .and_then(Value::as_array)
        .with_context(|| format!("{what}: missing `polygon`/`perimeter` array"))?;
    let mut ring = raw
        .iter()
        .map(|p| xz(p, what))
        .collect::<Result<Vec<_>>>()?;
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    if ring.len() < 3 {
        bail!(
            "{what}: a water body ring needs 3 distinct points, got {}",
            ring.len()
        );
    }
    Ok(ring)
}

fn bodies_of(root: &Value, key: &str) -> Result<Vec<WaterBody>> {
    let Some(list) = root.get(key).and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    list.iter()
        .enumerate()
        .map(|(i, b)| {
            let what = format!("{key}[{i}]");
            Ok(WaterBody {
                id: id_field(b, &what)?,
                surface_y: f32_field(b, "surfaceElevationYM", &what)?,
                ring: ring_of(b, &what)?,
            })
        })
        .collect()
}

/// Rivers: one centreline through the exported nodes, plus the river's own average width.
fn rivers_of(root: &Value) -> Result<Vec<WaterLine>> {
    let Some(list) = root.get("rivers").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    list.iter()
        .enumerate()
        .map(|(i, r)| {
            let what = format!("rivers[{i}]");
            let nodes = r
                .get("nodes")
                .and_then(Value::as_array)
                .with_context(|| format!("{what}: missing `nodes` array"))?;
            let centerline = nodes
                .iter()
                .enumerate()
                .map(|(j, n)| {
                    let p = n
                        .get("pos")
                        .with_context(|| format!("{what}.nodes[{j}]: missing `pos`"))?;
                    xz(p, &format!("{what}.nodes[{j}].pos"))
                })
                .collect::<Result<Vec<_>>>()?;
            if centerline.len() < 2 {
                bail!(
                    "{what}: a centreline needs 2 points, got {}",
                    centerline.len()
                );
            }
            Ok(WaterLine {
                id: id_field(r, &what)?,
                width_m: f32_field(r, "averageWidthM", &what)?,
                centerline,
            })
        })
        .collect()
}

/// Parse `TBD_InlandWaterExport_vectors.json` into the archive.
///
/// `ponds` comes from a `ponds` array when the export has one (`TBD_MapExportPonds`) and from
/// `inlandWaterBodies` otherwise — everon 2026-09-04 carries the latter, empty. Both are read
/// rather than one guessed at, because an empty lane a loader trusts is worse than a refusal.
///
/// # Errors
/// Malformed JSON, or a body/river the archive cannot represent (missing id, non-finite
/// coordinate, a ring under three points, a centreline under two).
pub fn build_water_vectors(json: &str) -> Result<WaterVectorsArchive> {
    let root: Value = serde_json::from_str(json).context("parse water vectors json")?;
    let mut ponds = bodies_of(&root, "ponds")?;
    ponds.extend(bodies_of(&root, "inlandWaterBodies")?);
    Ok(WaterVectorsArchive {
        schema_version: ARCHIVE_SCHEMA_VERSION,
        lakes: bodies_of(&root, "lakes")?,
        rivers: rivers_of(&root)?,
        ponds,
    })
}

/// Serialise, **re-read through the validating reader**, then write. Returns the file size.
///
/// The read-back is the emitter's last cheap chance to catch a file the SPA could never open:
/// `access_checked` is the only thing that will ever be used on it.
///
/// # Errors
/// Serialisation, validation or the write failing.
pub fn write_water_vectors(path: &Path, archive: &WaterVectorsArchive) -> Result<usize> {
    let bytes = to_bytes(archive).map_err(|e| anyhow::anyhow!("{e}"))?;
    access_checked::<WaterVectorsArchive>(&bytes)
        .map_err(|e| anyhow::anyhow!("emitted archive fails its own validating read: {e}"))?;
    create_parent(path)?;
    std::fs::write(path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(bytes.len())
}

/* ────────────────────────────────────── bathymetry ──────────────────────────────────── */

fn create_parent(path: &Path) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    }
    Ok(())
}

/// Stream one ASCII raster row by row, handing each row's parsed values to `row`.
///
/// The line buffer and the value buffer are both reused, so the whole 328 MB file costs one line's
/// worth of allocation. Refuses a row that is not exactly `width` values and a file that is not
/// exactly `height` rows — a short raster would otherwise emit a container whose header describes a
/// grid the payload does not contain.
fn stream_raster<F>(path: &Path, width: u32, height: u32, mut row: F) -> Result<()>
where
    F: FnMut(u32, &[u32]) -> Result<()>,
{
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut reader = std::io::BufReader::with_capacity(WRITE_BUF, file);
    let mut line = String::new();
    let mut values: Vec<u32> = Vec::with_capacity(width as usize);
    let mut z = 0_u32;
    loop {
        line.clear();
        if reader
            .read_line(&mut line)
            .with_context(|| format!("read {}", path.display()))?
            == 0
        {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        if z >= height {
            bail!(
                "{}: more than {height} rows — the meta and the raster disagree",
                path.display()
            );
        }
        values.clear();
        for tok in line.split_ascii_whitespace() {
            let v: u32 = tok.parse().with_context(|| {
                format!(
                    "{}: row {z} has a non-integer value {tok:?}",
                    path.display()
                )
            })?;
            values.push(v);
        }
        if values.len() != width as usize {
            bail!(
                "{}: row {z} has {} values, expected {width}",
                path.display(),
                values.len()
            );
        }
        row(z, &values)?;
        z += 1;
    }
    if z != height {
        bail!("{}: {z} rows, expected {height}", path.display());
    }
    Ok(())
}

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
                out.depth[dst] = reduce_depth(out.depth[dst], self.depth[src]);
                out.mask[dst] = reduce_mask(out.mask[dst], self.mask[src]);
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
        pad4(out, self.depth.len() * 2 + self.mask.len())
    }
}

/// Zero-pad to the next multiple of 4 and return the level's whole stride.
fn pad4(out: &mut impl Write, written: usize) -> Result<usize> {
    let stride = written.next_multiple_of(4);
    out.write_all(&vec![0_u8; stride - written])?;
    Ok(stride)
}

/// Write `water/bathymetry.tbd-bath` from the two staging rasters. Returns the file size.
///
/// Level 0 goes straight from the `.txt` files to the output — the depth raster in one pass, the
/// mask raster in a second — while level 1 is folded in the same two passes. Levels 2 and up are
/// folded from their predecessor. See the module docs for why that is the shape.
///
/// # Errors
/// A raster that disagrees with the meta about its dimensions, a depth value that does not fit the
/// container's `u16`, an I/O failure, or a payload whose length disagrees with what the header's
/// own [`TbdbHeader::level_span`] ladder implies (the emitter's self-check against the reader).
pub fn write_bathymetry(
    out_path: &Path,
    meta: &RasterMeta,
    depth_txt: &Path,
    mask_txt: &Path,
) -> Result<u64> {
    let (w, h) = (meta.width, meta.height);
    let mips = mip_count(w, h);
    let header = TbdbHeader::new(w, h, mips, meta.depth_scale);
    create_parent(out_path)?;
    let file = std::fs::File::create(out_path)
        .with_context(|| format!("create {}", out_path.display()))?;
    let mut out = BufWriter::with_capacity(WRITE_BUF, file);
    out.write_all(&header.to_header_bytes())?;

    let (nw, nh) = ((w as usize / 2).max(1), (h as usize / 2).max(1));
    let mut next = Level {
        width: nw,
        height: nh,
        depth: vec![0; nw * nh],
        mask: vec![0; nw * nh],
    };

    // Pass 1 — level 0 depth, straight through; level 1 depth folded alongside.
    let mut row_bytes: Vec<u8> = Vec::with_capacity(w as usize * 2);
    stream_raster(depth_txt, w, h, |z, values| {
        #[allow(clippy::cast_possible_truncation)]
        let dz = downsample_index(z, nh as u32) as usize;
        row_bytes.clear();
        for (x, &v) in values.iter().enumerate() {
            let d = u16::try_from(v).with_context(|| {
                format!(
                    "{}: depth {v} at ({x}, {z}) does not fit the container's u16",
                    depth_txt.display()
                )
            })?;
            row_bytes.extend_from_slice(&d.to_le_bytes());
            #[allow(clippy::cast_possible_truncation)]
            let dx = downsample_index(x as u32, nw as u32) as usize;
            let dst = dz * nw + dx;
            next.depth[dst] = reduce_depth(next.depth[dst], d);
        }
        out.write_all(&row_bytes)?;
        Ok(())
    })?;

    // Pass 2 — level 0 mask, class byte collapsed to a boolean; level 1 mask folded alongside.
    let mut row_mask: Vec<u8> = Vec::with_capacity(w as usize);
    stream_raster(mask_txt, w, h, |z, values| {
        #[allow(clippy::cast_possible_truncation)]
        let dz = downsample_index(z, nh as u32) as usize;
        row_mask.clear();
        for (x, &v) in values.iter().enumerate() {
            let m = u8::from(v != 0);
            row_mask.push(m);
            #[allow(clippy::cast_possible_truncation)]
            let dx = downsample_index(x as u32, nw as u32) as usize;
            let dst = dz * nw + dx;
            next.mask[dst] = reduce_mask(next.mask[dst], m);
        }
        out.write_all(&row_mask)?;
        Ok(())
    })?;

    let texels = w as usize * h as usize;
    let mut payload = pad4(&mut out, texels * 2 + texels)?;
    let mut level = next;
    for _ in 1..mips {
        payload += level.write_to(&mut out)?;
        level = level.fold();
    }
    out.flush().context("flush bathymetry")?;
    drop(out);

    // The emitter's self-check against the reader: the payload it just wrote must be exactly the
    // one `TbdbHeader::level_span` will compute when the SPA reads this file back. A `None` from
    // the ladder is an overflow, which is why it is not unwrapped.
    let mut implied = 0_usize;
    for l in 0..mips {
        let span = header
            .level_span(l)
            .with_context(|| format!("level {l} span overflows on this host"))?;
        implied = implied
            .checked_add(span.stride)
            .context("bathymetry payload length overflows usize")?;
    }
    if implied != payload {
        bail!("bathymetry payload is {payload} B but the header implies {implied} B");
    }
    let size = std::fs::metadata(out_path)
        .with_context(|| format!("stat {}", out_path.display()))?
        .len();
    Ok(size)
}

/* ─────────────────────────────────────── the CLI ────────────────────────────────────── */

/// Resolve `--terrain`: a directory path when it names one, else a terrain id under
/// `packages/map-assets`.
#[must_use]
pub fn terrain_dir(terrain: &str) -> PathBuf {
    let as_path = PathBuf::from(terrain);
    if as_path.is_dir() {
        return as_path;
    }
    repo_root().join("packages/map-assets").join(terrain)
}

/// `map water --terrain <id|dir>` — both water binaries from one staging directory.
///
/// # Errors
/// Any staging file being missing or malformed, or either write failing. Returns exit code 1
/// (rather than an error) when the terrain or staging directory is simply absent, matching the
/// rest of the `map` lane.
pub fn emit_water(terrain: &str) -> Result<u8> {
    let dir = terrain_dir(terrain);
    if !dir.is_dir() {
        eprintln!("water: no terrain directory at {}", dir.display());
        return Ok(1);
    }
    let staging = dir.join(STAGING_WATER);
    if !staging.is_dir() {
        eprintln!(
            "water: no staging export at {} — run the Workbench inland-water exporter first",
            staging.display()
        );
        return Ok(1);
    }

    let vectors_src = staging.join(VECTORS_JSON);
    let archive = build_water_vectors(
        &std::fs::read_to_string(&vectors_src)
            .with_context(|| format!("read {}", vectors_src.display()))?,
    )?;
    super::refuse_empty_write(
        "water",
        archive.lakes.is_empty() && archive.rivers.is_empty() && archive.ponds.is_empty(),
        "no lakes, rivers or ponds — refusing to write an empty water_vectors.rkyv",
    )?;
    let vectors_out = dir.join(WATER_VECTORS_RKYV);
    let vectors_bytes = write_water_vectors(&vectors_out, &archive)?;

    let meta_src = staging.join(META_JSON);
    let meta = parse_meta(
        &std::fs::read_to_string(&meta_src)
            .with_context(|| format!("read {}", meta_src.display()))?,
    )?;
    let bath_out = dir.join(BATHYMETRY_TBDB);
    let bath_bytes = write_bathymetry(
        &bath_out,
        &meta,
        &staging.join(DEPTH_TXT),
        &staging.join(MASK_TXT),
    )?;

    println!(
        "water: {} lakes + {} rivers + {} ponds → {} ({vectors_bytes} bytes)",
        archive.lakes.len(),
        archive.rivers.len(),
        archive.ponds.len(),
        vectors_out.display()
    );
    println!(
        "water: {}×{} × {} mips @ {} m/unit → {} ({bath_bytes} bytes)",
        meta.width,
        meta.height,
        mip_count(meta.width, meta.height),
        meta.depth_scale,
        bath_out.display()
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use map_engine_core::world::{Bathymetry, WaterAt, WaterMask, WaterVectors};

    use super::*;

    /// A 4×4 grid with three wet texels at three different depths, so `max` is distinguishable
    /// from `first`, `last` and `any`, and so no level folds to a uniform answer.
    const DEPTH_4X4: &str = "7 0 0 0\n0 0 0 0\n0 30 0 0\n0 0 0 12\n";
    const MASK_4X4: &str = "2 0 0 0\n0 0 0 0\n0 3 0 0\n0 0 0 2\n";
    const SCALE: f32 = 0.1;

    fn meta_json(w: u32, h: u32) -> String {
        format!("{{\"widthPx\":{w},\"heightPx\":{h},\"depthScaleToMeters\":0.1}}")
    }

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!(
                "tbd-t935-9-{tag}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            std::fs::create_dir_all(p.join(STAGING_WATER)).expect("mkdir scratch");
            Self(p)
        }

        /// Write the four staging files, then run the whole `emit_water` CLI path over them.
        fn stage(&self, depth: &str, mask: &str, meta: &str, vectors: &str) -> &Path {
            let s = self.0.join(STAGING_WATER);
            std::fs::write(s.join(DEPTH_TXT), depth).expect("depth");
            std::fs::write(s.join(MASK_TXT), mask).expect("mask");
            std::fs::write(s.join(META_JSON), meta).expect("meta");
            std::fs::write(s.join(VECTORS_JSON), vectors).expect("vectors");
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).ok();
        }
    }

    fn vectors_json() -> String {
        r#"{
          "lakes": [{
            "id": "lake_1", "name": "Lake 1", "surfaceElevationYM": 84.762,
            "polygon": [[0,84.762,0],[4,84.762,0],[4,84.762,4],[0,84.762,4],[0,84.762,0]]
          }],
          "rivers": [{
            "id": "river_1", "name": "River 1", "averageWidthM": 12.5,
            "nodes": [
              {"pos": [0,1.5,1], "widthM": 12.5},
              {"pos": [2,1.5,1], "widthM": 12.5},
              {"pos": [4,1.5,1], "widthM": 12.5}
            ]
          }],
          "ponds": [{
            "id": "pond_1", "surfaceElevationYM": 3.25,
            "perimeter": [[1,3.25,1],[2,3.25,1],[2,3.25,2]]
          }],
          "inlandWaterBodies": []
        }"#
        .to_string()
    }

    fn bathymetry_of(dir: &Path) -> Bathymetry {
        let raw = std::fs::read(dir.join(BATHYMETRY_TBDB)).expect("read bath");
        Bathymetry::from_bytes(&raw).expect("parse emitted bathymetry")
    }

    #[test]
    fn mip_count_halves_to_one_texel() {
        assert_eq!(mip_count(1, 1), 1);
        assert_eq!(mip_count(2, 2), 2);
        assert_eq!(mip_count(4, 4), 3);
        assert_eq!(mip_count(5, 5), 3);
        assert_eq!(mip_count(12_800, 12_800), 14);
        // The coarsest level of every case above must actually be 1×1 on the longer side.
        for (w, h) in [(1_u32, 1_u32), (4, 4), (5, 3), (12_800, 12_800), (1, 9)] {
            let last = mip_count(w, h) - 1;
            assert_eq!(
                ((w >> u32::from(last)).max(1)).max((h >> u32::from(last)).max(1)),
                1,
                "{w}x{h} does not reach one texel at level {last}"
            );
        }
    }

    /// THE SLICE'S PIN. The emitted pyramid's every level agrees with its own level 0: a coarse
    /// texel is water iff ANY level-0 texel folding into it is, and its depth is that block's MAX.
    /// Read back through the CORE reader, so emitter and loader are pinned to each other rather
    /// than to a private copy of the rule.
    #[test]
    fn every_emitted_mip_level_agrees_with_level_zero() {
        let s = Scratch::new("mips");
        let dir = s.stage(DEPTH_4X4, MASK_4X4, &meta_json(4, 4), &vectors_json());
        assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
        let b = bathymetry_of(dir);
        assert_eq!((b.width(), b.height(), b.level_count()), (4, 4, 3));
        let base = b.level(0).expect("level 0");
        assert_eq!(
            base.depth,
            &[7, 0, 0, 0, 0, 0, 0, 0, 0, 30, 0, 0, 0, 0, 0, 12]
        );
        assert_eq!(base.mask, &[1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1]);

        for level in 1..b.level_count() {
            let grid = b.level(level).expect("level");
            let n = (grid.width * grid.height) as usize;
            let (mut want_mask, mut want_depth) = (vec![0_u8; n], vec![0_u16; n]);
            for z in 0..base.height {
                for x in 0..base.width {
                    let (mut tx, mut tz) = (x, z);
                    for l in 1..=level {
                        let (lw, lh) = b.header().level_dims(l).expect("dims");
                        tx = downsample_index(tx, lw);
                        tz = downsample_index(tz, lh);
                    }
                    let src = base.index(x, z).expect("src");
                    let dst = grid.index(tx, tz).expect("dst");
                    want_mask[dst] |= u8::from(base.mask[src] != 0);
                    want_depth[dst] = want_depth[dst].max(base.depth[src]);
                }
            }
            assert_eq!(grid.mask, &want_mask[..], "level {level} mask");
            assert_eq!(grid.depth, &want_depth[..], "level {level} depth");
            assert!(
                want_mask.contains(&1) && want_depth.iter().any(|&d| d > 0),
                "level {level} oracle is vacuous"
            );
        }
        // The coarsest level is one texel and it is wet — three separate lakes cannot summarise
        // to dry ground.
        let top = b.level(b.level_count() - 1).expect("top");
        assert_eq!((top.width, top.height), (1, 1));
        assert_eq!((top.mask[0], top.depth[0]), (1, 30));
    }

    /// The container the emitter writes answers the placement question the SPA will ask of it —
    /// end to end, through `WaterMask`, on the real file bytes.
    #[test]
    fn the_emitted_container_answers_the_placement_query() {
        let s = Scratch::new("query");
        let dir = s.stage(DEPTH_4X4, MASK_4X4, &meta_json(4, 4), &vectors_json());
        assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
        let raw = std::fs::read(dir.join(BATHYMETRY_TBDB)).expect("read");
        let m = WaterMask::from_bytes(&raw, [0.0, 0.0, 3.0, 3.0]).expect("mask");
        // 4 samples over [0, 3] → 1 m apart; the wet texel (1, 2) is world (1, 2).
        assert_eq!(m.sample(1.0, 2.0), WaterAt::Water { depth_m: 3.0 });
        assert!(m.is_water(1.0, 2.0));
        assert!(!m.is_known_dry_land(1.0, 2.0));
        assert_eq!(m.sample(2.0, 1.0), WaterAt::Dry);
        assert!(m.is_known_dry_land(2.0, 1.0));
        assert_eq!(m.depth_m(0.0, 0.0), Some(f32::from(7_u8) * SCALE));
        // Off the map: no reading, and above all not "known dry land".
        assert_eq!(m.sample(-1.0, 2.0), WaterAt::Unknown);
        assert!(!m.is_water(-1.0, 2.0) && !m.is_known_dry_land(-1.0, 2.0));
        assert_eq!(m.depth_m(99.0, 99.0), None);
    }

    #[test]
    fn the_vectors_archive_carries_every_lane_with_its_surface_height() {
        let s = Scratch::new("vectors");
        let dir = s.stage(DEPTH_4X4, MASK_4X4, &meta_json(4, 4), &vectors_json());
        assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
        let raw = std::fs::read(dir.join(WATER_VECTORS_RKYV)).expect("read vectors");
        let v = WaterVectors::from_bytes(&raw).expect("parse through the core reader");
        assert_eq!(v.counts().expect("counts"), (1, 1, 1));
        let a = v.archive().expect("archive");
        assert_eq!(a.lakes[0].id.as_str(), "lake_1");
        assert_eq!(a.lakes[0].surface_y.to_native(), 84.762);
        // The closing point was dropped: 5 authored, 4 stored.
        assert_eq!(a.lakes[0].ring.len(), 4);
        assert_eq!(a.lakes[0].ring[1][0].to_native(), 4.0);
        // `[x, y, z]` → `[x, z]`: the ring must carry the PLAN, never the elevation.
        assert_eq!(a.lakes[0].ring[2][1].to_native(), 4.0);
        assert_eq!(a.rivers[0].width_m.to_native(), 12.5);
        assert_eq!(a.rivers[0].centerline.len(), 3);
        assert_eq!(a.rivers[0].centerline[1][1].to_native(), 1.0);
        // The pond lane fires: `perimeter` is read as a ring, not silently dropped.
        assert_eq!(a.ponds[0].id.as_str(), "pond_1");
        assert_eq!(a.ponds[0].surface_y.to_native(), 3.25);
        assert_eq!(a.ponds[0].ring.len(), 3);
    }

    #[test]
    fn bytes_are_deterministic_across_runs() {
        let mut runs = Vec::new();
        for tag in ["det-a", "det-b"] {
            let s = Scratch::new(tag);
            let dir = s.stage(DEPTH_4X4, MASK_4X4, &meta_json(4, 4), &vectors_json());
            assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
            runs.push((
                std::fs::read(dir.join(BATHYMETRY_TBDB)).expect("bath"),
                std::fs::read(dir.join(WATER_VECTORS_RKYV)).expect("vectors"),
            ));
        }
        assert_eq!(
            runs[0].0, runs[1].0,
            "bathymetry bytes are not deterministic"
        );
        assert_eq!(runs[0].1, runs[1].1, "vector bytes are not deterministic");
        // Every level is padded to 4, so the payload is too — and the file with it.
        assert!(runs[0].0.len().is_multiple_of(4));
    }

    /// Odd dimensions: 5 → 2 → 1. The last fold has to absorb a third source column, and the
    /// emitted level must still be the size the header's ladder claims.
    #[test]
    fn odd_dimensions_emit_a_consistent_pyramid() {
        let depth: String = (0..5)
            .map(|z| {
                (0..5)
                    .map(|x| if (x, z) == (4, 4) { "9" } else { "0" })
                    .collect::<Vec<_>>()
                    .join(" ")
                    + "\n"
            })
            .collect();
        let mask: String = depth.replace('9', "2");
        let s = Scratch::new("odd");
        let dir = s.stage(&depth, &mask, &meta_json(5, 5), &vectors_json());
        assert_eq!(emit_water(dir.to_str().expect("utf8")).expect("emit"), 0);
        let b = bathymetry_of(dir);
        assert_eq!(b.level_count(), 3);
        for level in 0..3 {
            let g = b.level(level).expect("level");
            let want = (5_u32 >> u32::from(level)).max(1);
            assert_eq!((g.width, g.height), (want, want), "level {level}");
            assert!(
                g.mask.iter().any(|&m| m != 0) && g.depth.contains(&9),
                "level {level} lost the corner lake the odd tail clamps into"
            );
        }
    }

    #[test]
    fn a_raster_that_disagrees_with_the_meta_is_refused() {
        let s = Scratch::new("short");
        // Three rows where the meta says four.
        let dir = s.stage(
            "0 0 0 0\n0 0 0 0\n0 0 0 0\n",
            MASK_4X4,
            &meta_json(4, 4),
            &vectors_json(),
        );
        let err = emit_water(dir.to_str().expect("utf8")).expect_err("must refuse");
        assert!(format!("{err:#}").contains("3 rows, expected 4"), "{err:#}");

        let s2 = Scratch::new("wide");
        let dir2 = s2.stage(
            DEPTH_4X4,
            "2 0 0 0 0\n0 0 0 0\n0 0 0 0\n0 0 0 0\n",
            &meta_json(4, 4),
            &vectors_json(),
        );
        let err2 = emit_water(dir2.to_str().expect("utf8")).expect_err("must refuse");
        assert!(
            format!("{err2:#}").contains("5 values, expected 4"),
            "{err2:#}"
        );
    }

    #[test]
    fn a_depth_that_does_not_fit_the_container_is_refused_not_truncated() {
        let s = Scratch::new("huge");
        let dir = s.stage(
            "65536 0 0 0\n0 0 0 0\n0 0 0 0\n0 0 0 0\n",
            MASK_4X4,
            &meta_json(4, 4),
            &vectors_json(),
        );
        let err = emit_water(dir.to_str().expect("utf8")).expect_err("must refuse");
        assert!(format!("{err:#}").contains("does not fit"), "{err:#}");
    }

    #[test]
    fn a_meta_that_cannot_describe_a_grid_is_refused() {
        for bad in [
            r#"{"widthPx":0,"heightPx":4,"depthScaleToMeters":0.1}"#,
            r#"{"widthPx":4,"heightPx":0,"depthScaleToMeters":0.1}"#,
            r#"{"widthPx":4,"heightPx":4,"depthScaleToMeters":0}"#,
            r#"{"widthPx":4,"heightPx":4}"#,
            r#"{"heightPx":4,"depthScaleToMeters":0.1}"#,
        ] {
            assert!(parse_meta(bad).is_err(), "{bad}");
        }
        assert_eq!(
            parse_meta(r#"{"widthPx":12800,"heightPx":12800,"depthScaleToMeters":0.1}"#)
                .expect("everon meta"),
            RasterMeta {
                width: 12_800,
                height: 12_800,
                depth_scale: SCALE
            }
        );
    }

    #[test]
    fn a_vector_export_the_archive_cannot_represent_is_refused() {
        for (bad, needle) in [
            (
                r#"{"lakes":[{"surfaceElevationYM":1,"polygon":[[0,0,0],[1,0,0],[1,0,1]]}]}"#,
                "id",
            ),
            (
                r#"{"lakes":[{"id":"l","polygon":[[0,0,0],[1,0,0],[1,0,1]]}]}"#,
                "surfaceElevationYM",
            ),
            (
                r#"{"lakes":[{"id":"l","surfaceElevationYM":1,"polygon":[[0,0,0],[1,0,0]]}]}"#,
                "3 distinct points",
            ),
            (
                r#"{"lakes":[{"id":"l","surfaceElevationYM":1,"polygon":[[0,0],[1,0],[1,1]]}]}"#,
                "[x, y, z]",
            ),
            (
                r#"{"rivers":[{"id":"r","averageWidthM":1,"nodes":[{"pos":[0,0,0]}]}]}"#,
                "2 points",
            ),
            (r#"{"rivers":[{"id":"r","averageWidthM":1}]}"#, "nodes"),
        ] {
            let err = build_water_vectors(bad).expect_err(bad);
            assert!(format!("{err:#}").contains(needle), "{bad} → {err:#}");
        }
    }

    #[test]
    fn an_export_with_no_water_at_all_is_refused() {
        let s = Scratch::new("empty");
        let dir = s.stage(
            DEPTH_4X4,
            MASK_4X4,
            &meta_json(4, 4),
            r#"{"lakes":[],"rivers":[]}"#,
        );
        let err = emit_water(dir.to_str().expect("utf8")).expect_err("must refuse");
        assert!(
            format!("{err:#}").contains("refusing empty write"),
            "{err:#}"
        );
        assert!(
            !dir.join(WATER_VECTORS_RKYV).exists(),
            "nothing may be written"
        );
    }

    #[test]
    fn a_missing_terrain_or_staging_directory_exits_one() {
        assert_eq!(emit_water("/nonexistent/terrain/xyzzy").expect("no dir"), 1);
        let s = Scratch::new("nostaging");
        std::fs::remove_dir_all(s.0.join(STAGING_WATER)).expect("drop staging");
        assert_eq!(
            emit_water(s.0.to_str().expect("utf8")).expect("no staging"),
            1
        );
    }

    #[test]
    fn terrain_dir_takes_an_id_or_a_directory() {
        assert_eq!(
            terrain_dir("everon"),
            repo_root().join("packages/map-assets/everon")
        );
        let here = repo_root();
        assert_eq!(terrain_dir(here.to_str().expect("utf8")), here);
    }
}
