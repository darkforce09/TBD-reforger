use super::*;

use crate::repository_layout::{CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR, map_scratch_dir};

/// Classify the stitched orthophoto into land-cover masks: classification at CLASS_PX (nearest
/// sample), close-then-open morphology, soft-edge masks + meta JSON.
pub fn build_landcover_masks(terrain: &str) -> Result<LandcoverOut> {
    if terrain != "everon" {
        bail!("build-landcover-mask: no SAP source registered for terrain \"{terrain}\"");
    }
    let root = repo_root();
    let sap = map_scratch_dir(&root, "everon").join("sap/everon-sap-ortho.png"); // E2c-allow
    // Checkout-relative spelling of `sap`, quoted verbatim in the meta JSON's provenance field and
    // in the missing-source error, where an absolute host path would be noise.
    let sap_rel = "assets_v2/scratch/everon/sap/everon-sap-ortho.png"; // E2c-allow
    if !sap.exists() {
        bail!(
            "build-landcover-mask: SAP ortho missing: {sap_rel}\nassets_v2/scratch/ is gitignored — restore it (cargo xtask ci map-water-everon rebuilds the water composite)."
        );
    }
    let out_dir = map_scratch_dir(&root, "everon").join("map"); // E2c-allow
    std::fs::create_dir_all(&out_dir)?;
    let started = std::time::Instant::now();

    let full = image_operations::load_png_rgb(&sap)?;
    let smp = image_operations::sample_rgb(&full, CLASS_PX, CLASS_PX);
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
        let mut p = image_operations::box_blur_f32(plane, CLASS_PX, CLASS_PX, 6);
        for v in &mut p {
            *v = if *v >= 0.35 { 1.0 } else { 0.0 };
        }
        let mut p = image_operations::box_blur_f32(&p, CLASS_PX, CLASS_PX, 6);
        for v in &mut p {
            *v = if *v >= 0.6 { 1.0 } else { 0.0 };
        }
        image_operations::box_blur_f32(&p, CLASS_PX, CLASS_PX, 2)
    };
    let forest = close_open(&forest);
    let bright = close_open(&bright);
    let write_mask = |plane: &[f32], path: &Path| -> Result<()> {
        let data: Vec<u8> = plane
            .iter()
            .map(|&v| {
                crate::world_export_pipeline::json_number_formatting::js_math_round(
                    f64::from(v.clamp(0.0, 1.0)) * 255.0,
                ) as u8
            })
            .collect();
        image_operations::save_png_gray(path, CLASS_PX, CLASS_PX, &data)
    };
    let forest_out = out_dir.join("landcover-forest-mask.png");
    let bright_out = out_dir.join("landcover-bright-mask.png");
    write_mask(&forest, &forest_out)?;
    write_mask(&bright, &bright_out)?;
    let frac = |c: u64| js_num((c as f64 / n as f64 * 10000.0).round() / 10000.0);
    let meta = json!({
        "lane": "cartographic landcover",
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

/// despike (see build-map-cartographic.mjs): duplicate-drop, return-spike drop, width-stub
/// perpendicular-excursion drop.
pub(super) fn despike(verts: &[f32]) -> Vec<(f64, f64)> {
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

pub(super) fn road_style(ty: u8) -> Option<(&'static str, u32)> {
    match ty {
        0 => Some(("#9aa3a2", 20)),
        1 => Some(("#b0452b", 10)),
        2 => Some(("#c8823c", 8)),
        3 => Some(("#ded6bd", 5)),
        5 => Some(("#7a7466", 3)),
        _ => None,
    }
}

pub fn build_map_cartographic(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!(
            "build-map-cartographic: no cartographic source registered for terrain \"{terrain}\".\nExport one first (Workbench → Plugins → TBD → \"Export TBD Satellite\") and add a source row."
        );
        return Ok(1);
    }
    let root = repo_root();
    let tga = map_scratch_dir(&root, "everon").join("spike/TBD_SatExport_everon.tga"); // E2c-allow
    let out = map_scratch_dir(&root, "everon").join("map/everon-map-ortho.png"); // E2c-allow
    let water_mask_path = map_scratch_dir(&root, "everon").join("sap/water-inland-mask.png"); // E2c-allow
    let (world_px, source_px) = (12800usize, 4096usize);
    if !tga.exists() {
        eprintln!(
            "build-map-cartographic: source raster missing: {}\nassets_v2/scratch/ is gitignored — regenerate via the Workbench export.",
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
        let m = image_operations::load_png_rgb(mask_path)?;
        // masks are CLASS_PX² grayscale → resize to source, multiply by tint alpha, Over.
        let m = image_operations::resize_rgb(&m, source_px, source_px);
        for i in 0..source_px * source_px {
            let a = f64::from(m.data[i * 3]) / 255.0 * alpha;
            if a <= 0.0 {
                continue;
            }
            let o = i * 3;
            for (c, col) in color.iter().enumerate() {
                base.data[o + c] =
                    crate::world_export_pipeline::json_number_formatting::js_math_round(
                        f64::from(base.data[o + c]) * (1.0 - a) + col * a,
                    ) as u8;
            }
        }
    }

    // ── Upscale → world extent, inland-water tint ──
    let mut world = image_operations::resize_rgb(&base, world_px, world_px);
    drop(base);
    let has_water = water_mask_path.exists();
    if has_water {
        let wm = image_operations::load_png_rgb(&water_mask_path)?;
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
                world.data[o + c] =
                    crate::world_export_pipeline::json_number_formatting::js_math_round(
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
    image_operations::save_png_rgb(&out, &world)?;

    let meta = json!({
        "lane": "cartographic landcover",
        "source": "workbench-cartographic",
        "terrain": terrain,
        "sourceRaster": "assets_v2/scratch/everon/spike/TBD_SatExport_everon.tga",
        "sourceDimensions": [source_px, source_px],
        "dimensions": [world_px, world_px],
        "worldBounds": [0, 0, world_px, world_px],
        "upscale": format!("{source_px}->{world_px} Lanczos (documented upscale, slice spec §1)"),
        "orientation": "north-up (TGA top origin preserved; no flips on this path)",
        "overlays": {
            "landCover": {
                "source": "build-landcover-mask (SAP appearance heuristic, L1)",
                "thresholds": landcover.meta["thresholds"],
                "fractions": landcover.meta["fractions"],
                "style": { "open": { "color": "#CDC6A3", "alpha": 0.7 }, "forest": { "color": "#37502D", "alpha": 0.8 } },
                "provenance": "SAP ortho read-only; satellite bundle untouched",
            },
            "inlandWater": if has_water {
                json!({ "mask": "assets_v2/scratch/everon/sap/water-inland-mask.png", "color": "#2E5266", "provenance": "inland-water classifier output (read-only reuse)" })
            } else {
                Value::Null
            },
            "roads": {
                "source": "world::topo (.topo vector network)",
                "records": drawn_records,
                "vertices": drawn_verts,
                "style": { "0": { "color": "#9aa3a2", "width": 20 }, "1": { "color": "#b0452b", "width": 10 }, "2": { "color": "#c8823c", "width": 8 }, "3": { "color": "#ded6bd", "width": 5 }, "5": { "color": "#7a7466", "width": 3 } },
            },
        },
        "spikeArtifact": format!("{CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR}/landcover_source_spike.json"),
        "buildSeconds": started.elapsed().as_secs(),
        "generatedAt": iso_from_system_time(std::time::SystemTime::now()),
    });
    std::fs::write(
        out.parent().unwrap().join("map-ortho-meta.json"),
        serde_json::to_string_pretty(&meta)? + "\n",
    )?;
    let out_rel = "assets_v2/scratch/everon/map/everon-map-ortho.png"; // E2c-allow
    println!(
        "build-map-cartographic: OK {out_rel} ({world_px}² north-up, {drawn_records} road records / {drawn_verts} verts, water={has_water}, {}s)",
        meta["buildSeconds"]
    );
    Ok(0)
}
