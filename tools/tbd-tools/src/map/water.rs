//! T-165.9 — the water lane: `analyze-water-sources.mjs` (the T-090.1.2.5.2 inland-water
//! classifier — grey/wet pixel classes on the road-subtracted field, component acceptance,
//! mask + preview + spike JSON) and `composite-water-ortho.mjs` (ocean ramp + inland tint
//! over the SAP ortho, inward feather, in-place with backup + meta block).

use std::path::PathBuf;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::img::{self, Rgb8};
use crate::serve::repo_root;
use crate::world::aux::iso_from_system_time;
use crate::world::jsval::{js_math_round, js_num};
use crate::world::pak::PakVfs;
use crate::world::topo::{TOPO_AIRFIELD, decode_topo};

fn sap_dir() -> PathBuf {
    repo_root().join("packages/map-assets/everon/staging/sap") // E2c-allow (Eden-only lane)
}

fn read_dem_u16(path: &std::path::Path) -> Result<(Vec<u16>, usize, usize)> {
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

/* ─────────────────────────── composite-water-ortho ─────────────────────────── */

const OCEAN_BRIGHT: [f64; 3] = [58.0, 96.0, 120.0];
const OCEAN_DARK: [f64; 3] = [28.0, 52.0, 78.0];
const INLAND_COLOR: [f64; 3] = [52.0, 88.0, 112.0];
const WATER_ALPHA: f64 = 0.8;
const DEPTH_FULL_M: f64 = 80.0;
const FEATHER_R: usize = 3;

pub fn composite_water_ortho() -> Result<u8> {
    let root = repo_root();
    let sap = sap_dir();
    let ortho_path = sap.join("everon-sap-ortho.png");
    let backup = sap.join("everon-sap-ortho.pre-water.png");
    let meta_path = sap.join("TBD_SatExport_meta.json");
    let inland_path = sap.join("water-inland-mask.png");
    let dem_path = root.join("packages/map-assets/everon/dem/everon-dem-16bit.png"); // E2c-allow
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("packages/map-assets/everon/manifest.json"), // E2c-allow
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
    let mut ortho = img::load_png_rgba(&ortho_path)?;
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
    let inland = img::load_png_rgba(&inland_path)?;
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

/* ─────────────────────────── analyze-water-sources ─────────────────────────── */

const DETECT_DIM: usize = 3200;
const SAT_MAX: f32 = 0.12;
const LUM_MIN: f32 = 0.2;
const LUM_MAX: f32 = 0.44;
const OPEN_R: usize = 2;
const DENSITY_MIN: f64 = 0.6;
const OCEAN_DILATE_R: usize = 5;
const FLAT_DILATE_R: usize = 2;
const MIN_AREA_M2: u64 = 2000;
const MEAN_SAT_MAX: f64 = 0.115;
const SLOPE_PX_MAX_DEG: f32 = 18.0;
const FLAT_FRAC_MAX: f64 = 0.12;
const SLOPE_MEAN_MAX_DEG: f64 = 8.0;
const ROAD_SAMPLE_STEP_PX: f64 = 2.0;
const ROAD_OVERLAP_MAX: f64 = 0.45;
const RIBBON_W_MAX_PX: f64 = 5.0;
const LIN_MIN_AREA_M2: u64 = 800;
const LIN_SLOPE_MEAN_MAX_DEG: f64 = 16.0;
const LIN_FLAT_FRAC_MAX: f64 = 0.2;
const GREY_RIVER_VALLEY_MIN: f64 = 0.2;
const GREY_RIVER_LOWLAND_SLOPE_DEG: f64 = 8.0;
const WET_MIN_AREA_M2: u64 = 1000;
const WET_VALLEY_FRAC_MIN: f64 = 0.6;
const WET_MEAN_SAT_MAX: f64 = 0.18;
const WET_MEAN_LUM_MAX: f64 = 0.31;
const WET_LUM_MIN: f32 = 0.09;
const WET_LUM_MAX: f32 = 0.33;
const WET_SAT_MAX: f32 = 0.19;
const WET_SLOPE_PX_MAX_DEG: f32 = 24.0;
const VALLEY_BLUR_R: usize = 12;
const VALLEY_CARVE_M: f32 = 0.8;

fn dilate(src: &[u8], d: usize, r: usize) -> Vec<u8> {
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

#[allow(clippy::too_many_lines)]
pub fn analyze_water_sources() -> Result<u8> {
    let root = repo_root();
    let sap = sap_dir();
    let ortho_path = sap.join("everon-sap-ortho.png");
    let dem_path = root.join("packages/map-assets/everon/dem/everon-dem-16bit.png"); // E2c-allow
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("packages/map-assets/everon/manifest.json"), // E2c-allow
    )?)?;
    let out_json = root.join(".ai/artifacts/t090_1_2_5_2_source_spike.json");
    let prev_spike = root.join(".ai/artifacts/t090_1_2_5_1_refine_spike.json");
    let out_mask = sap.join("water-inland-mask.png");
    let out_preview = sap.join("water-spike-preview.png");
    let t0 = std::time::Instant::now();
    let log = |m: &str| println!("[water-spike] {m}");

    if !ortho_path.exists() {
        eprintln!(
            "missing {} — run the SAP stitch pipeline first",
            ortho_path.display()
        );
        return Ok(1);
    }

    // ── DEM: sea + exact-flat + slope + elev planes (6400² → 3200², north-up) ──
    let (dem, dw, dh) = read_dem_u16(&dem_path)?;
    if dw != 6400 || dh != 6400 {
        bail!("DEM {dw}x{dh}, expected 6400²");
    }
    let lo = manifest["dem"]["heightRangeMinM"].as_f64().unwrap_or(0.0);
    let hi = manifest["dem"]["heightRangeMaxM"].as_f64().unwrap_or(1.0);
    let sea_u16 = js_math_round(((0.0 - lo) / (hi - lo)) * 65535.0) as u16;
    let m_per_u16 = ((hi - lo) / 65535.0) as f32;
    let d = DETECT_DIM;
    let dem_v = |x: usize, y: usize| dem[y * dw + x];
    let mut sea = vec![0u8; d * d];
    let mut flat6400 = vec![0u8; dw * dw];
    let mut sea_px6400 = 0u64;
    for y in 0..dw {
        for x in 0..dw {
            let v = dem_v(x, y);
            if v <= sea_u16 {
                sea_px6400 += 1;
                continue;
            }
            if x < dw - 1
                && y < dw - 1
                && v == dem_v(x + 1, y)
                && v == dem_v(x, y + 1)
                && v == dem_v(x + 1, y + 1)
            {
                flat6400[y * dw + x] = 1;
            }
        }
    }
    let mut flat = vec![0u8; d * d];
    let mut slope = vec![0f32; d * d];
    let mut elev_m = vec![0f32; d * d];
    for y in 0..dw {
        let ny = dw - 1 - y;
        for x in 0..dw {
            let di = (ny >> 1) * d + (x >> 1);
            let v = dem_v(x, y);
            elev_m[di] += (f32::from(v) * m_per_u16 + lo as f32) / 4.0;
            if v <= sea_u16 {
                sea[di] = 1;
            }
            if flat6400[y * dw + x] == 1 {
                flat[di] = 1;
            }
            if x > 0 && x < dw - 1 && y > 0 && y < dw - 1 {
                let gx =
                    (f32::from(dem_v(x + 1, y)) - f32::from(dem_v(x - 1, y))) * m_per_u16 / 4.0;
                let gy =
                    (f32::from(dem_v(x, y + 1)) - f32::from(dem_v(x, y - 1))) * m_per_u16 / 4.0;
                let s = gx.hypot(gy).atan().to_degrees();
                if s > slope[di] {
                    slope[di] = s;
                }
            }
        }
    }
    // Valley-carve mask.
    let valley: Vec<u8> = {
        let elev64: Vec<f32> = elev_m.clone();
        let blurred = img::box_blur_f32(&elev64, d, d, VALLEY_BLUR_R);
        (0..d * d)
            .map(|i| u8::from(blurred[i] - elev_m[i] > VALLEY_CARVE_M))
            .collect()
    };
    let sea_fraction = sea_px6400 as f64 / (dw * dw) as f64;
    log(&format!(
        "DEM sea fraction {:.1} % (sea level u16={sea_u16})",
        sea_fraction * 100.0
    ));

    // Inland <=0 audit (flood-fill sea from borders; leftovers = inland).
    let inland_below_sea_m2 = {
        let mut m = sea.clone();
        let mut q: Vec<usize> = Vec::new();
        for x in 0..d {
            for y in [0, d - 1] {
                if m[y * d + x] == 1 {
                    m[y * d + x] = 2;
                    q.push(y * d + x);
                }
            }
        }
        for y in 0..d {
            for x in [0, d - 1] {
                if m[y * d + x] == 1 {
                    m[y * d + x] = 2;
                    q.push(y * d + x);
                }
            }
        }
        while let Some(i) = q.pop() {
            let x = i % d;
            let y = i / d;
            if x > 0 && m[i - 1] == 1 {
                m[i - 1] = 2;
                q.push(i - 1);
            }
            if x < d - 1 && m[i + 1] == 1 {
                m[i + 1] = 2;
                q.push(i + 1);
            }
            if y > 0 && m[i - d] == 1 {
                m[i - d] = 2;
                q.push(i - d);
            }
            if y < d - 1 && m[i + d] == 1 {
                m[i + d] = 2;
                q.push(i + d);
            }
        }
        m.iter().filter(|&&v| v == 1).count() as u64 * 16
    };

    log(&format!("downsampling ortho → {d}²"));
    let full = img::load_png_rgb(&ortho_path)?;
    let ortho = img::resize_rgb(&full, d, d);
    drop(full);

    let mut sat = vec![0f32; d * d];
    let mut lum = vec![0f32; d * d];
    for i in 0..d * d {
        let (s, l) = img::hsl_sat_lum(
            ortho.data[i * 3],
            ortho.data[i * 3 + 1],
            ortho.data[i * 3 + 2],
        );
        sat[i] = s;
        lum[i] = l;
    }

    let sea_wide = dilate(&sea, d, OCEAN_DILATE_R);
    let flat_wide = dilate(&flat, d, FLAT_DILATE_R);

    // ── Exact ROAD corridors from the .topo network ──
    let vfs = PakVfs::open_default()?;
    let topo = decode_topo(&vfs, "everon")?; // E2c-allow (spike lane is Eden-only)
    let road_half_w = |ty: u8| -> Option<usize> {
        match ty {
            0 => Some(3),
            1 | 2 => Some(2),
            3 | 5 => Some(1),
            _ => None,
        }
    };
    let mut road_corridor = vec![0u8; d * d];
    let stamp_disc = |mask2: &mut [u8], cx: f64, cy: f64, r: usize| {
        let ri = r as isize;
        let x0 = (js_math_round(cx) as isize - ri).max(0);
        let x1 = (js_math_round(cx) as isize + ri).min(d as isize - 1);
        let y0 = (js_math_round(cy) as isize - ri).max(0);
        let y1 = (js_math_round(cy) as isize + ri).min(d as isize - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if (x as f64 - cx).powi(2) + (y as f64 - cy).powi(2) <= (r * r) as f64 + 0.5 {
                    mask2[y as usize * d + x as usize] = 1;
                }
            }
        }
    };
    let mut road_record_count = 0u64;
    for rec in &topo.records {
        let Some(half_w) = road_half_w(rec.rec_type) else {
            continue;
        };
        let v = &rec.verts;
        let mut s = 0usize;
        while s + 3 < v.len() {
            let (ax, ay) = (f64::from(v[s]) / 4.0, f64::from(v[s + 1]) / 4.0);
            let (bx, by) = (f64::from(v[s + 2]) / 4.0, f64::from(v[s + 3]) / 4.0);
            let steps = ((bx - ax).hypot(by - ay).ceil() as usize).max(1);
            for t in 0..=steps {
                stamp_disc(
                    &mut road_corridor,
                    ax + (bx - ax) * t as f64 / steps as f64,
                    ay + (by - ay) * t as f64 / steps as f64,
                    half_w,
                );
            }
            s += 4;
        }
        road_record_count += 1;
    }
    let sample_frac = |pred: &dyn Fn(usize) -> bool, only_type: Option<u8>| -> f64 {
        let mut hit = 0u64;
        let mut n = 0u64;
        for rec in &topo.records {
            if let Some(ty) = only_type
                && rec.rec_type != ty
            {
                continue;
            }
            let v = &rec.verts;
            let mut s = 0usize;
            while s + 3 < v.len() {
                let (ax, ay) = (f64::from(v[s]) / 4.0, f64::from(v[s + 1]) / 4.0);
                let (bx, by) = (f64::from(v[s + 2]) / 4.0, f64::from(v[s + 3]) / 4.0);
                let steps =
                    (((bx - ax).hypot(by - ay) / ROAD_SAMPLE_STEP_PX).ceil() as usize).max(1);
                for t in 0..=steps {
                    let x = js_math_round(ax + (bx - ax) * t as f64 / steps as f64) as isize;
                    let y = js_math_round(ay + (by - ay) * t as f64 / steps as f64) as isize;
                    if x < 0 || x >= d as isize || y < 0 || y >= d as isize {
                        continue;
                    }
                    n += 1;
                    if pred(y as usize * d + x as usize) {
                        hit += 1;
                    }
                }
                s += 4;
            }
        }
        if n == 0 { 0.0 } else { hit as f64 / n as f64 }
    };
    let airfield_flat_frac =
        (sample_frac(&|i| flat_wide[i] == 1, Some(TOPO_AIRFIELD)) * 1000.0).round() / 1000.0;
    let road_px = road_corridor.iter().filter(|&&v| v == 1).count() as u64;
    log(&format!(
        "topo road corridors: {road_record_count} records → {road_px} px ({airfield_flat_frac} airfield-flat check)"
    ));

    // ── Pixel classes ──
    let mut grey = vec![0u8; d * d];
    let mut wet = vec![0u8; d * d];
    let mut grey_px = 0u64;
    let mut wet_px = 0u64;
    let mut grey_on_sea = 0u64;
    let mut sea_px = 0u64;
    for i in 0..d * d {
        let is_grey = sat[i] < SAT_MAX
            && lum[i] > LUM_MIN
            && lum[i] < LUM_MAX
            && slope[i] <= SLOPE_PX_MAX_DEG;
        if sea[i] == 1 {
            sea_px += 1;
            if is_grey {
                grey_on_sea += 1;
            }
        }
        if sea_wide[i] == 1 {
            continue;
        }
        if is_grey {
            grey[i] = 1;
            grey_px += 1;
        }
        if valley[i] == 1
            && lum[i] > WET_LUM_MIN
            && lum[i] < WET_LUM_MAX
            && sat[i] < WET_SAT_MAX
            && slope[i] <= WET_SLOPE_PX_MAX_DEG
        {
            wet[i] = 1;
            wet_px += 1;
        }
    }
    let grey_ocean_recall = grey_on_sea as f64 / sea_px as f64;
    log(&format!(
        "grey px inland (pre-open): {grey_px}; wet-valley px: {wet_px}; ocean grey recall {:.1} %",
        grey_ocean_recall * 100.0
    ));

    // Speckle-tolerant density opening.
    let density_open = |src: &[u8], r: usize, min_frac: f64| -> Vec<u8> {
        let side = 2 * r + 1;
        let need = ((side * side) as f64 * min_frac).ceil() as i64;
        let mut ii = vec![0i64; (d + 1) * (d + 1)];
        for y in 0..d {
            let mut row = 0i64;
            for x in 0..d {
                row += i64::from(src[y * d + x]);
                ii[(y + 1) * (d + 1) + (x + 1)] = ii[y * (d + 1) + (x + 1)] + row;
            }
        }
        let box_sum = |x0: usize, y0: usize, x1: usize, y1: usize| -> i64 {
            ii[(y1 + 1) * (d + 1) + (x1 + 1)]
                - ii[y0 * (d + 1) + (x1 + 1)]
                - ii[(y1 + 1) * (d + 1) + x0]
                + ii[y0 * (d + 1) + x0]
        };
        let mut core = vec![0u8; d * d];
        for y in 0..d {
            for x in 0..d {
                if src[y * d + x] == 0 {
                    continue;
                }
                let s = box_sum(
                    x.saturating_sub(r),
                    y.saturating_sub(r),
                    (x + r).min(d - 1),
                    (y + r).min(d - 1),
                );
                if s >= need {
                    core[y * d + x] = 1;
                }
            }
        }
        let core_wide = dilate(&core, d, r + 1);
        (0..d * d)
            .map(|i| u8::from(src[i] == 1 && core_wide[i] == 1))
            .collect()
    };
    grey = density_open(&grey, OPEN_R, DENSITY_MIN);
    wet = density_open(&wet, 1, 0.55);
    for i in 0..d * d {
        if wet[i] == 1 {
            grey[i] = 1;
        }
    }

    // Connected components + per-component acceptance.
    struct Comp {
        px: Vec<usize>,
        accepted: bool,
        klass: &'static str,
        road_frac: f64,
        area_m2: u64,
        mean_sat: f64,
        mean_lum: f64,
        mean_slope: f64,
        mean_elev: f64,
        flat_frac: f64,
        valley_frac: f64,
        ribbon_w: f64,
        bbox: [usize; 4],
        centre: [f64; 2],
    }
    let mut labels = vec![-1i32; d * d];
    let mut comps: Vec<Comp> = Vec::new();
    for i in 0..d * d {
        if grey[i] == 0 || labels[i] != -1 {
            continue;
        }
        let id = comps.len() as i32;
        let mut st = vec![i];
        labels[i] = id;
        let mut px = Vec::new();
        while let Some(k) = st.pop() {
            px.push(k);
            let x = k % d;
            let y = k / d;
            for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as i64 + dx;
                let ny = y as i64 + dy;
                if nx < 0 || nx >= d as i64 || ny < 0 || ny >= d as i64 {
                    continue;
                }
                let j = (ny * d as i64 + nx) as usize;
                if grey[j] == 1 && labels[j] == -1 {
                    labels[j] = id;
                    st.push(j);
                }
            }
        }
        let (mut s_sat, mut s_slope, mut s_elev, mut s_lum) = (0f64, 0f64, 0f64, 0f64);
        let (mut n_flat, mut n_valley, mut n_road, mut perim) = (0u64, 0u64, 0u64, 0u64);
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (d, 0usize, d, 0usize);
        for &k in &px {
            s_sat += f64::from(sat[k]);
            s_slope += f64::from(slope[k]);
            s_elev += f64::from(elev_m[k]);
            s_lum += f64::from(lum[k]);
            if flat_wide[k] == 1 {
                n_flat += 1;
            }
            if valley[k] == 1 {
                n_valley += 1;
            }
            if road_corridor[k] == 1 {
                n_road += 1;
            }
            let x = k % d;
            let y = k / d;
            if x == 0
                || x == d - 1
                || y == 0
                || y == d - 1
                || grey[k - 1] == 0
                || grey[k + 1] == 0
                || grey[k - d] == 0
                || grey[k + d] == 0
            {
                perim += 1;
            }
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let np = px.len() as f64;
        let mean_sat = s_sat / np;
        let mean_slope = s_slope / np;
        let flat_frac = n_flat as f64 / np;
        let valley_frac = n_valley as f64 / np;
        let area_m2 = px.len() as u64 * 16;
        let ribbon_w = 2.0 * np / (perim.max(1)) as f64;
        let is_linear = ribbon_w <= RIBBON_W_MAX_PX;
        let road_frac = n_road as f64 / np;
        let is_grey_river = mean_sat <= MEAN_SAT_MAX;
        let (klass, mut accepted) = if !is_linear {
            (
                "compact",
                area_m2 >= MIN_AREA_M2
                    && mean_sat <= MEAN_SAT_MAX
                    && flat_frac <= FLAT_FRAC_MAX
                    && mean_slope <= SLOPE_MEAN_MAX_DEG,
            )
        } else if is_grey_river {
            (
                "grey-river",
                area_m2 >= LIN_MIN_AREA_M2
                    && mean_slope <= LIN_SLOPE_MEAN_MAX_DEG
                    && flat_frac <= LIN_FLAT_FRAC_MAX
                    && (valley_frac >= GREY_RIVER_VALLEY_MIN
                        || mean_slope <= GREY_RIVER_LOWLAND_SLOPE_DEG),
            )
        } else {
            (
                "wet-channel",
                area_m2 >= WET_MIN_AREA_M2
                    && mean_slope <= LIN_SLOPE_MEAN_MAX_DEG
                    && flat_frac <= LIN_FLAT_FRAC_MAX
                    && valley_frac >= WET_VALLEY_FRAC_MIN
                    && mean_sat <= WET_MEAN_SAT_MAX
                    && s_lum / np <= WET_MEAN_LUM_MAX,
            )
        };
        if accepted && road_frac > ROAD_OVERLAP_MAX {
            accepted = false;
        }
        comps.push(Comp {
            accepted,
            klass,
            road_frac: (road_frac * 1000.0).round() / 1000.0,
            area_m2,
            mean_sat: (mean_sat * 10000.0).round() / 10000.0,
            mean_lum: ((s_lum / np) * 1000.0).round() / 1000.0,
            mean_slope: (mean_slope * 100.0).round() / 100.0,
            mean_elev: ((s_elev / np) * 10.0).round() / 10.0,
            flat_frac: (flat_frac * 1000.0).round() / 1000.0,
            valley_frac: (valley_frac * 1000.0).round() / 1000.0,
            ribbon_w: (ribbon_w * 100.0).round() / 100.0,
            bbox: [min_x * 4, min_y * 4, (max_x + 1) * 4, (max_y + 1) * 4],
            centre: [
                ((min_x + max_x) as f64 / 2.0) * 4.0,
                12800.0 - ((min_y + max_y) as f64 / 2.0) * 4.0,
            ],
            px,
        });
    }
    let mut accepted: Vec<&Comp> = comps.iter().filter(|c| c.accepted).collect();
    accepted.sort_by_key(|c| std::cmp::Reverse(c.area_m2));
    log(&format!(
        "components: {} total, {} accepted (>={MIN_AREA_M2} m², meanSat<={MEAN_SAT_MAX}, flatFrac<={FLAT_FRAC_MAX})",
        comps.len(),
        accepted.len()
    ));
    for c in accepted.iter().take(16) {
        log(&format!(
            "  {:<7} {:.1} ha @ world ({}, {}) sat={} slope={}° flat={} valley={} w={}px",
            c.klass,
            c.area_m2 as f64 / 1e4,
            js_num(c.centre[0]),
            js_num(c.centre[1]),
            js_num(c.mean_sat),
            js_num(c.mean_slope),
            js_num(c.flat_frac),
            js_num(c.valley_frac),
            js_num(c.ribbon_w)
        ));
    }

    // ── Mask (3200² → 12800² threshold upscale) + preview ──
    let mut mask3200 = vec![0u8; d * d];
    for c in &accepted {
        for &k in &c.px {
            mask3200[k] = 255;
        }
    }
    {
        // nearest ×4 upscale ≡ resize+threshold on a binary plane
        let big = 12800usize;
        let mut mask_big = vec![0u8; big * big];
        for y in 0..big {
            let sy = y / 4;
            for x in 0..big {
                mask_big[y * big + x] = mask3200[sy * d + x / 4];
            }
        }
        img::save_png_gray(&out_mask, big, big, &mask_big)?;
        // preview: 1600² ortho with the mask tinted blue on top
        let prev = img::resize_rgb(&ortho, 1600, 1600);
        let mut pv = prev.data;
        for y in 0..1600 {
            let sy = y * d / 1600;
            for x in 0..1600 {
                if mask3200[sy * d + x * d / 1600] > 64 {
                    let o = (y * 1600 + x) * 3;
                    pv[o] = 0x22;
                    pv[o + 1] = 0x66;
                    pv[o + 2] = 0xff;
                }
            }
        }
        img::save_png_rgb(
            &out_preview,
            &Rgb8 {
                w: 1600,
                h: 1600,
                data: pv,
            },
        )?;
    }
    log(&format!(
        "wrote {} + {}",
        out_mask.display(),
        out_preview.display()
    ));

    // ── Spike JSON ──
    let comp_json = |c: &Comp| -> Value {
        json!({
            "accepted": c.accepted, "class": c.klass, "roadFrac": js_num(c.road_frac),
            "areaM2": c.area_m2, "meanSat": js_num(c.mean_sat), "meanLum": js_num(c.mean_lum),
            "meanSlopeDeg": js_num(c.mean_slope), "meanElevM": js_num(c.mean_elev),
            "flatFrac": js_num(c.flat_frac), "valleyFrac": js_num(c.valley_frac),
            "ribbonWidthPx": js_num(c.ribbon_w),
            "bboxOrthoPx": c.bbox, "centreWorldM": [js_num(c.centre[0]), js_num(c.centre[1])],
        })
    };
    let mut comparison = Value::Null;
    if prev_spike.exists() {
        let prev: Value = serde_json::from_str(&std::fs::read_to_string(&prev_spike)?)?;
        let prev_bodies = prev["results"]["acceptedBodies"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let near = |a: &[f64; 2], b: &Value| {
            let bx = b[0].as_f64().unwrap_or(f64::MAX);
            let by = b[1].as_f64().unwrap_or(f64::MAX);
            (a[0] - bx).hypot(a[1] - by) <= 250.0
        };
        let mut retained = 0u64;
        let mut dropped: Vec<Value> = Vec::new();
        for p in &prev_bodies {
            if accepted.iter().any(|c| near(&c.centre, &p["centreWorldM"])) {
                retained += 1;
            } else {
                dropped.push(json!({ "centreWorldM": p["centreWorldM"], "areaM2": p["areaM2"], "class": p["class"] }));
            }
        }
        let new_bodies = accepted
            .iter()
            .filter(|c| {
                !prev_bodies
                    .iter()
                    .any(|p| near(&c.centre, &p["centreWorldM"]))
            })
            .count();
        comparison = json!({
            "prevAccepted": prev_bodies.len(), "retained": retained, "dropped": dropped, "newBodies": new_bodies,
        });
    }
    let spike = json!({
        "slice": "T-090.1.2.5.2",
        "parent": "T-090.1.2.5 + .2.5.1 spikes: .ai/artifacts/t090_1_2_5_water_source_spike.json / t090_1_2_5_1_refine_spike.json (shipped history, unchanged)",
        "generatedAt": iso_from_system_time(std::time::SystemTime::now()),
        "decision": {
            "verdict": "G1-B — Eden.topo carries the full ROAD network but NO hydro layer; exact road-corridor SUBTRACTION removes the path/ditch FP class deterministically, enabling a safe wet-channel relaxation that closes the hill-stream/gully FN gap",
            "oceanMask": "A-dem-below-sea-level (UNCHANGED)",
            "inlandMask": "appearance classes (compact + grey-river + wet-channel) computed on the ROAD-SUBTRACTED pixel field; wet-channel relaxed (operator call: carved gully watercourses read as water even when seasonally dry)",
            "automation": "fully offline: pak (.topo + supertextures) + committed DEM → make map-water-everon; terrain-parameterized (operator one-button requirement) — T-165.9 Rust",
            "forbiddenMethodsAttestation": "No hand-painted lakes, no AI-generated rivers, no solid rectangles. The subtraction layer is the engine's own map-geometry road network decoded from Eden.topo; water acceptance remains engine-rendered supertexture appearance + engine DEM filters.",
        },
        "params": {
            "roadSubtraction": { "halfWidthPxByType": { "0": 3, "1": 2, "2": 2, "3": 1, "5": 1 }, "gridMetersPerPx": 4, "roadOverlapMax": ROAD_OVERLAP_MAX },
            "compactClass": {
                "detectDim": DETECT_DIM, "satMax": SAT_MAX, "lumMin": LUM_MIN, "lumMax": LUM_MAX,
                "openRadiusPx": OPEN_R, "densityMin": DENSITY_MIN, "oceanDilateRadiusPx": OCEAN_DILATE_R,
                "flatDilateRadiusPx": FLAT_DILATE_R, "minAreaM2": MIN_AREA_M2, "meanSatMax": MEAN_SAT_MAX,
                "flatFracMax": FLAT_FRAC_MAX, "slopePxMaxDeg": js_num(f64::from(SLOPE_PX_MAX_DEG)),
                "slopeMeanMaxDeg": js_num(SLOPE_MEAN_MAX_DEG), "ribbonWidthMaxPx": js_num(RIBBON_W_MAX_PX),
            },
            "greyRiver": {
                "linMinAreaM2": LIN_MIN_AREA_M2, "linSlopeMeanMaxDeg": js_num(LIN_SLOPE_MEAN_MAX_DEG),
                "linFlatFracMax": LIN_FLAT_FRAC_MAX, "greyRiverValleyMin": GREY_RIVER_VALLEY_MIN,
                "greyRiverLowlandSlopeDeg": js_num(GREY_RIVER_LOWLAND_SLOPE_DEG),
            },
            "wetChannelRelaxed": {
                "wetMinAreaM2": { "old251": 1200, "new": WET_MIN_AREA_M2 },
                "wetValleyFracMin": { "old251": 0.7, "new": WET_VALLEY_FRAC_MIN },
                "wetMeanSatMax": { "old251": 0.16, "new": WET_MEAN_SAT_MAX },
                "wetMeanLumMax": { "old251": 0.28, "new": WET_MEAN_LUM_MAX },
                "wetPxBand": { "lum": [WET_LUM_MIN, WET_LUM_MAX], "satMax": WET_SAT_MAX, "slopeMaxDeg": js_num(f64::from(WET_SLOPE_PX_MAX_DEG)) },
                "valleyBlurRadiusPx": VALLEY_BLUR_R,
                "valleyCarveM": WET_VALLEY_CARVE_JSON,
            },
        },
        "results": {
            "topoValidation": {
                "airfieldOnEngineFlatFrac": js_num(airfield_flat_frac),
                "note": "type-1 colour-overlay crosses ridges + connects airfield/towns → highways (no hydro layer in .topo); all 5 classes rasterized as exclusion corridors",
            },
            "roadCorridorPx": road_px,
            "greyOceanRecall": js_num((grey_ocean_recall * 1000.0).round() / 1000.0),
            "seaFraction": js_num((sea_fraction * 10000.0).round() / 10000.0),
            "inlandBelowSeaM2": inland_below_sea_m2,
            "acceptedBodies": accepted.iter().map(|c| comp_json(c)).collect::<Vec<_>>(),
            "rejectedComponentCount": comps.len() - accepted.len(),
            "comparisonVsShip251": comparison,
        },
        "outputs": {
            "inlandMaskPng": "packages/map-assets/everon/staging/sap/water-inland-mask.png (gitignored)",
            "previewPng": "packages/map-assets/everon/staging/sap/water-spike-preview.png (gitignored)",
        },
    });
    std::fs::write(&out_json, serde_json::to_string_pretty(&spike)? + "\n")?;
    log(&format!(
        "wrote {} in {:.0}s",
        out_json.display(),
        t0.elapsed().as_secs_f64()
    ));
    Ok(0)
}

// valleyCarveM prints as 0.8 in JS.
const WET_VALLEY_CARVE_JSON: f64 = 0.8;
