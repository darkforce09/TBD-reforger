use super::*;

use crate::repository_layout::terrain_dir;

pub fn export_height_labels(terrain: &str) -> Result<u8> {
    let root = repo_root();
    let terrain_dir = terrain_dir(&root, terrain);
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
    // T-537: refuse writing [] over committed height-map_labels.json.
    super::super::refuse_empty_write(
        "export-height-labels",
        out.is_empty(),
        "zero labels — refusing empty overwrite of height-labels.json",
    )?;
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
