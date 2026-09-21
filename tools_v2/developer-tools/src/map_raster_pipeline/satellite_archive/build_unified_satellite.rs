use super::*;

pub fn build_unified_satellite(
    input: &Path,
    out: &Path,
    terrain: &str,
    tile_threshold: usize,
    container_version: u16,
) -> Result<u8> {
    use sha2::Digest as _;
    if container_version != 1 && container_version != 2 {
        eprintln!("unsupported --container-version {container_version} (expected 1 or 2)");
        return Ok(1);
    }
    let world_bounds: [u64; 4] = match terrain {
        "everon" => [0, 0, 12800, 12800],
        "arland" => [0, 0, 4096, 4096],
        _ => {
            eprintln!("unknown terrain \"{terrain}\" (add its worldBounds to the unified builder)");
            return Ok(1);
        }
    };
    if !input.exists() {
        eprintln!("input not found: {}", input.display());
        return Ok(1);
    }
    let t0 = std::time::Instant::now();
    let log = |m: &str| println!("[tbd-sat] {m}");

    log(&format!("normalizing {}", input.display()));
    let base = image_operations::load_png_rgb(input)?;
    let (src_w, src_h) = (base.w, base.h);
    log(&format!(
        "source {src_w}x{src_h}; tileThreshold={tile_threshold}"
    ));

    // Mip chain dims: base → 1×1 with the GL rule.
    let dims = mip_dims(src_w, src_h);
    log(&format!("mip chain: {} levels ({src_w} → 1)", dims.len()));

    // Cascade-halve + tile + encode (rayon-free: encode sequentially — image-webp lossless
    // is fast enough for the rebuild-smoke acceptance; parallelism can come later).
    let mut blocks: Vec<TileBuf> = Vec::new();
    let mut level_meta = Vec::new();
    let mut current = base;
    for (level, &(lw, lh)) in dims.iter().enumerate() {
        if level > 0 {
            current = image_operations::resize_rgb(&current, lw, lh);
        }
        let cols = lw.div_ceil(tile_threshold);
        let rows = lh.div_ceil(tile_threshold);
        let tile_w = lw.div_ceil(cols);
        let tile_h = lh.div_ceil(rows);
        for gy in 0..rows {
            for gx in 0..cols {
                let x = gx * tile_w;
                let y = gy * tile_h;
                let tw = tile_w.min(lw - x);
                let th = tile_h.min(lh - y);
                let tile = if cols == 1 && rows == 1 {
                    current.data.clone()
                } else {
                    image_operations::crop_rgb(&current, x, y, tw, th)?.data
                };
                let buf = image_operations::encode_webp_lossless_rgb(&image_operations::Rgb8 {
                    w: tw,
                    h: th,
                    data: tile,
                })?;
                blocks.push(TileBuf {
                    level,
                    x,
                    y,
                    w: tw,
                    h: th,
                    buf,
                });
            }
        }
        level_meta.push((lw, lh));
    }
    log(&format!("encoded {} VP8L blocks", blocks.len()));

    let input_sha256 = {
        let mut hsh = sha2::Sha256::new();
        hsh.update(std::fs::read(input)?);
        hsh.finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let mut source_meta = Value::Null;
    let meta_path = input
        .parent()
        .unwrap_or(Path::new("."))
        .join("TBD_SatExport_meta.json");
    if meta_path.exists() {
        let m: Value = serde_json::from_str(&std::fs::read_to_string(&meta_path)?)?;
        let mut sm = serde_json::Map::new();
        sm.insert("source".into(), m["source"].clone());
        sm.insert("seamRepair".into(), m["seamRepair"].clone());
        sm.insert("generatedAt".into(), m["generatedAt"].clone());
        if m["waterComposite"].is_object() {
            let wc = &m["waterComposite"];
            sm.insert(
                "waterComposite".into(),
                json!({
                    "lane": wc["lane"], "oceanMaskSource": wc["oceanMaskSource"],
                    "inlandMaskSource": wc["inlandMaskSource"], "generatedAt": wc["generatedAt"],
                }),
            );
        }
        source_meta = Value::Object(sm);
    }

    let file = if container_version == 2 {
        let index = tbds_v2_index(&blocks, &level_meta, (src_w, src_h), tile_threshold)?;
        tbds_v2_bytes(&index, &blocks)?
    } else {
        build_tbds_v1_bytes(
            &blocks,
            &level_meta,
            (src_w, src_h),
            terrain,
            world_bounds,
            &source_meta,
            &input_sha256,
        )?
    };
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, &file)?;

    // Driven by the encoded blocks rather than by either index, so the receipt says the same thing
    // for both container versions.
    for (level, &(lw, _)) in level_meta.iter().enumerate() {
        let (n, bytes) = blocks
            .iter()
            .filter(|b| b.level == level)
            .fold((0usize, 0u64), |(n, bytes), b| {
                (n + 1, bytes + b.buf.len() as u64)
            });
        log(&format!(
            "  level {level:>2}  {lw:>5}px  {n} block(s)  {:.2} MB",
            bytes as f64 / 1e6
        ));
    }
    log(&format!(
        "wrote {}  {:.1} MB in {:.0}s (tbd-sat v{container_version})",
        out.display(),
        file.len() as f64 / 1e6,
        t0.elapsed().as_secs_f64()
    ));
    log("manifest block:");
    let base_name = out.file_name().unwrap_or_default().to_string_lossy();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "delivery": "unified",
            "unified": {
                "path": format!("satellite/{base_name}"),
                "url": format!("/map-assets/{terrain}/satellite/{base_name}"),
                "encoding": format!("tbd-sat-v{container_version}"),
                "baseWidthPx": src_w,
                "baseHeightPx": src_h,
                "mipCount": dims.len(),
                "bytes": file.len(),
            },
        }))?
    );
    Ok(0)
}

/// The v1 container bytes: `"TBDS"`, formatVersion 1, jsonLength, the hand-packed JSON table, then
/// the payload. Retained verbatim behind `--container-version 1` because `everon-sat.tbd-sat` is
/// committed in this shape and stays that way until T-935.13 regenerates it.
pub(crate) fn build_tbds_v1_bytes(
    blocks: &[TileBuf],
    level_meta: &[(usize, usize)],
    base: (usize, usize),
    terrain: &str,
    world_bounds: [u64; 4],
    source_meta: &Value,
    input_sha256: &str,
) -> Result<Vec<u8>> {
    let (src_w, src_h) = base;
    let mut mips: Vec<Value> = Vec::new();
    for (level, &(lw, lh)) in level_meta.iter().enumerate() {
        let tiles: Vec<Value> = blocks
            .iter()
            .filter(|b| b.level == level)
            .map(|b| {
                json!({ "x": b.x, "y": b.y, "width": b.w, "height": b.h, "offset": 0, "length": b.buf.len() })
            })
            .collect();
        mips.push(json!({ "level": level, "width": lw, "height": lh, "tiles": tiles }));
    }
    let mut index = json!({
        "formatVersion": 1,
        "terrainId": terrain,
        "worldBounds": world_bounds,
        "metersPerPixel": js_num(world_bounds[2] as f64 / src_w as f64),
        "source": if source_meta["source"].is_string() { source_meta["source"].clone() } else { json!("unknown") },
        "sourceMeta": source_meta,
        "encoding": "webp-lossless",
        "createdAt": iso_from_system_time(std::time::SystemTime::now()),
        "inputSha256": input_sha256,
        "baseWidthPx": src_w,
        "baseHeightPx": src_h,
        "mipCount": level_meta.len(),
        "mips": mips,
    });

    // Two-pass offset patch until the JSON length stabilizes.
    let mut json_buf: Vec<u8>;
    let mut json_len = 0usize;
    loop {
        let mut offset = 12 + json_len;
        for mip in index["mips"].as_array_mut().unwrap() {
            for t in mip["tiles"].as_array_mut().unwrap() {
                t["offset"] = json!(offset);
                offset += t["length"].as_u64().unwrap() as usize;
            }
        }
        json_buf = serde_json::to_string(&index)?.into_bytes();
        if json_buf.len() == json_len {
            break;
        }
        json_len = json_buf.len();
    }

    let mut file = Vec::with_capacity(12 + json_buf.len());
    file.extend_from_slice(b"TBDS");
    file.extend_from_slice(&1u32.to_le_bytes());
    file.extend_from_slice(&(json_buf.len() as u32).to_le_bytes());
    file.extend_from_slice(&json_buf);
    for b in blocks {
        file.extend_from_slice(&b.buf);
    }
    Ok(file)
}
