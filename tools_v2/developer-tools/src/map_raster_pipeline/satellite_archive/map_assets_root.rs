use super::*;

use crate::repository_layout::terrain_assets_dir;

pub(super) fn map_assets_root() -> PathBuf {
    terrain_assets_dir(&repo_root())
}

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
    // v1 stores its version as a u32 at offset 4; v2's `TbdsHeader` stores a u16 version there
    // followed by a u16 `flags` that the writer leaves zero — so the same four bytes read 1 or 2
    // for either container, and this dispatch does not have to know which shape it is looking at
    // before it has decided.
    let version = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    let summary = match version {
        1 => verify_bundle_v1(&buf, terrain, &manifest, &mut errors),
        2 => verify_bundle_v2(&buf, &mut errors),
        v => {
            return Ok(die(&format!(
                "unsupported formatVersion {v} (expected 1 or 2)"
            )));
        }
    };
    let Some(summary) = summary else {
        eprintln!(
            "verify-unified-satellite: FAIL ({}) for {terrain}",
            errors.len()
        );
        for e in &errors {
            eprintln!("  - {e}");
        }
        return Ok(1);
    };
    let (base_w, base_h, block_count) = (summary.base_w, summary.base_h, summary.block_count);
    let mut fail = |m: String| errors.push(m);

    if sat["delivery"] != "unified" {
        fail(format!(
            "manifest tiles.satellite.delivery \"{}\" !== \"unified\"",
            sat["delivery"].as_str().unwrap_or("")
        ));
    }
    if unified["encoding"] != summary.encoding {
        fail(format!(
            "manifest unified.encoding \"{}\" !== \"{}\" (the bundle on disk is v{version})",
            unified["encoding"].as_str().unwrap_or(""),
            summary.encoding
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
    if unified["mipCount"].as_u64() != Some(summary.mip_count) {
        fail(format!(
            "manifest unified.mipCount {} !== bundle {}",
            unified["mipCount"], summary.mip_count
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
        "verify-unified-satellite: OK {terrain} — {base_w}x{base_h}, {} mips, {block_count} VP8L blocks, {:.1} MB (tbd-sat v{version})",
        summary.mip_count,
        size as f64 / 1e6
    );
    Ok(0)
}

/// The v1 (hand-packed JSON table) bundle checks. `None` = fatal.
pub(super) fn verify_bundle_v1(
    buf: &[u8],
    terrain: &str,
    manifest: &Value,
    errors: &mut Vec<String>,
) -> Option<BundleSummary> {
    let mut fail = |m: String| errors.push(m);
    let json_len = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize;
    if 12 + json_len > buf.len() {
        fail(format!(
            "jsonLength {json_len} overruns file ({} bytes)",
            buf.len()
        ));
        return None;
    }
    let index: Value = match serde_json::from_slice(&buf[12..12 + json_len]) {
        Ok(v) => v,
        Err(e) => {
            fail(format!("JSON index unparseable: {e}"));
            return None;
        }
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
            match image_operations::webp_dims(&buf[off..off + len.min(64)]) {
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

    Some(BundleSummary {
        base_w,
        base_h,
        mip_count: index["mipCount"].as_u64().unwrap_or(0),
        block_count,
        encoding: "tbd-sat-v1",
    })
}

pub fn verify_tile_pyramid(terrain: &str, view_map: bool, expect_lossless_env: bool) -> Result<u8> {
    let view = if view_map { "map" } else { "satellite" };
    let root = map_assets_root();
    let tiles_dir = root.join(terrain).join("tiles").join(view);
    let manifest_path = root.join(terrain).join("manifest.json");

    if !tiles_dir.exists() {
        println!(
            "verify-tile-pyramid: SKIP {terrain}/{view} — no pyramid on disk (local rebuild: cargo xtask ci map-water-everon or the pyramid builder)"
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
                match image_operations::webp_dims(&head) {
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
