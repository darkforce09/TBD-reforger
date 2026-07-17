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
    let pass = |m: String| println!("  PASS  {m}");
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

/* ─────────────────────────── locations (T-152.6 G2–G7) ─────────────────────────── */

pub const REQUIRED_EVERON_TOWNS: [&str; 7] = [
    "Morton",
    "Gorey",
    "Raccoon Rock",
    "Saint Philippe",
    "Levie",
    "Montignac",
    "Kermovan",
];
pub const MAJOR_EVERON_ROADS: [&str; 6] = [
    "Main Highway",
    "North-South Highway",
    "Coastal Road",
    "Airfield Access",
    "Gorey Road",
    "Morton Road",
];
const N_MIN: usize = 10;

fn norm_name(s: &str) -> String {
    s.to_lowercase().split_whitespace().collect()
}

/// `verifyLocationsGates` port (G3 count, G4 required towns w/ 6-char prefix fuzz, G5 row
/// quality, G6 placeholder names, G7 kind hygiene).
fn locations_gate_errors(locs: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    if locs.len() < N_MIN {
        errors.push(format!("G3: count {} < N_MIN {N_MIN}", locs.len()));
    }
    let names: Vec<String> = locs
        .iter()
        .filter_map(|l| l["name"].as_str().map(norm_name))
        .collect();
    let name_set: std::collections::HashSet<&str> = names.iter().map(String::as_str).collect();
    for town in REQUIRED_EVERON_TOWNS {
        let k = norm_name(town);
        let prefix: String = k.chars().take(6).collect();
        let ok = name_set.contains(k.as_str()) || names.iter().any(|n| n.contains(&prefix));
        if !ok {
            errors.push(format!("G4: missing required town \"{town}\""));
        }
    }
    let subfeature = regex::RegexBuilder::new(r"\b(sawmill|sawmil|farm|quarry|mine)\b")
        .case_insensitive(true)
        .build()
        .expect("static regex");
    let placeholder = regex::RegexBuilder::new(r"location composition")
        .case_insensitive(true)
        .build()
        .expect("static regex");
    for loc in locs {
        let id = loc["id"].as_str().unwrap_or("?");
        let name = loc["name"].as_str().unwrap_or("");
        if name.chars().count() < 2 {
            errors.push(format!("G5: name too short id={id}"));
        }
        let (x, y) = (loc["x"].as_f64(), loc["y"].as_f64());
        if !(x.map(f64::is_finite) == Some(true) && y.map(f64::is_finite) == Some(true)) {
            errors.push(format!("G5: non-finite coords id={id}"));
        }
        if placeholder.is_match(name) {
            errors.push(format!("G6: placeholder name id={id}"));
        }
        if loc["kind"] == "town" && subfeature.is_match(name) {
            errors.push(format!(
                "G7: sub-feature tagged \"town\" id={id} (\"{name}\") — expected \"locality\""
            ));
        }
        if loc["kind"] == "locality" && loc["importance"].as_f64().unwrap_or(0.5) > 0.45 {
            errors.push(format!(
                "G7: locality importance {} > 0.45 id={id}",
                loc["importance"]
            ));
        }
    }
    errors
}

pub fn locations(terrain: &str) -> Result<u8> {
    let root = repo_root()?;
    let loc_path = root
        .join("packages/map-assets")
        .join(terrain)
        .join("locations.json");
    if !loc_path.exists() {
        eprintln!("verify-locations: missing {}", loc_path.display());
        return Ok(1);
    }
    let locs_doc = read_json(&loc_path)?;
    let locs = locs_doc.as_array().cloned().unwrap_or_default();
    let mut failures = 0usize;
    println!("verify-locations ({terrain}):");

    let schema = read_json(&root.join("packages/tbd-schema/schema/locations.schema.json"))?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let schema_errs: Vec<String> = validator
        .iter_errors(&locs_doc)
        .map(|e| {
            let p = e.instance_path().to_string();
            format!("        {} {e}", if p.is_empty() { "/".into() } else { p })
        })
        .collect();
    if schema_errs.is_empty() {
        println!("  PASS  G2 schema valid");
    } else {
        failures += 1;
        println!("  FAIL  G2 schema invalid");
        for e in schema_errs {
            println!("{e}");
        }
    }

    let gate_errors = locations_gate_errors(&locs);
    if gate_errors.is_empty() {
        println!("  PASS  G3 count {} ≥ N_MIN {N_MIN}", locs.len());
        println!(
            "  PASS  G4 REQUIRED_EVERON_TOWNS ({}) covered",
            REQUIRED_EVERON_TOWNS.len()
        );
        println!("  PASS  G5 row quality (name length, finite x/y)");
        println!("  PASS  G6 no \"Location composition\" placeholder names");
    } else {
        for e in &gate_errors {
            failures += 1;
            println!("  FAIL  {e}");
        }
    }

    if failures > 0 {
        eprintln!("\nverify-locations: FAIL ({failures})");
        Ok(1)
    } else {
        println!("\nverify-locations: OK");
        Ok(0)
    }
}

/* ─────────── town labels (T-152.8/.17 — native rebuild on core importance_declutter) ─────────── */

pub fn town_labels(terrain: &str, deck_zoom: f64) -> Result<u8> {
    use map_engine_core::world::{
        LocationLabel, declutter_town_labels, town_declutter_invariant_holds, town_label_fade_alpha,
    };
    let root = repo_root()?;
    let loc_path = root
        .join("packages/map-assets")
        .join(terrain)
        .join("locations.json");
    if !loc_path.exists() {
        eprintln!(
            "verify-town-labels: missing {} (run T-152.6)",
            loc_path.display()
        );
        return Ok(1);
    }
    let raw = fs::read_to_string(&loc_path)?;
    let all: Vec<LocationLabel> = serde_json::from_str(&raw).context("locations.json parse")?;
    let mut failures = 0usize;
    let fail = |msgs: &mut Vec<String>, m: String| msgs.push(m);
    let mut fails: Vec<String> = Vec::new();
    println!("verify-town-labels ({terrain} @ z={deck_zoom}):");

    let drawn = declutter_town_labels(&all, deck_zoom);

    // G2 — required towns ⊆ drawn (normalized-name membership).
    let drawn_names: std::collections::HashSet<String> =
        drawn.iter().map(|l| norm_name(&l.name)).collect();
    let missing: Vec<&str> = REQUIRED_EVERON_TOWNS
        .iter()
        .copied()
        .filter(|t| !drawn_names.contains(&norm_name(t)))
        .collect();
    if missing.is_empty() {
        println!(
            "  PASS  G2 REQUIRED_EVERON_TOWNS ({}) ⊆ drawn @ z={deck_zoom}",
            REQUIRED_EVERON_TOWNS.len()
        );
    } else {
        fail(
            &mut fails,
            format!(
                "G2 required towns missing from drawn: [{}]",
                missing.join(", ")
            ),
        );
    }

    // G3 — declutter invariant (the core A3 predicate).
    if town_declutter_invariant_holds(&drawn, &all, deck_zoom) {
        println!("  PASS  G3 declutter invariant (A3 predicate, core oracle)");
    } else {
        fail(&mut fails, "G3 town_declutter_invariant_holds".to_string());
    }

    // G4 — provenance: every drawn (id, name) exists in the source rows.
    let src: std::collections::HashSet<(&str, &str)> = all
        .iter()
        .map(|l| (l.id.as_str(), l.name.as_str()))
        .collect();
    if drawn
        .iter()
        .all(|l| src.contains(&(l.id.as_str(), l.name.as_str())))
    {
        println!("  PASS  G4 name provenance = locations.json[id]");
    } else {
        fail(
            &mut fails,
            "G4 drawn label not present in source locations.json".to_string(),
        );
    }

    // G5 — empty source → 0 drawn.
    if declutter_town_labels(&[], deck_zoom).is_empty() {
        println!("  PASS  G5 empty source → |drawn|=0");
    } else {
        fail(&mut fails, "G5 empty source drew labels".to_string());
    }

    // G1 — kind hygiene (settlement lane only).
    let allowed = ["town", "village", "airport", "locality"];
    let excluded = ["peak", "hill", "natural"];
    let kind_of = |l: &LocationLabel| l.kind.clone().unwrap_or_else(|| "town".into());
    let unknown = drawn
        .iter()
        .filter(|l| !allowed.contains(&kind_of(l).as_str()))
        .count();
    let excl = drawn
        .iter()
        .filter(|l| excluded.contains(&kind_of(l).as_str()))
        .count();
    if unknown == 0 && excl == 0 {
        println!(
            "  PASS  G1 kind hygiene: {} drawn ⊆ {{town,village,airport,locality}}; 0 peak/hill/natural @ z={deck_zoom}",
            drawn.len()
        );
    } else {
        fail(
            &mut fails,
            format!("G1 kind hygiene: {excl} excluded + {unknown} unknown kind drawn"),
        );
    }

    // G4 fade endpoints (α 1.0 → 0.5 → 0.0 over z ∈ [2.0, 3.0]).
    let fa = town_label_fade_alpha;
    let approx = |a: f64, b: f64| (a - b).abs() < 1e-6;
    if approx(fa(2.0), 1.0) && approx(fa(2.5), 0.5) && approx(fa(3.0), 0.0) {
        println!(
            "  PASS  G4 fade α: 2.0→{} 2.5→{} 3.0→{}",
            fa(2.0),
            fa(2.5),
            fa(3.0)
        );
    } else {
        fail(
            &mut fails,
            format!(
                "G4 fade endpoints wrong: α(2.0)={} α(2.5)={} α(3.0)={}",
                fa(2.0),
                fa(2.5),
                fa(3.0)
            ),
        );
    }

    // G4 band edges — nothing drawn above the fade ceiling / below the widened floor.
    let above = declutter_town_labels(&all, 3.1).len();
    let below = declutter_town_labels(&all, -4.6).len();
    if above == 0 && below == 0 {
        println!(
            "  PASS  G4 band edges: |drawn|=0 @ z=3.1 (above ceiling) and z=−4.6 (below floor)"
        );
    } else {
        fail(
            &mut fails,
            format!("G4 band edges: {above} drawn @ z=3.1, {below} drawn @ z=−4.6"),
        );
    }

    println!(
        "  NOTE  GPU pack checks retired with the wasm render surface (Leptos lane gated by the editor smokes)"
    );

    for m in &fails {
        failures += 1;
        println!("  FAIL  {m}");
    }
    if failures > 0 {
        eprintln!("\nverify-town-labels: FAIL ({failures})");
        Ok(1)
    } else {
        println!("\nverify-town-labels: OK");
        Ok(0)
    }
}

/* ─────────── road names (T-152.9 — native rebuild on core road_labels) ─────────── */

pub fn road_names(terrain: &str, deck_zoom: f64) -> Result<u8> {
    use map_engine_core::world::{
        ROAD_NAME_MAX_ON_SCREEN, ROAD_NAME_PERP_TOL_M, declutter_road_labels,
        parse_road_names_json, perpendicular_dist_to_polyline, place_road_labels,
        road_declutter_invariant_holds, road_declutter_min_dist_m,
    };
    let root = repo_root()?;
    let base = root.join("packages/map-assets").join(terrain);
    let names_path = base.join("road-names.json");
    let roads_path = base.join("objects/roads.json.gz");
    for (p, hint) in [(&names_path, ""), (&roads_path, "")] {
        if !p.exists() {
            eprintln!("verify-road-names: missing {}{hint}", p.display());
            return Ok(1);
        }
    }
    let names_raw = fs::read_to_string(&names_path)?;
    let names =
        parse_road_names_json(&names_raw).map_err(|e| anyhow::anyhow!("road-names: {e}"))?;
    let gz = fs::read(&roads_path)?;
    let mut store = map_engine_core::world::WorldStore::new();
    let seg_count = store
        .load_roads_gz(&gz)
        .map_err(|e| anyhow::anyhow!("roads.json.gz: {e}"))?;
    let _ = seg_count;
    let mut failures = 0usize;
    let mut fails: Vec<String> = Vec::new();
    println!("verify-road-names ({terrain} @ z={deck_zoom}):");

    let drawn = declutter_road_labels(
        &place_road_labels(&names, &store.roads, deck_zoom),
        deck_zoom,
    );

    // G3 — major roads ⊆ drawn.
    let drawn_names: std::collections::HashSet<&str> =
        drawn.iter().map(|l| l.name.as_str()).collect();
    let missing: Vec<&str> = MAJOR_EVERON_ROADS
        .iter()
        .copied()
        .filter(|r| !drawn_names.contains(r))
        .collect();
    if missing.is_empty() {
        println!(
            "  PASS  G3 MAJOR_EVERON_ROADS ({}) ⊆ drawn @ z={deck_zoom}",
            MAJOR_EVERON_ROADS.len()
        );
    } else {
        fails.push(format!(
            "G3 major roads missing from drawn: [{}]",
            missing.join(", ")
        ));
    }

    // G4 — name length ≥ 2 on every drawn row.
    if drawn.iter().all(|l| l.name.trim().chars().count() >= 2) {
        println!("  PASS  G4 name.length ≥ 2");
    } else {
        fails.push("G4 drawn label with name shorter than 2".to_string());
    }

    // G5 — placement within perpendicular tolerance of its own segment.
    let by_id: std::collections::HashMap<&str, &map_engine_core::world::RoadSegment> =
        store.roads.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut perp_bad = 0usize;
    for l in &drawn {
        match by_id.get(l.segment_id.as_str()) {
            Some(seg) => {
                let d = perpendicular_dist_to_polyline(&seg.points, l.x, l.y);
                if d > ROAD_NAME_PERP_TOL_M {
                    perp_bad += 1;
                }
            }
            None => perp_bad += 1,
        }
    }
    if perp_bad == 0 {
        println!("  PASS  G5 placement ≤ {ROAD_NAME_PERP_TOL_M} m perpendicular");
    } else {
        fails.push(format!(
            "G5 {perp_bad} drawn labels beyond {ROAD_NAME_PERP_TOL_M} m perpendicular"
        ));
    }

    // G6 — declutter invariant (core oracle) + min-dist restated.
    if road_declutter_invariant_holds(&drawn, deck_zoom) {
        println!(
            "  PASS  G6 declutter dist ≥ {:.0} m (core oracle)",
            road_declutter_min_dist_m(deck_zoom)
        );
    } else {
        fails.push("G6 road_declutter_invariant_holds".to_string());
    }

    // G7 — cap.
    if drawn.len() <= ROAD_NAME_MAX_ON_SCREEN {
        println!(
            "  PASS  G7 |drawn|={} ≤ {ROAD_NAME_MAX_ON_SCREEN}",
            drawn.len()
        );
    } else {
        fails.push(format!(
            "G7 |drawn|={} > {ROAD_NAME_MAX_ON_SCREEN}",
            drawn.len()
        ));
    }

    // Toggle-off oracle — empty names → 0 drawn.
    let empty = parse_road_names_json(r#"{"schemaVersion":"1.0","terrainId":"everon","roads":[]}"#)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if declutter_road_labels(
        &place_road_labels(&empty, &store.roads, deck_zoom),
        deck_zoom,
    )
    .is_empty()
    {
        println!("  PASS  toggle off oracle: empty names → |drawn|=0");
    } else {
        fails.push("toggle-off: empty names drew labels".to_string());
    }

    println!(
        "  NOTE  GPU pack checks retired with the wasm render surface (Leptos lane gated by the editor smokes)"
    );

    for m in &fails {
        failures += 1;
        println!("  FAIL  {m}");
    }
    if failures > 0 {
        eprintln!("\nverify-road-names: FAIL ({failures})");
        Ok(1)
    } else {
        println!("\nverify-road-names: OK");
        Ok(0)
    }
}

/* ─────────── terrain alignment (T-091.0 — DEM vs GetSurfaceY anchors) ─────────── */

/// Decode a 16-bit grayscale PNG into a u16 raster (channel 0). Mirrors the pngjs
/// `{ skipRescale: true }` read + `rasterFromPngjs` channel extraction.
fn decode_u16_gray_png(bytes: &[u8]) -> Result<(Vec<u16>, usize, usize)> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().context("png read_info")?;
    let info = reader.info();
    anyhow::ensure!(
        info.bit_depth == png::BitDepth::Sixteen,
        "DEM must be 16-bit PNG; got depth={:?}",
        info.bit_depth
    );
    anyhow::ensure!(
        matches!(
            info.color_type,
            png::ColorType::Grayscale | png::ColorType::GrayscaleAlpha
        ),
        "DEM must be grayscale; colorType={:?}",
        info.color_type
    );
    let channels = info.color_type.samples();
    let (w, h) = (info.width as usize, info.height as usize);
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf).context("png frame")?;
    let data = &buf[..frame.buffer_size()];
    let mut raster = vec![0u16; w * h];
    for (i, px) in raster.iter_mut().enumerate() {
        let off = i * channels * 2;
        *px = u16::from_be_bytes([data[off], data[off + 1]]);
    }
    Ok((raster, w, h))
}

/// JS `Number.prototype.toFixed(3)` semantics: ties round away from zero on the
/// magnitude (ECMA picks the larger n for |x|), unlike Rust's `{:.3}` half-to-even.
/// Anchor files carry exact dyadic ties (0.0625, -18.3125) where the two differ.
fn js_fixed3(x: f64) -> String {
    let n = (x.abs() * 1000.0).round() as i64;
    let sign = if x.is_sign_negative() && (n != 0 || x < 0.0) {
        "-"
    } else {
        ""
    };
    format!("{sign}{}.{:03}", n / 1000, n % 1000)
}

pub fn terrain_alignment(terrain: &str, strict: bool) -> Result<u8> {
    use map_engine_core::dem::sample::{DemManifest, sample_elevation_meters, world_to_pixel};
    const MIN_ANCHORS_STRICT: usize = 10;
    let root = repo_root()?;
    let base = root.join("packages/map-assets").join(terrain);
    let manifest = read_json(&base.join("manifest.json"))?;

    // Manifest schema.
    let schema = read_json(&root.join("packages/tbd-schema/schema/terrain-manifest.schema.json"))?;
    let v = jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("compile: {e}"))?;
    if v.iter_errors(&manifest).next().is_some() {
        eprintln!("FAIL  Manifest schema");
        return Ok(1);
    }

    let wpx = manifest["dem"]["widthPx"].as_u64().unwrap_or(0) as usize;
    let hpx = manifest["dem"]["heightPx"].as_u64().unwrap_or(0) as usize;
    let stub = wpx == 0 || hpx == 0;
    if stub {
        println!("WARN  Stub DEM (widthPx/heightPx=0) — strict anchor math deferred");
        if strict {
            eprintln!("FAIL  --strict requires exported DEM with widthPx/heightPx > 0");
            return Ok(1);
        }
    }

    let anchors_path = base.join("anchors/verification.json");
    let example_path = base.join("anchors/verification.example.json");
    let anchors_file = if anchors_path.exists() {
        anchors_path
    } else if example_path.exists() {
        println!("WARN  Using verification.example.json (not production anchors)");
        if strict {
            eprintln!(
                "FAIL  --strict requires packages/map-assets/{terrain}/anchors/verification.json"
            );
            return Ok(1);
        }
        example_path
    } else {
        println!("\nverify-terrain-alignment: OK (no anchors file)");
        return Ok(0);
    };

    let anchors_doc = read_json(&anchors_file)?;
    let aschema = read_json(&root.join("packages/tbd-schema/schema/terrain-anchors.schema.json"))?;
    let av = jsonschema::validator_for(&aschema).map_err(|e| anyhow::anyhow!("compile: {e}"))?;
    if av.iter_errors(&anchors_doc).next().is_some() {
        eprintln!("FAIL  Anchors schema");
        return Ok(1);
    }
    println!("PASS  Anchors validate ({})", anchors_file.display());

    let threshold = anchors_doc["thresholdM"].as_f64().unwrap_or(1.0);
    let anchors = anchors_doc["anchors"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if strict && anchors.len() < MIN_ANCHORS_STRICT {
        eprintln!(
            "FAIL  --strict requires ≥{MIN_ANCHORS_STRICT} anchors, got {}",
            anchors.len()
        );
        return Ok(1);
    }
    if stub {
        println!("\nverify-terrain-alignment: OK (stub — schema only)");
        return Ok(0);
    }

    let dem_path = base.join(manifest["dem"]["path"].as_str().unwrap_or_default());
    if !dem_path.exists() {
        eprintln!("FAIL  DEM file missing: {}", dem_path.display());
        return Ok(1);
    }
    let (raster, w, h) = decode_u16_gray_png(&fs::read(&dem_path)?)?;
    if w != wpx || h != hpx {
        eprintln!("FAIL  PNG IHDR {w}×{h} !== manifest {wpx}×{hpx}");
        return Ok(1);
    }
    println!("PASS  DEM PNG {w}×{h} @ {}", dem_path.display());

    let dm = DemManifest {
        min_x: manifest["worldBounds"][0].as_f64().unwrap_or(0.0),
        min_y: manifest["worldBounds"][1].as_f64().unwrap_or(0.0),
        max_x: manifest["worldBounds"][2].as_f64().unwrap_or(0.0),
        max_y: manifest["worldBounds"][3].as_f64().unwrap_or(0.0),
        width_px: w,
        height_px: h,
        flip_x: manifest["dem"]["axisFlip"]["x"].as_bool().unwrap_or(false),
        flip_z: manifest["dem"]["axisFlip"]["z"].as_bool().unwrap_or(false),
        height_min_m: manifest["dem"]["heightRangeMinM"].as_f64().unwrap_or(0.0),
        height_max_m: manifest["dem"]["heightRangeMaxM"].as_f64().unwrap_or(0.0),
    };

    let mut failures = 0usize;
    let mut max_delta = 0f64;
    println!("\nAnchor elevation verify (|demYM - surfaceYM| ≤ thresholdM):");
    println!("id\tx\tz\tsurfaceYM\tdemYM\tdeltaM\tPASS");
    for a in &anchors {
        let id = a["id"].as_str().unwrap_or("?");
        let Some(surface) = a["surfaceYM"].as_f64().filter(|v| v.is_finite()) else {
            eprintln!("FAIL  {id}: surfaceYM missing or non-finite");
            failures += 1;
            continue;
        };
        let (x, z) = (
            a["x"].as_f64().unwrap_or(0.0),
            a["z"].as_f64().unwrap_or(0.0),
        );
        let Some(dem_ym) = sample_elevation_meters(x, z, &dm, &raster, w, h) else {
            eprintln!("FAIL  {id}: Anchor ({x}, {z}) outside DEM raster");
            failures += 1;
            continue;
        };
        let delta = (dem_ym - surface).abs();
        max_delta = max_delta.max(delta);
        let ok = delta <= threshold;
        println!(
            "{id}\t{x}\t{z}\t{}\t{}\t{}\t{}",
            js_fixed3(surface),
            js_fixed3(dem_ym),
            js_fixed3(delta),
            if ok { "PASS" } else { "FAIL" }
        );
        if !ok {
            failures += 1;
        }
    }

    for a in &anchors {
        let id = a["id"].as_str().unwrap_or("?");
        let (x, z) = (
            a["x"].as_f64().unwrap_or(0.0),
            a["z"].as_f64().unwrap_or(0.0),
        );
        if x < 0.0 || x > dm.max_x || z < 0.0 || z > dm.max_y {
            eprintln!("FAIL  {id}: ({x}, {z}) outside worldBounds");
            failures += 1;
        }
        let pc = world_to_pixel(x, z, &dm);
        let (u, vv) = (pc.px / (w as f64 - 1.0), pc.py / (h as f64 - 1.0));
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&vv) {
            eprintln!("FAIL  {id}: normalized (u,v)=({u},{vv}) outside [0,1]");
            failures += 1;
        }
    }

    println!(
        "\nmaxDeltaM={} thresholdM={threshold}",
        js_fixed3(max_delta)
    );
    if failures > 0 {
        eprintln!("\n{failures} failure(s) — slice FAIL");
        Ok(1)
    } else {
        println!("\nverify-terrain-alignment: OK");
        Ok(0)
    }
}
