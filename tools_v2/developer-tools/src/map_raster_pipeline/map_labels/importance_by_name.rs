use super::*;

pub(super) fn importance_by_name(name: &str) -> Option<f64> {
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

pub(super) fn display_override(base: &str) -> Option<&'static str> {
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
pub(super) fn is_subfeature(name: &str) -> bool {
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

pub(super) fn slug(terrain_id: &str, name: &str) -> String {
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

pub(super) fn round3(v: f64) -> f64 {
    js_math_round(v * 1000.0) / 1000.0
}

pub(super) fn default_importance(name: &str) -> f64 {
    importance_by_name(name).unwrap_or(0.55)
}

pub(super) fn reject_name(name: &str) -> bool {
    name.len() < 2 || name.to_lowercase().contains("location composition")
}

pub(super) fn cfgworld_supplement() -> Vec<Value> {
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
