//! T-165.4 — label gates (height/town/road/locations), ported from `scripts/map-assets/`.
//! The height-labels port RESTORES the wasm-era branch natively (declutter G5/G6 + the ASL
//! oracle + G3 completeness) — those gates had retired-skipped when the React wasm pkg died;
//! `map-engine-core::dem` is the same math the wasm wrapped, so the Rust gate runs it directly.
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::root::find_repo_root as repo_root;
use map_engine_core::dem::peaks::{
    HeightLabel, HeightLabelKind, PEAK_MIN_VALUE_M, declutter_height_labels, height_label_min_sep_m,
};
use map_engine_core::dem::png_decode::decode_png_to_meters;
use map_engine_core::dem::sample::{DemManifest, sample_elevation_from_meters_cache};

const PEAK_LABEL_MAX: usize = 48;

fn read_json(p: &PathBuf) -> Result<Value> {
    let raw = fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", p.display()))
}

fn to_height_label(row: &Value) -> HeightLabel {
    HeightLabel {
        x: row["x"].as_f64().unwrap_or(0.0),
        y: row["y"].as_f64().unwrap_or(0.0),
        value_m: row["value_m"].as_i64().unwrap_or(0) as i32,
        kind: if row["kind"] == "contour" {
            HeightLabelKind::Contour
        } else {
            HeightLabelKind::Peak
        },
        name: row["name"].as_str().map(String::from),
    }
}

pub fn height_labels(terrain: &str) -> Result<u8> {
    let root = repo_root()?;
    let base = root.join("packages/map-assets").join(terrain);
    let label_path = base.join("height-labels.json");
    if !label_path.exists() {
        eprintln!("verify-height-labels: missing {}", label_path.display());
        return Ok(1);
    }
    let labels_raw = read_json(&label_path)?;
    let rows = labels_raw.as_array().cloned().unwrap_or_default();
    let mut failures = 0usize;
    let mut pass = |m: String| println!("  PASS  {m}");
    let mut fail_msgs: Vec<String> = Vec::new();
    println!("verify-height-labels ({terrain}):");

    let is_named = |l: &Value| l["name"].as_str().map(|s| !s.is_empty()) == Some(true);
    let named: Vec<&Value> = rows.iter().filter(|l| is_named(l)).collect();
    let unnamed: Vec<&Value> = rows.iter().filter(|l| !is_named(l)).collect();

    // G2 floor.
    let floor_m = f64::from(PEAK_MIN_VALUE_M);
    let below: Vec<&Value> = rows
        .iter()
        .filter(|l| l["value_m"].as_f64().unwrap_or(0.0) < floor_m)
        .collect();
    if below.is_empty() {
        pass(format!(
            "G2 floor: 0 rows < {floor_m} m ({} named + {} DEM)",
            named.len(),
            unnamed.len()
        ));
    } else {
        fail_msgs.push(format!(
            "G2 floor: {} rows < {floor_m} m [{}]",
            below.len(),
            below
                .iter()
                .map(|l| format!("{}={}", l["name"].as_str().unwrap_or("?"), l["value_m"]))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // G4 dedupe: no unnamed DEM peak within 200 m of a named row.
    let collisions = unnamed
        .iter()
        .filter(|u| {
            named.iter().any(|n| {
                let dx = u["x"].as_f64().unwrap_or(0.0) - n["x"].as_f64().unwrap_or(0.0);
                let dy = u["y"].as_f64().unwrap_or(0.0) - n["y"].as_f64().unwrap_or(0.0);
                dx.hypot(dy) < 200.0
            })
        })
        .count();
    if collisions == 0 {
        pass("G4 dedupe: no DEM peak within 200 m of a named row".to_string());
    } else {
        fail_msgs.push(format!(
            "G4 dedupe: {collisions} DEM peaks within 200 m of a named row"
        ));
    }

    // G3 named merge.
    let locations_path = base.join("locations.json");
    let mut loc_peak_hill: Option<Vec<Value>> = None;
    if locations_path.exists() {
        let locs = read_json(&locations_path)?;
        let ph: Vec<Value> = locs
            .as_array()
            .into_iter()
            .flatten()
            .filter(|l| l["kind"] == "peak" || l["kind"] == "hill")
            .cloned()
            .collect();
        let valid: std::collections::HashSet<&str> =
            ph.iter().filter_map(|l| l["name"].as_str()).collect();
        let orphans: Vec<&str> = named
            .iter()
            .filter_map(|l| l["name"].as_str())
            .filter(|n| !valid.contains(n))
            .collect();
        if orphans.is_empty() {
            pass(format!(
                "G3 named merge: {} named rows all trace to locations.json peak/hill",
                named.len()
            ));
        } else {
            fail_msgs.push(format!(
                "G3 named merge: {} named rows not in locations.json [{}]",
                orphans.len(),
                orphans
                    .iter()
                    .take(6)
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        loc_peak_hill = Some(ph);
    } else {
        println!("  SKIP  G3 named merge — no locations.json");
    }

    // ── Native branch (RESTORED at T-165.4 — was wasm-pkg-gated): declutter + ASL oracle. ──
    let manifest_path = base.join("manifest.json");
    let dem_rel = read_json(&manifest_path)
        .ok()
        .and_then(|m| m["dem"]["path"].as_str().map(String::from));
    let dem_path = dem_rel.as_ref().map(|p| base.join(p));
    match (
        manifest_path.exists(),
        dem_path.as_ref().filter(|p| p.exists()),
    ) {
        (true, Some(dem_file)) => {
            let manifest = read_json(&manifest_path)?;
            let dem = &manifest["dem"];
            let min_m = dem["heightRangeMinM"].as_f64().unwrap_or(0.0);
            let max_m = dem["heightRangeMaxM"].as_f64().unwrap_or(0.0);
            let labels: Vec<HeightLabel> = rows.iter().map(to_height_label).collect();

            // G5/G6 declutter @ z=0 via the core math (the same fns the wasm wrapped).
            let drawn = declutter_height_labels(&labels, 0.0);
            let sep = height_label_min_sep_m(0.0);
            let mut declutter_ok = drawn.len() <= PEAK_LABEL_MAX;
            'outer: for i in 0..drawn.len() {
                for j in (i + 1)..drawn.len() {
                    let d = (drawn[i].x - drawn[j].x).hypot(drawn[i].y - drawn[j].y);
                    if d < sep {
                        declutter_ok = false;
                        fail_msgs.push(format!("G4 declutter: pair ({i},{j}) dist={d:.1} < {sep}"));
                        break 'outer;
                    }
                }
            }
            let max_v = drawn.iter().map(|l| l.value_m).max().unwrap_or(0);
            if declutter_ok {
                pass(format!(
                    "G5 declutter @ z=0 (sep {sep:.0} m): {} ≤ {PEAK_LABEL_MAX} drawn",
                    drawn.len()
                ));
            } else {
                fail_msgs.push(format!("G5 cap: count {} > {PEAK_LABEL_MAX}", drawn.len()));
            }
            if max_v >= 350 {
                pass(format!("G6 max(value_m)={max_v} ≥ 350"));
            } else {
                fail_msgs.push(format!("G6 coverage: max(value_m)={max_v} < 350"));
            }

            // ASL ±0.5 m + sample > 0, all rows.
            let bytes = fs::read(dem_file)?;
            let decoded = decode_png_to_meters(&bytes, min_m, max_m)
                .map_err(|e| anyhow::anyhow!("dem decode: {e}"))?;
            let dm = DemManifest {
                min_x: 0.0,
                min_y: 0.0,
                max_x: manifest["worldBounds"][2].as_f64().unwrap_or(0.0),
                max_y: manifest["worldBounds"][3].as_f64().unwrap_or(0.0),
                width_px: decoded.width as usize,
                height_px: decoded.height as usize,
                flip_x: dem["axisFlip"]["x"].as_bool().unwrap_or(false),
                flip_z: dem["axisFlip"]["z"].as_bool().unwrap_or(false),
                height_min_m: min_m,
                height_max_m: max_m,
            };
            let mut asl_errs = Vec::new();
            for (i, l) in labels.iter().enumerate() {
                match sample_elevation_from_meters_cache(
                    l.x,
                    l.y,
                    &dm,
                    &decoded.meters,
                    dm.width_px,
                    dm.height_px,
                ) {
                    Some(e) if e > 0.0 => {
                        if (e - f64::from(l.value_m)).abs() > 0.5 {
                            asl_errs.push(format!(
                                "row {i}: value_m {} vs DEM {e:.2} (>±0.5 m)",
                                l.value_m
                            ));
                        }
                    }
                    other => asl_errs.push(format!("row {i}: DEM sample {other:?} not > 0")),
                }
            }
            if asl_errs.is_empty() {
                pass("ASL ±0.5 m + sample > 0 (core oracle, all rows incl. named)".to_string());
            } else {
                for e in asl_errs.iter().take(5) {
                    fail_msgs.push(e.clone());
                }
                if asl_errs.len() > 5 {
                    fail_msgs.push(format!("… +{} more ASL errors", asl_errs.len() - 5));
                }
            }

            // G3 completeness.
            if let Some(ph) = &loc_peak_hill {
                let mut expected = std::collections::HashSet::new();
                for l in ph {
                    let (x, y) = (
                        l["x"].as_f64().unwrap_or(0.0),
                        l["y"].as_f64().unwrap_or(0.0),
                    );
                    if let Some(e) = sample_elevation_from_meters_cache(
                        x,
                        y,
                        &dm,
                        &decoded.meters,
                        dm.width_px,
                        dm.height_px,
                    ) {
                        if e.is_finite() && e >= floor_m {
                            if let Some(n) = l["name"].as_str() {
                                expected.insert(n.to_string());
                            }
                        }
                    }
                }
                let have: std::collections::HashSet<String> = named
                    .iter()
                    .filter_map(|l| l["name"].as_str().map(String::from))
                    .collect();
                let missing: Vec<&String> =
                    expected.iter().filter(|n| !have.contains(*n)).collect();
                let extra: Vec<&String> = have.iter().filter(|n| !expected.contains(*n)).collect();
                if missing.is_empty() && extra.is_empty() {
                    pass(format!(
                        "G3 completeness: named set == {} locations peak/hill ≥ {floor_m} m",
                        expected.len()
                    ));
                } else {
                    if !missing.is_empty() {
                        fail_msgs.push(format!(
                            "G3 completeness: {} expected names missing [{}]",
                            missing.len(),
                            missing
                                .iter()
                                .take(6)
                                .map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    if !extra.is_empty() {
                        fail_msgs.push(format!(
                            "G3 completeness: {} unexpected named [{}]",
                            extra.len(),
                            extra
                                .iter()
                                .take(6)
                                .map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                }
            }
            println!(
                "  NOTE  contour index labels: T-152.16 FRESH operator waiver (see .ai/artifacts/t152_16_verify_log.md)"
            );
        }
        _ => println!("  SKIP  ASL oracle — DEM absent (run git lfs pull)"),
    }

    for m in &fail_msgs {
        failures += 1;
        println!("  FAIL  {m}");
    }
    if failures > 0 {
        eprintln!("\nverify-height-labels: FAIL ({failures})");
        Ok(1)
    } else {
        println!("\nverify-height-labels: OK");
        Ok(0)
    }
}
