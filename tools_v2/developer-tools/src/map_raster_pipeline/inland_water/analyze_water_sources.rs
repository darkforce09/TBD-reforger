use super::*;

use crate::repository_layout::{
    INLAND_WATER_ARTIFACTS_DIR, inland_water_artifacts_dir, terrain_dir, terrain_manifest_path,
};

#[allow(clippy::too_many_lines)]
pub fn analyze_water_sources() -> Result<u8> {
    let root = repo_root();
    let sap = sap_dir();
    let ortho_path = sap.join("everon-sap-ortho.png");
    let dem_path = terrain_dir(&root, "everon").join("dem/everon-dem-16bit.png"); // E2c-allow
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(
        terrain_manifest_path(&root, "everon"), // E2c-allow
    )?)?;
    let artifacts = inland_water_artifacts_dir(&root);
    let out_json = artifacts.join("source_spike.json");
    let prev_spike = artifacts.join("refine_spike.json");
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
        let blurred = image_operations::box_blur_f32(&elev64, d, d, VALLEY_BLUR_R);
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
    let full = image_operations::load_png_rgb(&ortho_path)?;
    let ortho = image_operations::resize_rgb(&full, d, d);
    drop(full);

    let mut sat = vec![0f32; d * d];
    let mut lum = vec![0f32; d * d];
    for i in 0..d * d {
        let (s, l) = image_operations::hsl_sat_lum(
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

    let source_components::WaterSourceClassification {
        components: comps,
        grey_ocean_recall,
    } = source_components::classify_source_components(
        source_components::WaterSourcePlanes {
            dimension: d,
            saturation: &sat,
            luminance: &lum,
            slope: &slope,
            elevation_m: &elev_m,
            sea: &sea,
            sea_wide: &sea_wide,
            flat_wide: &flat_wide,
            valley: &valley,
            road_corridor: &road_corridor,
        },
        &log,
    );
    let mut accepted: Vec<&WaterSourceComponent> = comps.iter().filter(|c| c.accepted).collect();
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
        image_operations::save_png_gray(&out_mask, big, big, &mask_big)?;
        // preview: 1600² ortho with the mask tinted blue on top
        let prev = image_operations::resize_rgb(&ortho, 1600, 1600);
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
        image_operations::save_png_rgb(
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
    let comp_json = |c: &WaterSourceComponent| -> Value {
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
        "lane": "inland-water source analysis",
        "parent": format!(
            "the water source and refine spikes: {INLAND_WATER_ARTIFACTS_DIR}/water_source_spike.json and {INLAND_WATER_ARTIFACTS_DIR}/refine_spike.json (both unchanged by this run)"
        ),
        "generatedAt": iso_from_system_time(std::time::SystemTime::now()),
        "decision": {
            "verdict": "G1-B — Eden.topo carries the full ROAD network but NO hydro layer; exact road-corridor SUBTRACTION removes the path/ditch FP class deterministically, enabling a safe wet-channel relaxation that closes the hill-stream/gully FN gap",
            "oceanMask": "A-dem-below-sea-level (UNCHANGED)",
            "inlandMask": "appearance classes (compact + grey-river + wet-channel) computed on the ROAD-SUBTRACTED pixel field; wet-channel relaxed (operator call: carved gully watercourses read as water even when seasonally dry)",
            "automation": "fully offline: pak (.topo + supertextures) + committed DEM → cargo xtask ci map-water-everon; terrain-parameterized (operator one-button requirement)",
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
            "inlandMaskPng": "assets_v2/scratch/everon/sap/water-inland-mask.png (gitignored)",
            "previewPng": "assets_v2/scratch/everon/sap/water-spike-preview.png (gitignored)",
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
