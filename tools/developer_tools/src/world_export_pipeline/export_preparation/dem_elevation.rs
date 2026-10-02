use super::*;

/// Write `dem/elevation.dem`: a `TBDE` header (spec §3.2) then `width * height` `u16`
/// samples little-endian, row-major, row 0 = north edge.
///
/// The samples are written **verbatim** — the same quantised values the 16-bit PNG carries — so the
/// two files decode to the identical grid and only the container differs. `min_m` / `max_m` are the
/// encoding range the plugin quantised against; the header stores them as `scale_m =
/// (max - min) / 65535` and `offset_m = min`, both `f32` (spec §3.2), which is where the only
/// precision difference from the manifest's `f64` height range comes from.
///
/// # Errors
/// When the grid does not match `width * height`, when those overflow `usize`, or on write failure.
pub fn write_elevation_dem(
    path: &Path,
    width: u32,
    height: u32,
    min_m: f64,
    max_m: f64,
    samples: &[u16],
) -> Result<()> {
    // `as f32` here and nowhere else: the format stores the range in f32, so the narrowing is
    // named at the boundary instead of hiding inside the header constructor's caller.
    #[allow(clippy::cast_possible_truncation)]
    let header = TbdeHeader::new(width, height, min_m as f32, max_m as f32);
    let expected = header
        .sample_count()
        .with_context(|| format!("TBDE {width}x{height} overflows usize"))?;
    anyhow::ensure!(
        samples.len() == expected,
        "TBDE {width}x{height} needs {expected} samples, got {}",
        samples.len()
    );
    let bytes = dem_raw::to_bytes(&header, samples);
    std::fs::write(path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn raw_u16_to_dem_png(raster_path: &Path, meta_path: &Path, out_path: &Path) -> Result<u8> {
    let meta: Value = serde_json::from_str(&std::fs::read_to_string(meta_path)?)?;
    let (w, h) = (
        meta["widthPx"].as_u64().unwrap_or(0) as usize,
        meta["heightPx"].as_u64().unwrap_or(0) as usize,
    );
    if w == 0 || h == 0 {
        eprintln!("Bad meta dims {w}x{h}");
        return Ok(1);
    }
    println!("Parsing raster {} ({w}x{h})...", raster_path.display());
    let buf = std::fs::read(raster_path)?;
    let mut raster = vec![0u16; w * h];
    let mut idx = 0usize;
    let mut cur = 0u32;
    let mut in_num = false;
    for &c in &buf {
        if c.is_ascii_digit() {
            cur = cur * 10 + u32::from(c - b'0');
            in_num = true;
        } else if in_num {
            raster[idx] = cur as u16;
            idx += 1;
            cur = 0;
            in_num = false;
        }
    }
    if in_num {
        raster[idx] = cur as u16;
        idx += 1;
    }
    if idx != w * h {
        eprintln!("FAIL parsed {idx} values, expected {}", w * h);
        return Ok(1);
    }
    let (u_min, u_max) = raster
        .iter()
        .fold((65535u16, 0u16), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    println!("Parsed {idx} samples; u16 range [{u_min}, {u_max}]");

    println!("Deflating IDAT...");
    let mut be = Vec::with_capacity(w * h * 2);
    for v in &raster {
        be.extend_from_slice(&v.to_be_bytes());
    }
    {
        let file = std::fs::File::create(out_path)?;
        let wtr = std::io::BufWriter::new(file);
        let mut enc = png::Encoder::new(wtr, w as u32, h as u32);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Sixteen);
        enc.set_compression(png::Compression::Best);
        enc.write_header()?.write_image_data(&be)?;
    }
    let png_len = std::fs::metadata(out_path)?.len();
    println!("Wrote {} ({png_len} bytes)", out_path.display());

    // Self-check: decode back, verify IHDR + 3 round-trip pixels.
    let dec = png::Decoder::new(std::fs::File::open(out_path)?);
    let mut reader = dec.read_info().context("png read_info")?;
    let info = reader.info();
    if info.bit_depth != png::BitDepth::Sixteen
        || info.color_type != png::ColorType::Grayscale
        || info.width as usize != w
        || info.height as usize != h
    {
        eprintln!(
            "FAIL IHDR check: depth={:?} colorType={:?} {}x{}",
            info.bit_depth, info.color_type, info.width, info.height
        );
        return Ok(1);
    }
    let mut data = vec![0u8; reader.output_buffer_size()];
    reader.next_frame(&mut data)?;
    for (x, y) in [(0usize, 0usize), (w - 1, h - 1), (w >> 1, h >> 1)] {
        let off = (y * w + x) * 2;
        let got = u16::from_be_bytes([data[off], data[off + 1]]);
        let want = raster[y * w + x];
        if got != want {
            eprintln!("FAIL round-trip ({x},{y}): got {got} want {want}");
            return Ok(1);
        }
    }
    println!("OK  IHDR bitDepth=16 colorType=0 dims match; round-trip pixels OK");

    // Dual emission (spec §7 wave 2): the same `raster` also goes out as
    // `dem/elevation.dem` beside the PNG. Both are written every run until the manifest names the
    // manifest — the loader picks by `manifest.dem.raw`, so deleting either write before then
    // blinds one reader. Nothing above this line changed.
    let dem_path = out_path.with_file_name("elevation.dem");
    let (min_m, max_m) = (
        meta["heightRangeMinM"]
            .as_f64()
            .unwrap_or(DEM_DEFAULT_MIN_M),
        meta["heightRangeMaxM"]
            .as_f64()
            .unwrap_or(DEM_DEFAULT_MAX_M),
    );
    write_elevation_dem(&dem_path, w as u32, h as u32, min_m, max_m, &raster)?;
    let dem_len = std::fs::metadata(&dem_path)?.len();
    println!(
        "Wrote {} ({dem_len} bytes; TBDE {w}x{h} u16 LE, range [{min_m}, {max_m}] m)",
        dem_path.display()
    );
    Ok(0)
}
