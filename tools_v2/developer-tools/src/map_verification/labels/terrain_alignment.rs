use super::*;

pub fn terrain_alignment(root: &Path, terrain: &str, strict: bool) -> Result<u8> {
    use website_map_engine::world::terrain::dem::manifest::DemManifest;
    use website_map_engine::world::terrain::dem::sampling::sample_elevation_meters;
    use website_map_engine::world::terrain::dem::sampling::world_to_pixel;
    const MIN_ANCHORS_STRICT: usize = 10;
    let base = terrain_dir(root, terrain);
    let manifest = read_json(&base.join("manifest.json"))?;

    // Manifest schema.
    let schema = read_json(&definition_path(root, "terrain-manifest.schema.json"))?;
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
                "FAIL  --strict requires assets_v2/terrains/{terrain}/anchors/verification.json"
            );
            return Ok(1);
        }
        example_path
    } else {
        println!("\nverify-terrain-alignment: OK (no anchors file)");
        return Ok(0);
    };

    let anchors_doc = read_json(&anchors_file)?;
    let aschema = read_json(&definition_path(root, "terrain-anchors.schema.json"))?;
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
