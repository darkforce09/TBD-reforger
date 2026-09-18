use super::*;

use crate::repository_layout::{terrain_dir, terrain_manifest_path};

pub fn analyze_sap_seams(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let ortho_path = sap_dir().join("everon-sap-ortho.png");
    let out_path = repo_root().join(".ai/artifacts/t090_1_2_2_seam_analysis.json");
    eprintln!("analyze-sap-seams: decoding {} …", ortho_path.display());
    let ortho = image_operations::load_png_rgb(&ortho_path)?;
    let res = analyze_seams(&ortho);
    let sum = summarize(&res);
    let stddev = image_operations::stddev_norm_magick(&ortho_path)?;

    let textured_with_apron: Vec<&SeamMetric> = res
        .vertical
        .iter()
        .chain(res.horizontal.iter())
        .filter(|s| {
            s.evaluated && (s.apron_left + s.apron_right) >= 2 && s.band_min_grad < FILL_FLOOR
        })
        .collect();
    let diagnosis = if sum.max_step_delta > STEP_CAP {
        "exposure_mismatch"
    } else if !textured_with_apron.is_empty() {
        "baked_apron_flat_band"
    } else {
        "clean"
    };

    let generated_at = {
        let full = iso_from_system_time(std::time::SystemTime::now());
        format!("{}Z", &full[..19])
    };
    let pick = |arr: &[SeamMetric], k: usize| -> Option<Value> {
        arr.iter().find(|s| s.k == k).map(metric_json)
    };
    let mut spot: Vec<Value> = Vec::new();
    for v in [
        pick(&res.vertical, 1),
        pick(&res.vertical, 49),
        pick(&res.horizontal, 1),
        pick(&res.horizontal, 49),
        sum.worst_evaluated.map(metric_json),
    ]
    .into_iter()
    .flatten()
    {
        spot.push(json!({
            "axis": v["axis"], "k": v["k"], "interiorGrad": v["interiorGrad"],
            "apronLeft": v["apronLeft"], "apronRight": v["apronRight"],
            "bandMinGrad": v["bandMinGrad"], "anchorSafe": v["anchorSafe"],
        }));
    }

    let report = json!({
        "slice": "T-090.1.2.2",
        "terrain": terrain,
        "orthoPath": "assets_v2/scratch/everon/sap/everon-sap-ortho.png",
        "gridPx": 256,
        "bandPx": 8,
        "thresholds": { "FILL_FLOOR": FILL_FLOOR, "REL_FLOOR": REL_FLOOR, "STEP_CAP": crate::world_export_pipeline::json_number_formatting::js_num(STEP_CAP), "DETAIL_MIN": crate::world_export_pipeline::json_number_formatting::js_num(DETAIL_MIN) },
        "diagnosis": diagnosis,
        "globalStddev": stddev,
        "summary": {
            "seamCount": sum.seam_count,
            "evaluatedCount": sum.evaluated_count,
            "worstApron": sum.worst_apron,
            "worstRatio": sum.worst_ratio.map(crate::world_export_pipeline::json_number_formatting::js_num),
            "worstBandMinGrad": sum.worst_evaluated.map(|s| crate::world_export_pipeline::json_number_formatting::js_num(s.band_min_grad)),
            "meanBandMinGradEval": sum.mean_band_min_grad_eval.map(crate::world_export_pipeline::json_number_formatting::js_num),
            "absoluteFloorMet": sum.absolute_floor_met,
            "maxStepDelta": crate::world_export_pipeline::json_number_formatting::js_num(sum.max_step_delta),
            "fillFailureCount": sum.fill_failures.len(),
            "stepFailureCount": sum.step_failures.len(),
            "anchorUnsafeCount": sum.anchor_unsafe.len(),
        },
        "worstEvaluated": sum.worst_evaluated.map(metric_json),
        "anchorSpotCheck": spot,
        "vertical": res.vertical.iter().map(metric_json).collect::<Vec<_>>(),
        "horizontal": res.horizontal.iter().map(metric_json).collect::<Vec<_>>(),
        "controls": res.controls.iter().map(|c| json!({ "axis": c.axis.to_string(), "at": c.at, "bandMinGrad": crate::world_export_pipeline::json_number_formatting::js_num(c.band_min_grad) })).collect::<Vec<_>>(),
        "generatedAt": generated_at,
    });
    std::fs::create_dir_all(out_path.parent().unwrap())?;
    std::fs::write(&out_path, serde_json::to_string_pretty(&report)? + "\n")?;

    println!("\nanalyze-sap-seams — {terrain}");
    println!("  diagnosis:            {diagnosis}");
    println!(
        "  seams:                {} (evaluated/textured: {})",
        sum.seam_count, sum.evaluated_count
    );
    println!(
        "  worst apron (flat run): {}  (primary FILL: apron ≤ 1)",
        sum.worst_apron
    );
    println!(
        "  worst recovery ratio: {}  (REL_FLOOR {REL_FLOOR})",
        sum.worst_ratio.map(fmt2).unwrap_or_default()
    );
    println!(
        "  worst bandMinGrad:    {}  (abs FILL_FLOOR {FILL_FLOOR}: {}/{} met)",
        sum.worst_evaluated
            .map(|s| fmt2(s.band_min_grad))
            .unwrap_or_default(),
        sum.absolute_floor_met,
        sum.evaluated_count
    );
    println!(
        "  mean bandMinGrad:     {}",
        sum.mean_band_min_grad_eval.map(fmt2).unwrap_or_default()
    );
    println!(
        "  max stepΔRGB:         {}  (STEP_CAP {STEP_CAP})",
        fmt2(sum.max_step_delta)
    );
    println!("  global stddev:        {stddev}");
    println!("  fill failures:        {}", sum.fill_failures.len());
    println!("  step failures:        {}", sum.step_failures.len());
    println!("  anchor-unsafe seams:  {}", sum.anchor_unsafe.len());
    println!(
        "  controls (interior bandMinGrad): {}",
        res.controls
            .iter()
            .map(|c| fmt2(c.band_min_grad))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "\n  NIT-1 anchor spot-check (apronL/apronR — anchors at c-5/c+4 must clear the apron):"
    );
    for a in &spot {
        println!(
            "    {} k={}: interior={} apron {}/{} bandMin={} anchorSafe={}",
            a["axis"].as_str().unwrap_or(""),
            a["k"],
            a["interiorGrad"],
            a["apronLeft"],
            a["apronRight"],
            a["bandMinGrad"],
            a["anchorSafe"]
        );
    }
    if !sum.anchor_unsafe.is_empty() {
        println!(
            "\n  WARN: {} seam(s) anchor-unsafe (apron ≥ 5) — widen anchors or reduce HW.",
            sum.anchor_unsafe.len()
        );
    }
    println!("\n  wrote {}", out_path.display());
    Ok(0)
}

pub fn verify_sap_ortho(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let root = repo_root();
    let sap = sap_dir();
    let catalog_path = sap.join("cell-catalog.json");
    let meta_path = sap.join("TBD_SatExport_meta.json");
    let ortho_path = sap.join("everon-sap-ortho.png");
    let manifest_path = terrain_manifest_path(&root, "everon"); // E2c-allow
    let z000 = terrain_dir(&root, "everon").join("tiles/satellite/0/0/0.webp"); // E2c-allow

    const EXPECT_CELLS: u64 = 2500;
    const EXPECT_DIM: usize = 12800;

    let mut errors: Vec<String> = Vec::new();
    let ok = |m: &str| println!("  ok: {m}");

    if !catalog_path.exists() {
        errors.push(format!(
            "missing {} — run 'cargo run -q -p developer-tools --bin world -- sap-catalog' first",
            catalog_path.display()
        ));
    } else {
        let cat: Value = serde_json::from_str(&std::fs::read_to_string(&catalog_path)?)?;
        let cells = cat["cells"].as_array().map(Vec::len).unwrap_or(0) as u64;
        if cat["cellCount"].as_u64() != Some(EXPECT_CELLS) || cells != EXPECT_CELLS {
            errors.push(format!(
                "catalog cellCount {}/{cells} != {EXPECT_CELLS}",
                cat["cellCount"]
            ));
        } else {
            ok(&format!("catalog {EXPECT_CELLS} cells"));
        }
    }

    let mut water_composited = false;
    if !meta_path.exists() {
        errors.push(format!(
            "missing {} — run the stitch first",
            meta_path.display()
        ));
    } else {
        let m: Value = serde_json::from_str(&std::fs::read_to_string(&meta_path)?)?;
        water_composited = m["waterComposite"].is_object();
        if m["source"] != "sap-supertexture-stitch" {
            errors.push(format!(
                "meta.source={}",
                m["source"].as_str().unwrap_or("")
            ));
        }
        if m["cellsDecoded"].as_u64() != Some(EXPECT_CELLS) {
            errors.push(format!(
                "meta.cellsDecoded {} != {EXPECT_CELLS}",
                m["cellsDecoded"]
            ));
        }
        if m["dimensions"] != json!([EXPECT_DIM, EXPECT_DIM]) {
            errors.push(format!("meta.dimensions {}", m["dimensions"]));
        }
        if m["metersPerPixel"] != 1 {
            errors.push(format!("meta.metersPerPixel {} != 1", m["metersPerPixel"]));
        }
        if m["worldBounds"] != json!([0, 0, EXPECT_DIM, EXPECT_DIM]) {
            errors.push(format!("meta.worldBounds {}", m["worldBounds"]));
        }
        if errors.is_empty() {
            ok("meta source/cells/dims/mpp/bounds");
        }
    }

    let mut ortho: Option<Rgb8> = None;
    if !ortho_path.exists() {
        errors.push(format!(
            "missing {} — run the stitch first",
            ortho_path.display()
        ));
    } else {
        let o = image_operations::load_png_rgb(&ortho_path)?;
        if o.w != EXPECT_DIM || o.h != EXPECT_DIM {
            errors.push(format!("ortho {}x{} != {EXPECT_DIM}^2", o.w, o.h));
        } else {
            ok(&format!("ortho {}x{}", o.w, o.h));
        }
        let sd = image_operations::stddev_norm_magick(&ortho_path)?;
        if sd <= MIN_STDDEV {
            errors.push(format!("ortho stddev {sd} <= {MIN_STDDEV} (flat?)"));
        } else {
            ok(&format!("ortho stddev {sd:.4} (> {MIN_STDDEV})"));
        }
        ortho = Some(o);
    }

    // Orientation guard — ortho land-mask vs DEM land-mask rendered north-up (natively).
    if let Some(o) = &ortho
        && manifest_path.exists()
    {
        let manifest: Value = serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
        let dem_path = terrain_dir(&root, "everon") // E2c-allow
            .join(manifest["dem"]["path"].as_str().unwrap_or(""));
        if !dem_path.exists() {
            errors.push(format!(
                "orientation guard: DEM missing at {}",
                dem_path.display()
            ));
        } else {
            let lo = manifest["dem"]["heightRangeMinM"].as_f64().unwrap_or(0.0);
            let hi = manifest["dem"]["heightRangeMaxM"].as_f64().unwrap_or(1.0);
            let sea_frac = (0.0 - lo) / (hi - lo);
            const S: usize = 512;
            let small = image_operations::resize_rgb(o, S, S);
            let mut sap_mask = vec![0u8; S * S]; // 1 = land
            #[allow(clippy::needless_range_loop)]
            for i in 0..S * S {
                let (r, g, b) = (
                    small.data[i * 3],
                    small.data[i * 3 + 1],
                    small.data[i * 3 + 2],
                );
                let land = if water_composited {
                    // land = NOT(blue-water hue window ~[0.50,0.68] with sat floor)
                    let (rf, gf, bf) = (
                        f32::from(r) / 255.0,
                        f32::from(g) / 255.0,
                        f32::from(b) / 255.0,
                    );
                    let max = rf.max(gf).max(bf);
                    let min = rf.min(gf).min(bf);
                    let l = (max + min) / 2.0;
                    let den = 1.0 - (2.0 * l - 1.0).abs();
                    let s = if den < 1e-6 { 0.0 } else { (max - min) / den };
                    let hue = if (max - min).abs() < 1e-6 {
                        0.0
                    } else if (max - rf).abs() < 1e-6 {
                        (((gf - bf) / (max - min)).rem_euclid(6.0)) / 6.0
                    } else if (max - gf).abs() < 1e-6 {
                        ((bf - rf) / (max - min) + 2.0) / 6.0
                    } else {
                        ((rf - gf) / (max - min) + 4.0) / 6.0
                    };
                    !((0.50..=0.68).contains(&hue) && s > 0.05)
                } else {
                    let (s, _) = image_operations::hsl_sat_lum(r, g, b);
                    s > 0.12
                };
                sap_mask[i] = u8::from(land);
            }
            // DEM land = elevation above sea; DEM raster row 0 = south → flip to north-up.
            let (raster, dw, dh) = {
                let dec = png::Decoder::new(std::fs::File::open(&dem_path)?);
                let mut reader = dec.read_info()?;
                let mut data = vec![0u8; reader.output_buffer_size()];
                let info = reader.next_frame(&mut data)?;
                let (w, h) = (info.width as usize, info.height as usize);
                let mut r16 = vec![0u16; w * h];
                for (i, px) in r16.iter_mut().enumerate() {
                    *px = u16::from_be_bytes([data[i * 2], data[i * 2 + 1]]);
                }
                (r16, w, h)
            };
            let sea_u16 = (sea_frac * 65535.0) as f64;
            let mut dem_mask = vec![0u8; S * S];
            for y in 0..S {
                // -flip: north-up display row y ← DEM row (S-1-y) scaled
                let sy = ((S - 1 - y) * dh) / S;
                for x in 0..S {
                    let sx = (x * dw) / S;
                    dem_mask[y * S + x] = u8::from(f64::from(raster[sy * dw + sx]) > sea_u16);
                }
            }
            let ae = sap_mask
                .iter()
                .zip(dem_mask.iter())
                .filter(|(a, b)| a != b)
                .count();
            let ratio = ae as f64 / (S * S) as f64;
            if ratio >= ORIENT_MAX {
                errors.push(format!(
                    "orientation guard: ortho vs north-up DEM AE ratio {ratio:.3} >= {ORIENT_MAX} (basemap upside-down?)"
                ));
            } else {
                ok(&format!(
                    "orientation guard: ortho matches north-up DEM (AE ratio {ratio:.3} < {ORIENT_MAX})"
                ));
            }
        }
    }

    if !z000.exists() || std::fs::metadata(&z000)?.len() == 0 {
        errors.push(format!("missing/empty committed tile {}", z000.display()));
    } else {
        ok(&format!(
            "committed satellite/0/0/0.webp ({} B)",
            std::fs::metadata(&z000)?.len()
        ));
    }

    if !errors.is_empty() {
        eprintln!("\nverify-sap-ortho FAIL ({}):", errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
        return Ok(1);
    }
    println!("\nverify-sap-ortho OK");
    Ok(0)
}

/// `bridgeSeams` — the apron-bridge feather, in place on an interleaved buffer (RGB or RGBA).
pub fn bridge_seams(canvas: &mut [u8], ortho_px: usize, channels: usize) -> Result<usize> {
    let hw = HW;
    let anchor = ANCHOR;
    let span = (2 * hw + 1) as f64;
    let stride = ortho_px * channels;
    if canvas.len() != stride * ortho_px {
        bail!(
            "bridgeSeams: canvas {} B != {} ({ortho_px}²×{channels})",
            canvas.len(),
            stride * ortho_px
        );
    }
    let seams: Vec<usize> = (1..GRID).map(|k| k * CELL_PX).collect();
    for &c in &seams {
        let a_l = c - anchor;
        let a_r = c + hw;
        for y in 0..ortho_px {
            let row = y * stride;
            let o_l = row + a_l * channels;
            let o_r = row + a_r * channels;
            for x in c - hw..=c + hw - 1 {
                let t = (x - a_l) as f64 / span;
                let o = row + x * channels;
                for k in 0..3 {
                    let l = f64::from(canvas[o_l + k]);
                    let r = f64::from(canvas[o_r + k]);
                    canvas[o + k] =
                        crate::world_export_pipeline::json_number_formatting::js_math_round(
                            l + (r - l) * t,
                        ) as u8;
                }
            }
        }
    }
    for &c in &seams {
        let a_t = c - anchor;
        let a_b = c + hw;
        for x in 0..ortho_px {
            let col = x * channels;
            let o_t = a_t * stride + col;
            let o_b = a_b * stride + col;
            for y in c - hw..=c + hw - 1 {
                let t = (y - a_t) as f64 / span;
                let o = y * stride + col;
                for k in 0..3 {
                    let tt = f64::from(canvas[o_t + k]);
                    let bb = f64::from(canvas[o_b + k]);
                    canvas[o + k] =
                        crate::world_export_pipeline::json_number_formatting::js_math_round(
                            tt + (bb - tt) * t,
                        ) as u8;
                }
            }
        }
    }
    Ok(seams.len())
}
