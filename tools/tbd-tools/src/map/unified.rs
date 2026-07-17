//! T-165.9 — unified satellite bundle (tbd-sat v1): the verifier (port of
//! `verify-unified-satellite.mjs`, dep-free byte parse) and the builder (port of
//! `build-unified-satellite.mjs` — Lanczos cascade mips, tile crop, VP8L via image-webp).
//! Plus the tile-pyramid verifier (port of `verify-tile-pyramid.mjs`).

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde_json::{Value, json};

use super::img;
use crate::serve::repo_root;
use crate::world::aux::iso_from_system_time;
use crate::world::jsval::js_num;

fn map_assets_root() -> PathBuf {
    repo_root().join("packages/map-assets")
}

/* ─────────────────────────── verify-unified-satellite ─────────────────────────── */

pub fn verify_unified_satellite(terrain: &str) -> Result<u8> {
    let root = map_assets_root();
    let manifest_path = root.join(terrain).join("manifest.json");
    let die = |m: &str| {
        eprintln!("verify-unified-satellite: FAIL — {m}");
        1u8
    };
    if !manifest_path.exists() {
        return Ok(die(&format!(
            "manifest missing {}",
            manifest_path.display()
        )));
    }
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    let sat = &manifest["tiles"]["satellite"];
    let unified = &sat["unified"];
    let bundle_rel = unified["path"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("satellite/{terrain}-sat.tbd-sat"));
    let bundle = root.join(terrain).join(&bundle_rel);

    let mut errors: Vec<String> = Vec::new();
    let mut fail = |m: String| errors.push(m);

    if !bundle.exists() {
        return Ok(die(&format!(
            "bundle missing {} (build it, then check .gitattributes LFS rule)",
            bundle.display()
        )));
    }
    let buf = std::fs::read(&bundle)?;
    if buf.len() < 64 || buf.starts_with(b"version http") {
        return Ok(die(&format!(
            "{} is a git-lfs pointer, not the bundle — run `git lfs pull`",
            bundle.display()
        )));
    }
    if &buf[0..4] != b"TBDS" {
        return Ok(die("bad magic (expected \"TBDS\")"));
    }
    let version = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if version != 1 {
        return Ok(die(&format!(
            "unsupported formatVersion {version} (expected 1)"
        )));
    }
    let json_len = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize;
    if 12 + json_len > buf.len() {
        return Ok(die(&format!(
            "jsonLength {json_len} overruns file ({} bytes)",
            buf.len()
        )));
    }
    let index: Value = match serde_json::from_slice(&buf[12..12 + json_len]) {
        Ok(v) => v,
        Err(e) => return Ok(die(&format!("JSON index unparseable: {e}"))),
    };

    if index["formatVersion"] != 1 {
        fail(format!(
            "index.formatVersion {} !== 1",
            index["formatVersion"]
        ));
    }
    if index["terrainId"] != terrain {
        fail(format!(
            "index.terrainId {} !== {terrain}",
            index["terrainId"]
        ));
    }
    if index["worldBounds"] != manifest["worldBounds"] {
        fail(format!(
            "index.worldBounds {} !== manifest {}",
            index["worldBounds"], manifest["worldBounds"]
        ));
    }
    if index["encoding"] != "webp-lossless" {
        fail(format!(
            "index.encoding {} !== webp-lossless",
            index["encoding"]
        ));
    }
    let base_w = index["baseWidthPx"].as_u64().unwrap_or(0);
    let base_h = index["baseHeightPx"].as_u64().unwrap_or(0);
    let expected_mips = (base_w.max(base_h) as f64).log2().floor() as u64 + 1;
    if index["mipCount"].as_u64() != Some(expected_mips) {
        fail(format!(
            "mipCount {} !== floor(log2(base))+1 = {expected_mips}",
            index["mipCount"]
        ));
    }
    let mips = index["mips"].as_array().cloned().unwrap_or_default();
    if mips.len() as u64 != index["mipCount"].as_u64().unwrap_or(u64::MAX) {
        fail(format!(
            "mips[] length {} !== mipCount {}",
            mips.len(),
            index["mipCount"]
        ));
    }
    let (mut w, mut h) = (base_w, base_h);
    for (i, mip) in mips.iter().enumerate() {
        if mip["level"].as_u64() != Some(i as u64) {
            fail(format!("mips[{i}].level = {} (must be {i})", mip["level"]));
        }
        if mip["width"].as_u64() != Some(w) || mip["height"].as_u64() != Some(h) {
            fail(format!(
                "level {i}: {}x{}, GL rule expects {w}x{h}",
                mip["width"], mip["height"]
            ));
        }
        w = 1.max(w / 2);
        h = 1.max(h / 2);
    }
    if let Some(last) = mips.last()
        && (last["width"] != 1 || last["height"] != 1)
    {
        fail(format!(
            "chain must end at 1x1 (got {}x{})",
            last["width"], last["height"]
        ));
    }

    let mut block_count = 0u64;
    let mut payload_bytes = 0u64;
    for mip in &mips {
        let mut seen = std::collections::HashSet::new();
        let mut covered = 0u64;
        let (mw, mh) = (
            mip["width"].as_i64().unwrap_or(0),
            mip["height"].as_i64().unwrap_or(0),
        );
        for t in mip["tiles"].as_array().cloned().unwrap_or_default() {
            block_count += 1;
            let (off, len) = (
                t["offset"].as_u64().unwrap_or(0) as usize,
                t["length"].as_u64().unwrap_or(0) as usize,
            );
            payload_bytes += len as u64;
            let (tx, ty) = (t["x"].as_i64().unwrap_or(-1), t["y"].as_i64().unwrap_or(-1));
            let (tw, th) = (
                t["width"].as_i64().unwrap_or(0),
                t["height"].as_i64().unwrap_or(0),
            );
            if off < 12 + json_len || off + len > buf.len() {
                fail(format!(
                    "level {} tile @({tx},{ty}): offset {off}+{len} out of range",
                    mip["level"]
                ));
                continue;
            }
            match img::webp_dims(&buf[off..off + len.min(64)]) {
                None => fail(format!(
                    "level {} tile @({tx},{ty}): not a RIFF/WEBP block",
                    mip["level"]
                )),
                Some(d) if &d.fourcc != b"VP8L" => fail(format!(
                    "level {} tile @({tx},{ty}): {}, expected VP8L (lossless)",
                    mip["level"],
                    String::from_utf8_lossy(&d.fourcc)
                )),
                Some(d) if i64::from(d.w) != tw || i64::from(d.h) != th => fail(format!(
                    "level {} tile @({tx},{ty}): VP8L says {}x{}, index says {tw}x{th}",
                    mip["level"], d.w, d.h
                )),
                _ => {}
            }
            if !seen.insert((tx, ty)) {
                fail(format!(
                    "level {}: duplicate tile @({tx},{ty})",
                    mip["level"]
                ));
            }
            if tx < 0 || ty < 0 || tx + tw > mw || ty + th > mh {
                fail(format!(
                    "level {} tile @({tx},{ty}) {tw}x{th} exceeds level {mw}x{mh}",
                    mip["level"]
                ));
            }
            covered += (tw * th) as u64;
        }
        if covered != (mw * mh) as u64 {
            fail(format!(
                "level {}: tiles cover {covered}px², level is {}px² (gap/overlap)",
                mip["level"],
                mw * mh
            ));
        }
    }
    if 12 + json_len as u64 + payload_bytes != buf.len() as u64 {
        fail(format!(
            "payload bytes {payload_bytes} + header {} !== file size {}",
            12 + json_len,
            buf.len()
        ));
    }

    if sat["delivery"] != "unified" {
        fail(format!(
            "manifest tiles.satellite.delivery \"{}\" !== \"unified\"",
            sat["delivery"].as_str().unwrap_or("")
        ));
    }
    if unified["encoding"] != "tbd-sat-v1" {
        fail(format!(
            "manifest unified.encoding \"{}\" !== \"tbd-sat-v1\"",
            unified["encoding"].as_str().unwrap_or("")
        ));
    }
    if let Some(url) = unified["url"].as_str()
        && !url.contains(&format!("/{terrain}/{bundle_rel}"))
    {
        fail(format!(
            "manifest unified.url {url} does not point at {terrain}/{bundle_rel}"
        ));
    }
    if unified["baseWidthPx"].as_u64() != Some(base_w)
        || unified["baseHeightPx"].as_u64() != Some(base_h)
    {
        fail(format!(
            "manifest unified base {}x{} !== bundle {base_w}x{base_h}",
            unified["baseWidthPx"], unified["baseHeightPx"]
        ));
    }
    if unified["mipCount"] != index["mipCount"] {
        fail(format!(
            "manifest unified.mipCount {} !== bundle {}",
            unified["mipCount"], index["mipCount"]
        ));
    }
    let size = std::fs::metadata(&bundle)?.len();
    if unified["bytes"].as_u64() != Some(size) {
        fail(format!(
            "manifest unified.bytes {} !== file size {size}",
            unified["bytes"]
        ));
    }

    if !errors.is_empty() {
        eprintln!(
            "verify-unified-satellite: FAIL ({}) for {terrain}",
            errors.len()
        );
        for e in &errors {
            eprintln!("  - {e}");
        }
        return Ok(1);
    }
    println!(
        "verify-unified-satellite: OK {terrain} — {base_w}x{base_h}, {} mips, {block_count} VP8L blocks, {:.1} MB",
        index["mipCount"],
        size as f64 / 1e6
    );
    Ok(0)
}

/* ─────────────────────────── verify-tile-pyramid ─────────────────────────── */

pub fn verify_tile_pyramid(terrain: &str, view_map: bool, expect_lossless_env: bool) -> Result<u8> {
    let view = if view_map { "map" } else { "satellite" };
    let root = map_assets_root();
    let tiles_dir = root.join(terrain).join("tiles").join(view);
    let manifest_path = root.join(terrain).join("manifest.json");

    if !tiles_dir.exists() {
        println!(
            "verify-tile-pyramid: SKIP {terrain}/{view} — no pyramid on disk (local rebuild: make map-water-everon or the pyramid builder)"
        );
        return Ok(0);
    }
    if !manifest_path.exists() {
        eprintln!(
            "verify-tile-pyramid: manifest missing {}",
            manifest_path.display()
        );
        return Ok(1);
    }
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    let tiles = &manifest["tiles"];
    let view_block = if view_map {
        &tiles["map"]
    } else {
        &tiles["satellite"]
    };
    let sat = &tiles["satellite"];
    let tile_size = tiles["tileSizePx"].as_u64().unwrap_or(256) as u32;
    let min_zoom = tiles["minZoom"].as_u64().unwrap_or(0) as u32;
    let max_zoom = tiles["maxZoom"].as_u64().unwrap_or(5) as u32;
    let expect_lossless = !view_map && (expect_lossless_env || sat["encoding"] == "webp-lossless");

    let mut errors: Vec<String> = Vec::new();
    let mut fail = |m: String| errors.push(m);

    if !tiles_dir.join("0/0/0.webp").exists() {
        fail(format!(
            "missing {}/0/0/0.webp (K3 file gate)",
            tiles_dir.display()
        ));
    }
    let expect_path = format!("tiles/{view}");
    if let Some(p) = view_block["path"].as_str()
        && p != expect_path
    {
        fail(format!("manifest tiles.{view}.path={p} != {expect_path}"));
    }
    if let Some(u) = view_block["urlTemplate"].as_str()
        && !u.contains(&format!("/{terrain}/tiles/{view}/"))
    {
        fail(format!(
            "manifest tiles.{view}.urlTemplate does not point at {terrain}/tiles/{view}: {u}"
        ));
    }

    let mut checked = 0u64;
    let mut lossless_checked = 0u64;
    let mut levels: Vec<u32> = Vec::new();
    for z in min_zoom..=max_zoom {
        let n = 1u32 << z;
        let z_dir = tiles_dir.join(z.to_string());
        if !z_dir.exists() {
            fail(format!(
                "z={z}: level missing (pyramid must be complete [{min_zoom}..{max_zoom}])"
            ));
            continue;
        }
        levels.push(z);
        let xs = std::fs::read_dir(&z_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok()?.parse::<u32>().ok())
            .count() as u32;
        if xs != n {
            fail(format!("z={z}: {xs} x-columns, expected {n}"));
        }
        for x in 0..n {
            for y in 0..n {
                let p = tiles_dir.join(format!("{z}/{x}/{y}.webp"));
                if !p.exists() {
                    fail(format!("z={z}: missing tile {x}/{y}.webp"));
                    continue;
                }
                let head = std::fs::read(&p)?;
                match img::webp_dims(&head) {
                    None => fail(format!("z={z} {x}/{y}: not a valid RIFF/WEBP")),
                    Some(d) => {
                        if d.w != 0 && (d.w != tile_size || d.h != tile_size) {
                            fail(format!(
                                "z={z} {x}/{y}: {}x{}, expected {tile_size}x{tile_size}",
                                d.w, d.h
                            ));
                        }
                        if expect_lossless {
                            if &d.fourcc == b"VP8 " {
                                fail(format!(
                                    "z={z} {x}/{y}: VP8 lossy chunk, expected VP8L (lossless)"
                                ));
                            } else if &d.fourcc == b"VP8L" {
                                lossless_checked += 1;
                            }
                        }
                    }
                }
                checked += 1;
            }
        }
    }

    if !errors.is_empty() {
        eprintln!("verify-tile-pyramid: FAIL ({}) for {terrain}", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
        return Ok(1);
    }
    let lossless_note = if expect_lossless {
        format!(", {lossless_checked} VP8L lossless")
    } else {
        String::new()
    };
    println!(
        "verify-tile-pyramid: OK {terrain} — levels [{}], {checked} tiles, {tile_size}px{lossless_note}",
        levels
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(0)
}

/* ─────────────────────────── build-unified-satellite ─────────────────────────── */

pub fn build_unified_satellite(
    input: &Path,
    out: &Path,
    terrain: &str,
    tile_threshold: usize,
) -> Result<u8> {
    use sha2::Digest as _;
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
    let base = img::load_png_rgb(input)?;
    let (src_w, src_h) = (base.w, base.h);
    log(&format!(
        "source {src_w}x{src_h}; tileThreshold={tile_threshold}"
    ));

    // Mip chain dims: base → 1×1 with the GL rule.
    let mut dims = Vec::new();
    let (mut w, mut h) = (src_w, src_h);
    loop {
        dims.push((w, h));
        if w == 1 && h == 1 {
            break;
        }
        w = 1.max(w / 2);
        h = 1.max(h / 2);
    }
    log(&format!("mip chain: {} levels ({src_w} → 1)", dims.len()));

    // Cascade-halve + tile + encode (rayon-free: encode sequentially — image-webp lossless
    // is fast enough for the rebuild-smoke acceptance; parallelism can come later).
    struct TileBuf {
        level: usize,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
        buf: Vec<u8>,
    }
    let mut blocks: Vec<TileBuf> = Vec::new();
    let mut level_meta = Vec::new();
    let mut current = base;
    for (level, &(lw, lh)) in dims.iter().enumerate() {
        if level > 0 {
            current = img::resize_rgb(&current, lw, lh);
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
                    img::crop_rgb(&current, x, y, tw, th)?.data
                };
                let buf = img::encode_webp_lossless_rgb(&img::Rgb8 {
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
                    "slice": wc["slice"], "oceanMaskSource": wc["oceanMaskSource"],
                    "inlandMaskSource": wc["inlandMaskSource"], "generatedAt": wc["generatedAt"],
                }),
            );
        }
        source_meta = Value::Object(sm);
    }

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
        "mipCount": dims.len(),
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
    for b in &blocks {
        file.extend_from_slice(&b.buf);
    }
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out, &file)?;

    for mip in index["mips"].as_array().unwrap() {
        let bytes: u64 = mip["tiles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["length"].as_u64().unwrap())
            .sum();
        log(&format!(
            "  level {:>2}  {:>5}px  {} block(s)  {:.2} MB",
            mip["level"],
            mip["width"],
            mip["tiles"].as_array().unwrap().len(),
            bytes as f64 / 1e6
        ));
    }
    log(&format!(
        "wrote {}  {:.1} MB in {:.0}s",
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
                "encoding": "tbd-sat-v1",
                "baseWidthPx": src_w,
                "baseHeightPx": src_h,
                "mipCount": dims.len(),
                "bytes": file.len(),
            },
        }))?
    );
    Ok(0)
}
