use super::*;

use crate::repository_layout::{map_scratch_dir, terrain_dir, terrain_manifest_path};

pub(super) fn sap_dir() -> PathBuf {
    map_scratch_dir(&repo_root(), "everon").join("sap") // E2c-allow (Eden-only lane)
}

pub(super) fn read_dem_u16(path: &std::path::Path) -> Result<(Vec<u16>, usize, usize)> {
    let dec = png::Decoder::new(std::fs::File::open(path)?);
    let mut reader = dec.read_info()?;
    let mut data = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut data)?;
    let (w, h) = (info.width as usize, info.height as usize);
    let mut r16 = vec![0u16; w * h];
    for (i, px) in r16.iter_mut().enumerate() {
        *px = u16::from_be_bytes([data[i * 2], data[i * 2 + 1]]);
    }
    Ok((r16, w, h))
}

pub fn composite_water_ortho() -> Result<u8> {
    let root = repo_root();
    let sap = sap_dir();
    let ortho_path = sap.join("everon-sap-ortho.png");
    let backup = sap.join("everon-sap-ortho.pre-water.png");
    let meta_path = sap.join("TBD_SatExport_meta.json");
    let inland_path = sap.join("water-inland-mask.png");
    let dem_path = terrain_dir(&root, "everon").join("dem/everon-dem-16bit.png"); // E2c-allow
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(
        terrain_manifest_path(&root, "everon"), // E2c-allow
    )?)?;
    let log = |m: &str| println!("[water-composite] {m}");
    let t0 = std::time::Instant::now();

    for p in [&ortho_path, &inland_path, &dem_path] {
        if !p.exists() {
            eprintln!(
                "missing {}{}",
                p.display(),
                if p == &inland_path {
                    " — run analyze-water first"
                } else {
                    ""
                }
            );
            return Ok(1);
        }
    }
    let mut meta: Value = serde_json::from_str(&std::fs::read_to_string(&meta_path)?)?;
    if meta["waterComposite"].is_object() {
        eprintln!(
            "meta already has waterComposite — restore everon-sap-ortho.pre-water.png (and remove the meta block) before re-running"
        );
        return Ok(1);
    }

    log("reading ortho (12800², this takes a moment)");
    let mut ortho = image_operations::load_png_rgba(&ortho_path)?;
    let w = ortho.w;
    if w != 12800 || ortho.h != 12800 {
        eprintln!("ortho is {w}x{}, expected 12800²", ortho.h);
        return Ok(1);
    }
    let (dem, dw, _dh) = read_dem_u16(&dem_path)?;
    let lo = manifest["dem"]["heightRangeMinM"].as_f64().unwrap_or(0.0);
    let hi = manifest["dem"]["heightRangeMaxM"].as_f64().unwrap_or(1.0);
    let sea_u16 = js_math_round(((0.0 - lo) / (hi - lo)) * 65535.0);
    let m_per_u16 = (hi - lo) / 65535.0;
    let inland = image_operations::load_png_rgba(&inland_path)?;
    if inland.w != w || inland.h != w {
        eprintln!("inland mask is {}x{}, expected {w}²", inland.w, inland.h);
        return Ok(1);
    }

    log("building water masks");
    let n = w * w;
    let mut alpha = vec![0u8; n];
    let mut is_ocean = vec![0u8; n];
    let mut ocean_px = 0u64;
    let mut inland_px = 0u64;
    for y in 0..w {
        let dem_y = (dw - 1).min((w - 1 - y) >> 1);
        for x in 0..w {
            let i = y * w + x;
            let v = f64::from(dem[dem_y * dw + (x >> 1)]);
            if v <= sea_u16 {
                alpha[i] = 255;
                is_ocean[i] = 1;
                ocean_px += 1;
            } else if inland.data[i * 4] > 127 {
                alpha[i] = 255;
                inland_px += 1;
            }
        }
    }
    log(&format!(
        "ocean {:.1} Mpx, inland {:.2} Mpx ({:.0} ha)",
        ocean_px as f64 / 1e6,
        inland_px as f64 / 1e6,
        inland_px as f64 / 1e4
    ));

    // Inward feather: separable box blur ×2 (integer semantics as the .mjs: (acc/win)|0).
    log(&format!("feathering (inward, r={FEATHER_R})"));
    let blur_pass = |src: &[u8]| -> Vec<u8> {
        let r = FEATHER_R as isize;
        let win = (2 * r + 1) as f64;
        let mut tmp = vec![0u8; n];
        for y in 0..w {
            let row = y * w;
            let mut acc: i64 = 0;
            for x in -r..=r {
                acc += i64::from(src[row + x.clamp(0, w as isize - 1) as usize]);
            }
            for x in 0..w {
                tmp[row + x] = ((acc as f64 / win) as i64).clamp(0, 255) as u8;
                let add = ((x as isize) + r + 1).min(w as isize - 1) as usize;
                let sub = ((x as isize) - r).max(0) as usize;
                acc += i64::from(src[row + add]) - i64::from(src[row + sub]);
            }
        }
        let mut out = vec![0u8; n];
        for x in 0..w {
            let mut acc: i64 = 0;
            for y in -r..=r {
                acc += i64::from(tmp[y.clamp(0, w as isize - 1) as usize * w + x]);
            }
            for y in 0..w {
                out[y * w + x] = ((acc as f64 / win) as i64).clamp(0, 255) as u8;
                let add = ((y as isize) + r + 1).min(w as isize - 1) as usize;
                let sub = ((y as isize) - r).max(0) as usize;
                acc += i64::from(tmp[add * w + x]) - i64::from(tmp[sub * w + x]);
            }
        }
        out
    };
    let mut soft = blur_pass(&blur_pass(&alpha));
    for i in 0..n {
        if alpha[i] == 0 {
            soft[i] = 0;
        }
    }

    log("blending");
    let d = &mut ortho.data;
    for y in 0..w {
        let dem_y = (dw - 1).min((w - 1 - y) >> 1);
        for x in 0..w {
            let i = y * w + x;
            let a8 = soft[i];
            if a8 == 0 {
                continue;
            }
            let (cr, cg, cb) = if is_ocean[i] == 1 {
                let v = f64::from(dem[dem_y * dw + (x >> 1)]);
                let depth_m = (sea_u16 - v) * m_per_u16;
                let t = (depth_m / DEPTH_FULL_M).min(1.0);
                (
                    OCEAN_BRIGHT[0] + (OCEAN_DARK[0] - OCEAN_BRIGHT[0]) * t,
                    OCEAN_BRIGHT[1] + (OCEAN_DARK[1] - OCEAN_BRIGHT[1]) * t,
                    OCEAN_BRIGHT[2] + (OCEAN_DARK[2] - OCEAN_BRIGHT[2]) * t,
                )
            } else {
                (INLAND_COLOR[0], INLAND_COLOR[1], INLAND_COLOR[2])
            };
            let a = (f64::from(a8) / 255.0) * WATER_ALPHA;
            let o = i * 4;
            d[o] = js_math_round(f64::from(d[o]) * (1.0 - a) + cr * a) as u8;
            d[o + 1] = js_math_round(f64::from(d[o + 1]) * (1.0 - a) + cg * a) as u8;
            d[o + 2] = js_math_round(f64::from(d[o + 2]) * (1.0 - a) + cb * a) as u8;
        }
    }

    if !backup.exists() {
        log("backing up pre-water ortho");
        std::fs::copy(&ortho_path, &backup)?;
    }
    log("writing composited ortho");
    let buf: image::RgbaImage =
        image::ImageBuffer::from_raw(ortho.w as u32, ortho.h as u32, ortho.data)
            .ok_or_else(|| anyhow::anyhow!("bad buffer"))?;
    buf.save(&ortho_path)?;

    meta["waterComposite"] = json!({
        "slice": "T-090.1.2.5",
        "refineSlice": "T-090.1.2.5.2",
        "oceanMaskSource": "dem-below-sea-level",
        "inlandMaskSource": "supertexture-water-appearance-dem-filtered + topo-road-subtraction (exact .topo road network guard; relaxed wet-channel stream class)",
        "spikeArtifact": ".ai/artifacts/t090_1_2_5_water_source_spike.json",
        "refineSpikeArtifact": ".ai/artifacts/t090_1_2_5_2_source_spike.json",
        "palette": { "oceanBright": OCEAN_BRIGHT.map(js_num), "oceanDark": OCEAN_DARK.map(js_num), "inland": INLAND_COLOR.map(js_num) },
        "waterAlpha": WATER_ALPHA,
        "depthFullM": js_num(DEPTH_FULL_M),
        "featherRadiusPx": FEATHER_R,
        "featherMode": "inward-only (land pixels outside the mask are byte-identical)",
        "oceanPx": ocean_px,
        "inlandPx": inland_px,
        "generatedAt": iso_from_system_time(std::time::SystemTime::now()),
    });
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)? + "\n")?;
    log(&format!(
        "done in {:.0}s — meta.waterComposite written",
        t0.elapsed().as_secs_f64()
    ));
    Ok(0)
}

pub(super) fn dilate(src: &[u8], d: usize, r: usize) -> Vec<u8> {
    let mut out = vec![0u8; d * d];
    let r = r as isize;
    for y in 0..d as isize {
        for x in 0..d as isize {
            if src[(y * d as isize + x) as usize] == 0 {
                continue;
            }
            for dy in -r..=r {
                let ny = y + dy;
                if ny < 0 || ny >= d as isize {
                    continue;
                }
                for dx in -r..=r {
                    let nx = x + dx;
                    if nx >= 0 && nx < d as isize {
                        out[(ny * d as isize + nx) as usize] = 1;
                    }
                }
            }
        }
    }
    out
}
