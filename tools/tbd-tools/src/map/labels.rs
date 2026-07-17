//! T-165.9 — label exporters: `export-locations.mjs` + `lib/locations-export.mjs`
//! (locations.json from staged raw JSONL) and `export-height-labels.mjs` (height-labels.json
//! — NATIVELY RESTORED: the Node exporter depended on the React-era wasm pkg deleted at
//! T-159.29.3; this port runs the same math on `map_engine_core::dem` directly, exactly like
//! the T-165.4 height-labels gate restoration).

use std::path::PathBuf;

use anyhow::{Context, Result};
use map_engine_core::dem::peaks::{
    HeightLabel, HeightLabelKind, PEAK_MIN_VALUE_M, declutter_height_labels, find_peaks,
};
use map_engine_core::dem::png_decode::decode_png_to_meters;
use map_engine_core::dem::sample::{DemManifest, sample_elevation_from_meters_cache};
use serde_json::{Map, Value, json};

use crate::serve::repo_root;
use crate::world::jsval::{js_math_round, js_num};

/* ─────────────────────────── locations export ─────────────────────────── */

const SUBFEATURE_WORDS: [&str; 5] = ["sawmill", "sawmil", "farm", "quarry", "mine"];
const LOCALITY_IMPORTANCE: f64 = 0.4;
const N_MIN: usize = 10;
const REQUIRED_EVERON_TOWNS: [&str; 7] = [
    "Morton",
    "Gorey",
    "Raccoon Rock",
    "Saint Philippe",
    "Levie",
    "Montignac",
    "Kermovan",
];

fn importance_by_name(name: &str) -> Option<f64> {
    Some(match name {
        "Montignac" => 0.85,
        "Saint Philippe" => 0.78,
        "Levie" => 0.74,
        "Chotain" => 0.72,
        "Morton" => 0.7,
        "Gorey" => 0.62,
        "Kermovan" => 0.58,
        "Raccoon Rock" => 0.52,
        "Highstone" => 0.48,
        _ => return None,
    })
}

fn display_override(base: &str) -> Option<&'static str> {
    Some(match base {
        "EntreDeux" => "Entre Deux",
        "Le_Moule" => "Le Moule",
        "Villeneuf" => "Villeneuve",
        "StPhilippe_StPhilippe_01" => "Saint Philippe",
        "Airport" => "Airport",
        _ => return None,
    })
}

/// Word-boundary sub-feature test (the JS `\b(sawmill|sawmil|farm|quarry|mine)\b/i`).
fn is_subfeature(name: &str) -> bool {
    let lower = name.to_lowercase();
    for w in SUBFEATURE_WORDS {
        let mut start = 0;
        while let Some(pos) = lower[start..].find(w) {
            let a = start + pos;
            let b = a + w.len();
            let before_ok = a == 0 || !lower.as_bytes()[a - 1].is_ascii_alphanumeric();
            let after_ok = b >= lower.len() || !lower.as_bytes()[b].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
            start = a + 1;
        }
    }
    false
}

fn slug(terrain_id: &str, name: &str) -> String {
    let mut s = String::new();
    let mut dash = false;
    for c in name.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
            dash = false;
        } else if !dash && !s.is_empty() {
            s.push('-');
            dash = true;
        }
    }
    let trimmed = s.trim_matches('-');
    format!("{terrain_id}-{trimmed}")
}

fn round3(v: f64) -> f64 {
    js_math_round(v * 1000.0) / 1000.0
}

fn default_importance(name: &str) -> f64 {
    importance_by_name(name).unwrap_or(0.55)
}

fn reject_name(name: &str) -> bool {
    name.len() < 2 || name.to_lowercase().contains("location composition")
}

fn cfgworld_supplement() -> Vec<Value> {
    vec![
        json!({ "id": "everon-gorey", "name": "Gorey", "x": 4844.906, "y": 8088.995, "kind": "village" }),
        json!({ "id": "everon-highstone", "name": "Highstone", "x": 4950, "y": 8550, "kind": "peak" }),
        json!({ "id": "everon-raccoon-rock", "name": "Raccoon Rock", "x": 1280, "y": 6400, "kind": "village" }),
        json!({ "id": "everon-kermovan", "name": "Kermovan", "x": 6359.376, "y": 9668.684, "kind": "village" }),
    ]
}

/// exportLocationsFromJsonl port (Path B: World/Locations prefabs + CfgWorlds supplement).
pub fn export_locations_from_jsonl(
    jsonl: &std::path::Path,
    terrain_id: &str,
) -> Result<Vec<Value>> {
    let text = std::fs::read_to_string(jsonl).with_context(|| jsonl.display().to_string())?;
    // Insertion-ordered map (JS Map semantics — set on existing key keeps position).
    let mut order: Vec<String> = Vec::new();
    let mut by_id: Map<String, Value> = Map::new();
    let put = |order: &mut Vec<String>,
               by_id: &mut Map<String, Value>,
               id: String,
               v: Value,
               overwrite: bool| {
        if by_id.contains_key(&id) {
            if overwrite {
                by_id.insert(id, v);
            }
        } else {
            order.push(id.clone());
            by_id.insert(id, v);
        }
    };
    for line in text.trim().lines() {
        if line.is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let rn = row["resourceName"].as_str().unwrap_or("");
        if !rn.contains("World/Locations/") {
            continue;
        }
        // basename: Locations/Eden/(sub/)?<base>.et
        let base = rn
            .split("Locations/Eden/")
            .nth(1)
            .and_then(|rest| rest.strip_suffix(".et"))
            .map(|rest| rest.rsplit('/').next().unwrap_or(rest).to_string());
        let Some(base) = base else { continue };

        let direct_town = {
            // /Prefabs/World/Locations/Eden/<file>.et — no subdirectory
            rn.contains("Prefabs/World/Locations/Eden/")
                && rn
                    .split("Prefabs/World/Locations/Eden/")
                    .nth(1)
                    .is_some_and(|rest| {
                        !rest.trim_end_matches(".et").contains('/') && rest.ends_with(".et")
                    })
        };
        if direct_town {
            let name =
                display_override(&base).map_or_else(|| base.replace('_', " "), str::to_string);
            if reject_name(&name) {
                continue;
            }
            let id = slug(terrain_id, &name);
            let base_kind = if base == "Airport" { "airport" } else { "town" };
            let sub = base_kind == "town" && (is_subfeature(&base) || is_subfeature(&name));
            let row_v = json!({
                "id": id,
                "name": name,
                "x": js_num(round3(row["x"].as_f64().unwrap_or(0.0))),
                "y": js_num(round3(row["z"].as_f64().unwrap_or(0.0))),
                "importance": js_num(if sub { LOCALITY_IMPORTANCE } else { default_importance(&name) }),
                "kind": if sub { "locality" } else { base_kind },
            });
            put(&mut order, &mut by_id, id.clone(), row_v, true);
            continue;
        }
        if rn.contains("StPhilippe_StPhilippe_01.et") {
            let name = "Saint Philippe";
            let id = slug(terrain_id, name);
            let row_v = json!({
                "id": id, "name": name,
                "x": js_num(round3(row["x"].as_f64().unwrap_or(0.0))),
                "y": js_num(round3(row["z"].as_f64().unwrap_or(0.0))),
                "importance": js_num(default_importance(name)), "kind": "town",
            });
            put(&mut order, &mut by_id, id.clone(), row_v, false);
            continue;
        }
        if !rn.contains("/Natural/") {
            continue;
        }
        let lb = base.to_lowercase();
        if !(lb.contains("hill")
            || lb.contains("mountains")
            || lb.contains("moutains")
            || lb.contains("peak")
            || lb.contains("ridge"))
        {
            continue;
        }
        let name = display_override(&base).map_or_else(|| base.replace('_', " "), str::to_string);
        if reject_name(&name) {
            continue;
        }
        let id = slug(terrain_id, &name.to_lowercase());
        if by_id.contains_key(&id) {
            continue;
        }
        let kind = if lb.contains("hill") { "hill" } else { "peak" };
        let row_v = json!({
            "id": id, "name": name,
            "x": js_num(round3(row["x"].as_f64().unwrap_or(0.0))),
            "y": js_num(round3(row["z"].as_f64().unwrap_or(0.0))),
            "importance": 0.35, "kind": kind,
        });
        put(&mut order, &mut by_id, id.clone(), row_v, false);
    }
    for sup in cfgworld_supplement() {
        let id = sup["id"].as_str().unwrap_or("").to_string();
        if by_id.contains_key(&id) {
            continue;
        }
        let name = sup["name"].as_str().unwrap_or("");
        let row_v = json!({
            "id": id, "name": name,
            "x": js_num(round3(sup["x"].as_f64().unwrap_or(0.0))),
            "y": js_num(round3(sup["y"].as_f64().unwrap_or(0.0))),
            "importance": js_num(default_importance(name)), "kind": sup["kind"],
        });
        put(&mut order, &mut by_id, id.clone(), row_v, false);
    }
    let mut rows: Vec<Value> = order.into_iter().map(|id| by_id[&id].clone()).collect();
    // JS `localeCompare` collation is case-insensitive at the primary level (byte order
    // would file "Peninsula" before "beach"); lowercase-key compare reproduces it for this
    // corpus (names are unique modulo case, so no secondary-level tiebreak is reachable).
    rows.sort_by_key(|r| r["name"].as_str().unwrap_or("").to_lowercase());
    Ok(rows)
}

/// verifyLocationsGates port (G3–G7).
pub fn verify_locations_gates(locs: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    if locs.len() < N_MIN {
        errors.push(format!("G3: count {} < N_MIN {N_MIN}", locs.len()));
    }
    let norm = |s: &str| {
        s.to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let names: Vec<String> = locs
        .iter()
        .map(|l| norm(l["name"].as_str().unwrap_or("")))
        .collect();
    for town in REQUIRED_EVERON_TOWNS {
        let k = norm(town);
        let prefix: String = k.chars().take(6).collect();
        if !names.iter().any(|n| n == &k || n.contains(&prefix)) {
            errors.push(format!("G4: missing required town \"{town}\""));
        }
    }
    for loc in locs {
        let name = loc["name"].as_str().unwrap_or("");
        let id = loc["id"].as_str().unwrap_or("");
        if name.len() < 2 {
            errors.push(format!("G5: name too short id={id}"));
        }
        if !loc["x"].is_number() || !loc["y"].is_number() {
            errors.push(format!("G5: non-finite coords id={id}"));
        }
        if name.to_lowercase().contains("location composition") {
            errors.push(format!("G6: placeholder name id={id}"));
        }
        if loc["kind"] == "town" && is_subfeature(name) {
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

pub fn export_locations(terrain: &str, src: Option<PathBuf>, dry_run: bool) -> Result<u8> {
    let root = repo_root();
    let default_src = root
        .join("packages/map-assets")
        .join(terrain)
        .join("staging/export/raw-entities.jsonl");
    let src = src.unwrap_or(default_src);
    let out_path = root
        .join("packages/map-assets")
        .join(terrain)
        .join("locations.json");
    if !src.exists() {
        eprintln!("export-locations: source not found: {}", src.display());
        eprintln!(
            "  Run TBD_TerrainWorldExportPlugin (full) + `world copy-export-profile --full` first."
        );
        eprintln!("  Or pass --src to a raw-entities.jsonl with World/Locations rows.");
        return Ok(1);
    }
    let locs = export_locations_from_jsonl(&src, terrain)?;
    let gate_errors = verify_locations_gates(&locs);
    if !gate_errors.is_empty() {
        for e in &gate_errors {
            eprintln!("  FAIL  {e}");
        }
        return Ok(1);
    }
    println!(
        "export-locations: {} rows for {terrain} (source: {})",
        locs.len(),
        src.display()
    );
    if dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&locs.iter().take(5).collect::<Vec<_>>())?
        );
        return Ok(0);
    }
    std::fs::create_dir_all(out_path.parent().unwrap())?;
    std::fs::write(
        &out_path,
        serde_json::to_string_pretty(&Value::Array(locs))? + "\n",
    )?;
    println!("  wrote {}", out_path.display());
    Ok(0)
}

/* ─────────────────────────── height-labels export (native restore) ─────────────────────────── */

pub fn export_height_labels(terrain: &str) -> Result<u8> {
    let root = repo_root();
    let terrain_dir = root.join("packages/map-assets").join(terrain);
    let manifest_path = terrain_dir.join("manifest.json");
    let dem_path = terrain_dir.join("dem/everon-dem-16bit.png");
    let locations_path = terrain_dir.join("locations.json");
    let out_path = terrain_dir.join("height-labels.json");
    if !manifest_path.exists() || !dem_path.exists() {
        eprintln!("export-height-labels: missing manifest or DEM — run git lfs pull first");
        return Ok(1);
    }
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    let dem = &manifest["dem"];
    let m = DemManifest {
        min_x: 0.0,
        min_y: 0.0,
        max_x: manifest["worldBounds"][2].as_f64().unwrap_or(0.0),
        max_y: manifest["worldBounds"][3].as_f64().unwrap_or(0.0),
        width_px: dem["widthPx"].as_u64().unwrap_or(0) as usize,
        height_px: dem["heightPx"].as_u64().unwrap_or(0) as usize,
        flip_x: dem["axisFlip"]["x"].as_bool().unwrap_or(false),
        flip_z: dem["axisFlip"]["z"].as_bool().unwrap_or(false),
        height_min_m: dem["heightRangeMinM"].as_f64().unwrap_or(0.0),
        height_max_m: dem["heightRangeMaxM"].as_f64().unwrap_or(0.0),
    };
    let decoded = decode_png_to_meters(&std::fs::read(&dem_path)?, m.height_min_m, m.height_max_m)
        .map_err(|e| anyhow::anyhow!("dem decode: {e:?}"))?;
    let (width, height) = (decoded.width as usize, decoded.height as usize);
    let peaks = find_peaks(&decoded.meters, width, height, &m);

    const DEDUPE_RADIUS_M: f64 = 200.0;
    let floor_m = PEAK_MIN_VALUE_M;
    let mut named: Vec<HeightLabel> = Vec::new();
    let mut named_dropped: Vec<(String, i32)> = Vec::new();
    if locations_path.exists() {
        let locations: Value = serde_json::from_str(&std::fs::read_to_string(&locations_path)?)?;
        for l in locations.as_array().cloned().unwrap_or_default() {
            let kind = l["kind"].as_str().unwrap_or("");
            if kind != "peak" && kind != "hill" {
                continue;
            }
            let (x, y) = (
                l["x"].as_f64().unwrap_or(0.0),
                l["y"].as_f64().unwrap_or(0.0),
            );
            let name = l["name"].as_str().unwrap_or("").to_string();
            let Some(elev) =
                sample_elevation_from_meters_cache(x, y, &m, &decoded.meters, width, height)
            else {
                eprintln!("export-height-labels: skip named \"{name}\" — no DEM sample");
                continue;
            };
            if !elev.is_finite() || elev <= 0.0 {
                eprintln!(
                    "export-height-labels: skip named \"{name}\" — no DEM sample (elev={elev})"
                );
                continue;
            }
            let value_m = js_math_round(elev) as i32;
            if value_m >= floor_m {
                named.push(HeightLabel {
                    x,
                    y,
                    value_m,
                    kind: HeightLabelKind::Peak,
                    name: Some(name),
                });
            } else {
                named_dropped.push((name, value_m));
            }
        }
    } else {
        eprintln!(
            "export-height-labels: no locations.json at {} — named merge skipped",
            locations_path.display()
        );
    }
    let dem_deduped: Vec<&HeightLabel> = peaks
        .iter()
        .filter(|p| {
            !named
                .iter()
                .any(|nl| (p.x - nl.x).hypot(p.y - nl.y) < DEDUPE_RADIUS_M)
        })
        .collect();
    let out: Vec<HeightLabel> = named
        .iter()
        .cloned()
        .chain(dem_deduped.iter().map(|p| (*p).clone()))
        .collect();
    let drawn = declutter_height_labels(&out, 0.0);

    let label_json = |l: &HeightLabel| -> Value {
        let mut o = Map::new();
        o.insert("x".into(), js_num(l.x));
        o.insert("y".into(), js_num(l.y));
        o.insert("value_m".into(), json!(l.value_m));
        o.insert("kind".into(), json!("peak"));
        if let Some(n) = &l.name {
            o.insert("name".into(), json!(n));
        }
        Value::Object(o)
    };
    std::fs::write(
        &out_path,
        serde_json::to_string_pretty(&out.iter().map(label_json).collect::<Vec<_>>())? + "\n",
    )?;

    let min_value = out.iter().map(|l| l.value_m).min();
    let named_frac = if out.is_empty() {
        0
    } else {
        (named.len() * 100) / out.len()
    };
    println!(
        "export-height-labels: {} labels ({} named + {} DEM = {named_frac}% named; {} @ z=0), min={} m → {}",
        out.len(),
        named.len(),
        dem_deduped.len(),
        drawn.len(),
        min_value.map_or("-".to_string(), |v| v.to_string()),
        out_path.display()
    );
    if !named_dropped.is_empty() {
        named_dropped.sort_by_key(|(_, v)| *v);
        eprintln!(
            "export-height-labels: dropped {} named row(s) < {floor_m} m floor (coastal mis-tags; kind fixes T-152.17/.19): {}",
            named_dropped.len(),
            named_dropped
                .iter()
                .map(|(n, v)| format!("{n}={v}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(0)
}
