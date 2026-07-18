//! T-165.9 — the cartographic lane: `build-landcover-mask.mjs` (SAP-appearance forest/bright
//! masks), `build-map-cartographic.mjs` (TGA base → landcover tints → Lanczos upscale →
//! inland-water tint → .topo road strokes via resvg — replaces the magick MVG draw pass),
//! and the tile-pyramid builder (`build-tile-pyramid.sh` — XYZ WebP levels; lossless via
//! image-webp, the lossy leg via the vendored-libwebp `webp` crate per N3).

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::img::{self, Rgb8};
use crate::serve::repo_root;
use crate::world::aux::iso_from_system_time;
use crate::world::jsval::js_num;
use crate::world::pak::PakVfs;
use crate::world::topo::decode_topo;

pub const CLASS_PX: usize = 3200;
const FOREST_LUM_MAX: f64 = 52.0;
const FOREST_GREEN_OVER_BLUE: u16 = 8;
const BRIGHT_RED_OVER_GREEN: u16 = 4;
const BRIGHT_LUM_MIN: f64 = 58.0;

pub struct LandcoverOut {
    pub forest_mask: PathBuf,
    pub bright_mask: PathBuf,
    pub meta: Value,
}

/// build-landcover-mask.mjs port — classification at CLASS_PX (nearest sample), close-then-
/// open morphology, soft-edge masks + meta JSON.
pub fn build_landcover_masks(terrain: &str) -> Result<LandcoverOut> {
    if terrain != "everon" {
        bail!("build-landcover-mask: no SAP source registered for terrain \"{terrain}\"");
    }
    let root = repo_root();
    let sap_rel = "packages/map-assets/everon/staging/sap/everon-sap-ortho.png"; // E2c-allow
    let sap = root.join(sap_rel);
    if !sap.exists() {
        bail!(
            "build-landcover-mask: SAP ortho missing: {sap_rel}\nstaging/ is gitignored — restore it (make map-water-everon rebuilds the water composite)."
        );
    }
    let out_dir = root.join("packages/map-assets/everon/staging/map"); // E2c-allow
    std::fs::create_dir_all(&out_dir)?;
    let started = std::time::Instant::now();

    let full = img::load_png_rgb(&sap)?;
    let smp = img::sample_rgb(&full, CLASS_PX, CLASS_PX);
    drop(full);
    let n = CLASS_PX * CLASS_PX;
    let mut forest = vec![0f32; n];
    let mut bright = vec![0f32; n];
    let (mut c_forest, mut c_bright, mut c_water, mut c_grass) = (0u64, 0u64, 0u64, 0u64);
    for i in 0..n {
        let r = u16::from(smp.data[i * 3]);
        let g = u16::from(smp.data[i * 3 + 1]);
        let b = u16::from(smp.data[i * 3 + 2]);
        if b >= g {
            c_water += 1;
            continue;
        }
        let l = f64::from(r + g + b) / 3.0;
        if g > r && g > b + FOREST_GREEN_OVER_BLUE && l <= FOREST_LUM_MAX {
            forest[i] = 1.0;
            c_forest += 1;
        } else if r >= g + BRIGHT_RED_OVER_GREEN && l >= BRIGHT_LUM_MIN {
            bright[i] = 1.0;
            c_bright += 1;
        } else {
            c_grass += 1;
        }
    }
    let close_open = |plane: &[f32]| -> Vec<f32> {
        let mut p = img::box_blur_f32(plane, CLASS_PX, CLASS_PX, 6);
        for v in &mut p {
            *v = if *v >= 0.35 { 1.0 } else { 0.0 };
        }
        let mut p = img::box_blur_f32(&p, CLASS_PX, CLASS_PX, 6);
        for v in &mut p {
            *v = if *v >= 0.6 { 1.0 } else { 0.0 };
        }
        img::box_blur_f32(&p, CLASS_PX, CLASS_PX, 2)
    };
    let forest = close_open(&forest);
    let bright = close_open(&bright);
    let write_mask = |plane: &[f32], path: &Path| -> Result<()> {
        let data: Vec<u8> = plane
            .iter()
            .map(|&v| {
                crate::world::jsval::js_math_round(f64::from(v.clamp(0.0, 1.0)) * 255.0) as u8
            })
            .collect();
        img::save_png_gray(path, CLASS_PX, CLASS_PX, &data)
    };
    let forest_out = out_dir.join("landcover-forest-mask.png");
    let bright_out = out_dir.join("landcover-bright-mask.png");
    write_mask(&forest, &forest_out)?;
    write_mask(&bright, &bright_out)?;
    let frac = |c: u64| js_num((c as f64 / n as f64 * 10000.0).round() / 10000.0);
    let meta = json!({
        "slice": "T-090.1.1.1",
        "source": sap_rel,
        "classPx": CLASS_PX,
        "thresholds": {
            "water": "b >= g (excluded)",
            "forest": format!("g > r && g > b+{FOREST_GREEN_OVER_BLUE} && L <= {FOREST_LUM_MAX}"),
            "bright": format!("r >= g+{BRIGHT_RED_OVER_GREEN} && L >= {BRIGHT_LUM_MIN}"),
        },
        "fractions": { "forest": frac(c_forest), "bright": frac(c_bright), "grass": frac(c_grass), "water": frac(c_water) },
        "buildSeconds": started.elapsed().as_secs(),
        "generatedAt": iso_from_system_time(std::time::SystemTime::now()),
    });
    std::fs::write(
        out_dir.join("landcover-mask-meta.json"),
        serde_json::to_string_pretty(&meta)? + "\n",
    )?;
    Ok(LandcoverOut {
        forest_mask: forest_out,
        bright_mask: bright_out,
        meta,
    })
}

pub fn build_landcover_cli(terrain: &str) -> Result<u8> {
    let out = build_landcover_masks(terrain)?;
    let f = &out.meta["fractions"];
    println!(
        "build-landcover-mask: OK {terrain} @ {CLASS_PX}² — fractions forest={} bright={} grass={} water={} ({}s)",
        f["forest"], f["bright"], f["grass"], f["water"], out.meta["buildSeconds"]
    );
    Ok(0)
}

/* ─────────────────────────── build-map-cartographic ─────────────────────────── */

/// despike (see build-map-cartographic.mjs): duplicate-drop, return-spike drop, width-stub
/// perpendicular-excursion drop.
fn despike(verts: &[f32]) -> Vec<(f64, f64)> {
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(verts.len() / 2);
    for i in (0..verts.len()).step_by(2) {
        pts.push((f64::from(verts[i]), f64::from(verts[i + 1])));
    }
    let d2 = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2);
    let mut filtered: Vec<(f64, f64)> = Vec::new();
    for (i, &p) in pts.iter().enumerate() {
        if i == 0 || d2(p, filtered[filtered.len() - 1]) > 0.01 {
            filtered.push(p);
        }
    }
    let mut pts = filtered;
    let perp2 = |p: (f64, f64), a: (f64, f64), b: (f64, f64)| -> f64 {
        let abx = b.0 - a.0;
        let aby = b.1 - a.1;
        let len2 = abx * abx + aby * aby;
        if len2 < 1e-6 {
            return d2(p, a);
        }
        let t = (((p.0 - a.0) * abx + (p.1 - a.1) * aby) / len2).clamp(0.0, 1.0);
        d2(p, (a.0 + t * abx, a.1 + t * aby))
    };
    let mut changed = true;
    while changed {
        changed = false;
        if pts.len() < 3 {
            break;
        }
        let mut keep = vec![pts[0]];
        for i in 1..pts.len() - 1 {
            let prev = keep[keep.len() - 1];
            let next = pts[i + 1];
            if d2(prev, next) < 1.0 || perp2(pts[i], prev, next) > 3.5f64.powi(2) {
                changed = true;
            } else {
                keep.push(pts[i]);
            }
        }
        keep.push(pts[pts.len() - 1]);
        let mut out: Vec<(f64, f64)> = Vec::new();
        for (i, &p) in keep.iter().enumerate() {
            if i == 0 || d2(p, out[out.len() - 1]) > 0.01 {
                out.push(p);
            }
        }
        pts = out;
    }
    pts
}

fn road_style(ty: u8) -> Option<(&'static str, u32)> {
    match ty {
        0 => Some(("#9aa3a2", 20)),
        1 => Some(("#b0452b", 10)),
        2 => Some(("#c8823c", 8)),
        3 => Some(("#ded6bd", 5)),
        5 => Some(("#7a7466", 3)),
        _ => None,
    }
}

const WATER_COLOR: [f64; 3] = [0x2e as f64, 0x52 as f64, 0x66 as f64];
const OPEN_TINT: ([f64; 3], f64) = ([0xcd as f64, 0xc6 as f64, 0xa3 as f64], 0.7);
const FOREST_TINT: ([f64; 3], f64) = ([0x37 as f64, 0x50 as f64, 0x2d as f64], 0.8);

pub fn build_map_cartographic(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!(
            "build-map-cartographic: no cartographic source registered for terrain \"{terrain}\".\nExport one first (Workbench → Plugins → TBD → \"Export TBD Satellite\") and add a source row."
        );
        return Ok(1);
    }
    let root = repo_root();
    let tga = root.join("packages/map-assets/everon/staging/spike/TBD_SatExport_everon.tga"); // E2c-allow
    let out = root.join("packages/map-assets/everon/staging/map/everon-map-ortho.png"); // E2c-allow
    let water_mask_path = root.join("packages/map-assets/everon/staging/sap/water-inland-mask.png"); // E2c-allow
    let (world_px, source_px) = (12800usize, 4096usize);
    if !tga.exists() {
        eprintln!(
            "build-map-cartographic: source raster missing: {}\nstaging/ is gitignored (local scratch) — regenerate via the Workbench export.",
            tga.display()
        );
        return Ok(1);
    }
    let started = std::time::Instant::now();
    std::fs::create_dir_all(out.parent().unwrap())?;

    let vfs = PakVfs::open_default()?;
    let topo = decode_topo(&vfs, terrain)?;
    let landcover = build_landcover_masks(terrain)?;

    // ── Base + landcover tints at source res ──
    let base_dyn = {
        let mut reader = image::ImageReader::open(&tga)?;
        reader.no_limits();
        reader.decode()?
    };
    let mut base = Rgb8 {
        w: base_dyn.width() as usize,
        h: base_dyn.height() as usize,
        data: base_dyn.to_rgb8().into_raw(),
    };
    if base.w != source_px || base.h != source_px {
        bail!("TGA {}x{} != {source_px}²", base.w, base.h);
    }
    for (mask_path, (color, alpha)) in [
        (&landcover.bright_mask, OPEN_TINT),
        (&landcover.forest_mask, FOREST_TINT),
    ] {
        let m = img::load_png_rgb(mask_path)?;
        // masks are CLASS_PX² grayscale → resize to source, multiply by tint alpha, Over.
        let m = img::resize_rgb(&m, source_px, source_px);
        for i in 0..source_px * source_px {
            let a = f64::from(m.data[i * 3]) / 255.0 * alpha;
            if a <= 0.0 {
                continue;
            }
            let o = i * 3;
            for (c, col) in color.iter().enumerate() {
                base.data[o + c] = crate::world::jsval::js_math_round(
                    f64::from(base.data[o + c]) * (1.0 - a) + col * a,
                ) as u8;
            }
        }
    }

    // ── Upscale → world extent, inland-water tint ──
    let mut world = img::resize_rgb(&base, world_px, world_px);
    drop(base);
    let has_water = water_mask_path.exists();
    if has_water {
        let wm = img::load_png_rgb(&water_mask_path)?;
        if wm.w != world_px {
            bail!("water mask {}x{} != {world_px}²", wm.w, wm.h);
        }
        for i in 0..world_px * world_px {
            let a = f64::from(wm.data[i * 3]) / 255.0;
            if a <= 0.0 {
                continue;
            }
            let o = i * 3;
            for (c, col) in WATER_COLOR.iter().enumerate() {
                world.data[o + c] = crate::world::jsval::js_math_round(
                    f64::from(world.data[o + c]) * (1.0 - a) + col * a,
                ) as u8;
            }
        }
    } else {
        eprintln!(
            "build-map-cartographic: water mask missing — shipping without inland-water tint"
        );
    }

    // ── Road strokes via resvg (replaces the magick MVG pass) ──
    let mut svg = String::from(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"12800\" height=\"12800\" viewBox=\"0 0 12800 12800\">",
    );
    let mut drawn_records = 0u64;
    let mut drawn_verts = 0u64;
    let mut raw_verts = 0u64;
    for ty in [0u8, 5, 3, 2, 1] {
        let Some((color, width)) = road_style(ty) else {
            continue;
        };
        for rec in &topo.records {
            if rec.rec_type != ty || rec.verts.len() < 4 {
                continue;
            }
            let pts = despike(&rec.verts);
            if pts.len() < 2 {
                continue;
            }
            let path: String = pts
                .iter()
                .map(|(x, y)| format!("{:.1},{:.1}", x, y))
                .collect::<Vec<_>>()
                .join(" ");
            svg.push_str(&format!(
                "<polyline fill=\"none\" stroke=\"{color}\" stroke-width=\"{width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" points=\"{path}\"/>"
            ));
            drawn_records += 1;
            drawn_verts += pts.len() as u64;
            raw_verts += rec.verts.len() as u64 / 2;
        }
    }
    svg.push_str("</svg>");
    let _ = raw_verts;
    let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &resvg::usvg::Options::default())
        .map_err(|e| anyhow::anyhow!("road svg: {e}"))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(world_px as u32, world_px as u32)
        .ok_or_else(|| anyhow::anyhow!("pixmap {world_px}²"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let pm = pixmap.data(); // premultiplied RGBA
    for i in 0..world_px * world_px {
        let a = u32::from(pm[i * 4 + 3]);
        if a == 0 {
            continue;
        }
        let o = i * 3;
        for c in 0..3 {
            // dst = src + dst*(1-a) with premultiplied src
            let src = u32::from(pm[i * 4 + c]);
            let dst = u32::from(world.data[o + c]);
            world.data[o + c] = (src + dst * (255 - a) / 255).min(255) as u8;
        }
    }
    img::save_png_rgb(&out, &world)?;

    let meta = json!({
        "slice": "T-090.1.1.1",
        "source": "workbench-cartographic",
        "terrain": terrain,
        "sourceRaster": "packages/map-assets/everon/staging/spike/TBD_SatExport_everon.tga",
        "sourceDimensions": [source_px, source_px],
        "dimensions": [world_px, world_px],
        "worldBounds": [0, 0, world_px, world_px],
        "upscale": format!("{source_px}->{world_px} Lanczos (documented upscale, slice spec §1) — T-165.9 Rust"),
        "orientation": "north-up (TGA top origin preserved; no flips on this path)",
        "overlays": {
            "landCover": {
                "source": "build-landcover-mask (SAP appearance heuristic, L1) — T-165.9 Rust",
                "thresholds": landcover.meta["thresholds"],
                "fractions": landcover.meta["fractions"],
                "style": { "open": { "color": "#CDC6A3", "alpha": 0.7 }, "forest": { "color": "#37502D", "alpha": 0.8 } },
                "provenance": "T-090.1.1.1 — SAP ortho read-only; satellite bundle untouched",
            },
            "inlandWater": if has_water {
                json!({ "mask": "packages/map-assets/everon/staging/sap/water-inland-mask.png", "color": "#2E5266", "provenance": "T-090.1.2.5.2 classifier (read-only reuse)" })
            } else {
                Value::Null
            },
            "roads": {
                "source": "world::topo (.topo vector network) — T-165.9 Rust",
                "records": drawn_records,
                "vertices": drawn_verts,
                "style": { "0": { "color": "#9aa3a2", "width": 20 }, "1": { "color": "#b0452b", "width": 10 }, "2": { "color": "#c8823c", "width": 8 }, "3": { "color": "#ded6bd", "width": 5 }, "5": { "color": "#7a7466", "width": 3 } },
            },
        },
        "spikeArtifact": ".ai/artifacts/t090_1_1_1_source_spike.json",
        "buildSeconds": started.elapsed().as_secs(),
        "generatedAt": iso_from_system_time(std::time::SystemTime::now()),
    });
    std::fs::write(
        out.parent().unwrap().join("map-ortho-meta.json"),
        serde_json::to_string_pretty(&meta)? + "\n",
    )?;
    let out_rel = "packages/map-assets/everon/staging/map/everon-map-ortho.png"; // E2c-allow
    println!(
        "build-map-cartographic: OK {out_rel} ({world_px}² north-up, {drawn_records} road records / {drawn_verts} verts, water={has_water}, {}s)",
        meta["buildSeconds"]
    );
    Ok(0)
}

/* ─────────────────────────── build-tile-pyramid ─────────────────────────── */

/// build-tile-pyramid.sh port: XYZ WebP levels from a full-extent ortho (+full.webp).
#[allow(clippy::too_many_arguments)]
pub fn build_tile_pyramid(
    input: &Path,
    out: &Path,
    min_zoom: u32,
    max_zoom: u32,
    tile: usize,
    quality: f32,
    lossless: bool,
    flip_v: bool,
) -> Result<u8> {
    if !input.exists() {
        eprintln!("input not found: {}", input.display());
        return Ok(1);
    }
    let enc_desc = if lossless {
        "lossless".to_string()
    } else {
        format!("q={quality}")
    };
    let src_dyn = {
        let mut reader = image::ImageReader::open(input)?;
        reader.no_limits();
        reader.decode()?
    };
    let mut norm = Rgb8 {
        w: src_dyn.width() as usize,
        h: src_dyn.height() as usize,
        data: src_dyn.to_rgb8().into_raw(),
    };
    if flip_v {
        let stride = norm.w * 3;
        for y in 0..norm.h / 2 {
            let (top, bottom) = (y * stride, (norm.h - 1 - y) * stride);
            for i in 0..stride {
                norm.data.swap(top + i, bottom + i);
            }
        }
    }
    println!(
        "[pyramid] source {}x{}; tile={tile} enc={enc_desc} zoom {min_zoom}..{max_zoom}",
        norm.w, norm.h
    );
    let _ = std::fs::remove_dir_all(out);
    std::fs::create_dir_all(out)?;

    let mut total = 0u64;
    for z in min_zoom..=max_zoom {
        let n = 1usize << z;
        let side = n * tile;
        let level = img::resize_rgb(&norm, side, side);
        println!("[pyramid] z={z}  {n}x{n} tiles ({side}px)");
        for x in 0..n {
            std::fs::create_dir_all(out.join(format!("{z}/{x}")))?;
        }
        for ty in 0..n {
            for tx in 0..n {
                let cell = img::crop_rgb(&level, tx * tile, ty * tile, tile, tile)?;
                let bytes = if lossless {
                    img::encode_webp_lossless_rgb(&cell)?
                } else {
                    img::encode_webp_lossy_rgb(&cell, quality)
                };
                std::fs::write(out.join(format!("{z}/{tx}/{ty}.webp")), bytes)?;
            }
        }
        total += (n * n) as u64;
    }

    // full.webp (≤4096 px edge).
    let fe = norm.w.min(4096);
    let full = if fe == norm.w {
        norm
    } else {
        img::resize_rgb(&norm, fe, fe)
    };
    let full_bytes = if lossless {
        img::encode_webp_lossless_rgb(&full)?
    } else {
        img::encode_webp_lossy_rgb(&full, quality)
    };
    std::fs::write(out.join("full.webp"), full_bytes)?;
    println!("[pyramid] wrote full.webp ({fe}px)");
    println!("[pyramid] wrote {total} tiles to {}", out.display());
    if !out.join("0/0/0.webp").exists() {
        eprintln!("[pyramid] FAIL: missing {}/0/0/0.webp", out.display());
        return Ok(1);
    }
    println!("[pyramid] OK  0/0/0.webp + full.webp present");
    Ok(0)
}

/* ─────────────────────────── Makefile inline-patch helpers (were `node -e`) ─────────────────────────── */

/// `make map-water-everon` step 2: drop the one-shot waterComposite block from the SAP meta.
pub fn reset_water_meta(terrain: &str) -> Result<u8> {
    let p = repo_root()
        .join("packages/map-assets")
        .join(terrain)
        .join("staging/sap/TBD_SatExport_meta.json");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&p)?)?;
    if let Some(obj) = m.as_object_mut() {
        obj.remove("waterComposite");
    }
    std::fs::write(&p, serde_json::to_string_pretty(&m)? + "\n")?;
    Ok(0)
}

/// `make map-water-everon` step 5: manifest.tiles.satellite.unified.bytes = bundle size.
pub fn patch_unified_bytes(terrain: &str) -> Result<u8> {
    let root = repo_root().join("packages/map-assets").join(terrain);
    let mp = root.join("manifest.json");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&mp)?)?;
    let bundle = root.join(
        m["tiles"]["satellite"]["unified"]["path"]
            .as_str()
            .unwrap_or("satellite/everon-sat.tbd-sat"),
    );
    m["tiles"]["satellite"]["unified"]["bytes"] = json!(std::fs::metadata(&bundle)?.len());
    std::fs::write(&mp, serde_json::to_string_pretty(&m)? + "\n")?;
    Ok(0)
}

/// `make map-cartographic-everon` step 3: tiles.map {source, encoding} patch.
pub fn patch_map_tiles_meta(terrain: &str) -> Result<u8> {
    let mp = repo_root()
        .join("packages/map-assets")
        .join(terrain)
        .join("manifest.json");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&mp)?)?;
    let map_block = m["tiles"]["map"]
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("manifest tiles.map missing"))?;
    map_block.insert("source".into(), json!("workbench-cartographic"));
    map_block.insert("encoding".into(), json!("webp-lossy"));
    std::fs::write(&mp, serde_json::to_string_pretty(&m)? + "\n")?;
    Ok(0)
}

/* ─────────────────────────── verify-t152-cartographic ─────────────────────────── */

/// verify-t152-cartographic.mjs port — slice-log checks + committed-data sub-verifiers.
pub fn verify_t152() -> Result<u8> {
    let root = repo_root();
    let artifacts = root.join(".ai/artifacts");
    let failures = std::cell::Cell::new(0usize);
    macro_rules! pass {
        ($($a:tt)*) => { println!("  PASS  {}", format!($($a)*)) };
    }
    macro_rules! failm {
        ($($a:tt)*) => {{ failures.set(failures.get() + 1); println!("  FAIL  {}", format!($($a)*)); }};
    }

    println!("verify-t152-cartographic: slice logs (G1 subset)");
    for i in 0..10 {
        let path = artifacts.join(format!("t152_{i}_verify_log.md"));
        let label = format!("T-152.{i}");
        if !path.exists() {
            failm!("{label} missing {}", path.display());
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        let auto = match text.find("\n## Manual") {
            Some(idx) => &text[..idx],
            None => &text[..],
        };
        if auto.contains("**FAIL**") {
            failm!("{label} verify log contains **FAIL** in automated section");
            continue;
        }
        let gate_pass_rows = auto
            .lines()
            .filter(|l| l.starts_with("| **G") && l.contains("| **PASS**"))
            .count();
        let gate_fail_rows = auto
            .lines()
            .filter(|l| l.starts_with("| **G") && l.contains("| **FAIL**"))
            .count();
        let verdict_ok = gate_fail_rows == 0
            && (gate_pass_rows > 0
                || text.to_lowercase().contains("all gn pass")
                || text.to_lowercase().contains("all automated gn pass")
                || text.to_lowercase().contains("automated gn all **pass**")
                || text
                    .to_lowercase()
                    .contains(&format!("tag **t-152.{i}** allowed"))
                || (i == 0 && text.contains("**ALL Gn PASS**"))
                || (i == 2 && text.contains("**G7**") && text.contains("**PASS**")));
        if !verdict_ok {
            failm!("{label} verify log missing PASS verdict / ship marker");
            continue;
        }
        pass!(
            "{label} log OK ({})",
            path.strip_prefix(&root).unwrap_or(&path).display()
        );
    }

    let run_make = |target: &str, envs: &[(&str, &str)]| {
        let mut cmd = std::process::Command::new("make");
        cmd.arg(target).current_dir(&root);
        for (k, v) in envs {
            cmd.env(k, v);
        }
        let r = cmd.output().expect("spawn make");
        if r.status.success() {
            pass!("make {target} exit 0");
        } else {
            failm!("make {target} exit {}", r.status.code().unwrap_or(1));
            let err = String::from_utf8_lossy(&r.stderr);
            let tail: Vec<&str> = err.trim().lines().rev().take(8).collect();
            for l in tail.iter().rev() {
                println!("{l}");
            }
        }
    };
    let run_cargo = |args: &[&str]| {
        let label = format!("cargo xtask {}", args.join(" "));
        let r = std::process::Command::new("cargo")
            .args(["run", "-q", "-p", "xtask", "--"])
            .args(args)
            .current_dir(&root)
            .output()
            .expect("spawn cargo");
        if r.status.success() {
            pass!("{label} exit 0");
        } else {
            failm!("{label} exit {}", r.status.code().unwrap_or(1));
        }
    };

    println!("\nverify-t152-cartographic: glyph atlas (.2)");
    run_make("map-glyphs-verify", &[]);
    println!("\nverify-t152-cartographic: export artifacts (G6 subset)");
    run_make("map-export-validate", &[]);
    println!("\nverify-t152-cartographic: P5_props phase census (.4)");
    run_make(
        "map-verify-phase",
        &[("TERRAIN", "everon"), ("PHASE", "P5_props")],
    ); // E2c-allow
    println!("\nverify-t152-cartographic: locations (.6)");
    run_cargo(&["schema", "locations", "--terrain", "everon"]); // E2c-allow
    println!("\nverify-t152-cartographic: height labels (.7)");
    run_cargo(&["schema", "height-labels", "--terrain", "everon"]); // E2c-allow
    println!("\nverify-t152-cartographic: town labels (.8)");
    run_cargo(&["schema", "town-labels", "--terrain", "everon", "--zoom=-2"]); // E2c-allow
    println!("\nverify-t152-cartographic: road names (.9)");
    run_cargo(&["schema", "road-names", "--terrain", "everon", "--zoom", "0"]); // E2c-allow

    println!("\nverify-t152-cartographic: wasm telemetry (L5)");
    println!(
        "  SKIP  wasm size guard — retired with the React wasm pkg at T-159.29.3 (make wasm-ci owns the crates)"
    );

    println!();
    if failures.get() > 0 {
        eprintln!("verify-t152-cartographic: FAIL ({})", failures.get());
        return Ok(1);
    }
    println!("verify-t152-cartographic: OK");
    Ok(0)
}
