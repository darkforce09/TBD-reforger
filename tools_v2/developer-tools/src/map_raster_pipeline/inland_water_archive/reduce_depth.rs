use super::*;

use crate::repository_layout;

/// Spec §3.3: a coarse texel keeps the **deepest** depth of the block it covers.
///
/// Conservative in the same direction as [`reduce_mask`] — a coarse level never reports shallower
/// water than the fine one it summarises.
pub(super) fn reduce_depth(acc: u16, v: u16) -> u16 {
    acc.max(v)
}

/// Spec §3.3: a coarse texel is water if **any** texel of the block it covers is.
///
/// This is the polarity the whole pyramid rests on. Invert it and every level above 0 marks the
/// lakes dry and the fields wet, which a placement guard reads as permission — the
/// `every_mip_level_agrees_with_level_zero` pin exists for exactly this line.
pub(super) fn reduce_mask(acc: u8, v: u8) -> u8 {
    acc | u8::from(v != 0)
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

/// `[x, y, z]` → `[x, z]`, the horizontal plane the ring lives in. `y` is carried once per body as
/// `surface_y`, not per point.
pub(super) fn xz(p: &Value, what: &str) -> Result<[f32; 2]> {
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

pub(super) fn f32_field(v: &Value, key: &str, what: &str) -> Result<f32> {
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

pub(super) fn id_field(v: &Value, what: &str) -> Result<String> {
    Ok(v.get("id")
        .and_then(Value::as_str)
        .with_context(|| format!("{what}: missing string `id`"))?
        .to_string())
}

/// A closed ring: lakes call it `polygon`, ponds call it `perimeter`
/// (`TBD_MapExportPonds.c:569`). The archive's contract is "first point is not repeated", so a
/// producer that closed the ring has its duplicate dropped.
pub(super) fn ring_of(body: &Value, what: &str) -> Result<Vec<[f32; 2]>> {
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

pub(super) fn bodies_of(root: &Value, key: &str) -> Result<Vec<WaterBody>> {
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
pub(super) fn rivers_of(root: &Value) -> Result<Vec<WaterLine>> {
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

pub(super) fn create_parent(path: &Path) -> Result<()> {
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
pub(super) fn stream_raster<F>(path: &Path, width: u32, height: u32, mut row: F) -> Result<()>
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

/// Zero-pad to the next multiple of 4 and return the level's whole stride.
pub(super) fn pad4(out: &mut impl Write, written: usize) -> Result<usize> {
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

/// Resolve `--terrain`: a directory path when it names one, else a terrain id under
/// `packages/map-assets`.
#[must_use]
pub fn terrain_dir(terrain: &str) -> PathBuf {
    let as_path = PathBuf::from(terrain);
    if as_path.is_dir() {
        return as_path;
    }
    repository_layout::terrain_dir(&repo_root(), terrain)
}
