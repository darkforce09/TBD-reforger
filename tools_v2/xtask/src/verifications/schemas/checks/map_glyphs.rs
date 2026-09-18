use super::*;
use developer_tools::repository_layout::{glyph_assets_dir, terrain_dir};

/// Glyph coverage gate (port of `verify-map-glyphs-manifest.mjs`) — golden + committed-catalog
/// iconKey coverage, SVG existence/viewBox, sane render fields, and the built-atlas rect/RIFF
/// checks when present.
pub fn map_glyphs() -> Result<u8> {
    use std::io::Read as _;
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let glyph_dir = glyph_assets_dir(&root);
    let manifest = read_json(&glyph_dir.join("manifest.json"))?;
    let glyphs = manifest["glyphs"].as_object().cloned().unwrap_or_default();
    let prefabs = read_json(&sroot.join("golden/map-objects/map-object-prefabs-sample.json"))?;

    let mut errors: Vec<String> = Vec::new();

    // 1. Golden coverage.
    for p in prefabs.as_array().into_iter().flatten() {
        if let Some(key) = p["render"]["iconKey"].as_str() {
            if !glyphs.contains_key(key) {
                errors.push(format!(
                    "prefab {}: render.iconKey '{key}' missing from glyph manifest",
                    p["prefabId"]
                ));
            }
        }
    }

    // 1b. Committed terrain catalogs.
    let catalog = terrain_dir(&root, "everon").join("objects/prefabs.json.gz");
    if catalog.exists() {
        let bytes = fs::read(&catalog)?;
        let mut inflated = Vec::new();
        let parsed: Result<Value> = (|| {
            flate2::read::GzDecoder::new(bytes.as_slice()).read_to_end(&mut inflated)?;
            Ok(serde_json::from_slice(&inflated)?)
        })();
        match parsed {
            Ok(doc) => {
                let mut missing: BTreeMap<String, usize> = BTreeMap::new();
                for p in doc["prefabs"].as_array().into_iter().flatten() {
                    if let Some(key) = p["render"]["iconKey"].as_str() {
                        if !glyphs.contains_key(key) {
                            *missing.entry(key.to_string()).or_insert(0) += 1;
                        }
                    }
                }
                for (key, n) in missing {
                    errors.push(format!(
                        "catalog everon: render.iconKey '{key}' ({n} prefabs) missing from glyph manifest"
                    ));
                }
            }
            Err(e) => errors.push(format!("catalog {}: unreadable ({e})", catalog.display())),
        }
    } else {
        // T-935.13 — do not skip glyph coverage when runtime gz-JSON leaves. We still ship the
        // gz as an emitter input; if it is absent the gate must fail, not go silently vacuous.
        errors.push(format!(
            "catalog {}: missing (glyph coverage would otherwise be skipped)",
            catalog.display()
        ));
    }

    // 2. SVG + render-field sanity.
    for (key, g) in &glyphs {
        let Some(svg_rel) = g["svg"].as_str() else {
            errors.push(format!("glyph '{key}': no svg path"));
            continue;
        };
        let svg_path = glyph_dir.join(svg_rel);
        if !svg_path.exists() {
            errors.push(format!("glyph '{key}': svg file not found ({svg_rel})"));
            continue;
        }
        let svg = fs::read_to_string(&svg_path)?;
        if !svg.contains("viewBox") {
            errors.push(format!("glyph '{key}': {svg_rel} has no viewBox"));
        }
        let has_svg_tag = svg.contains("<svg ") || svg.contains("<svg>") || svg.contains("<svg\n");
        if !has_svg_tag {
            errors.push(format!("glyph '{key}': {svg_rel} is not a valid <svg>"));
        }
        if !g["baseSizePx"].as_f64().map(|v| v > 0.0).unwrap_or(false) {
            errors.push(format!(
                "glyph '{key}': baseSizePx must be > 0 (got {})",
                g["baseSizePx"]
            ));
        }
        let anchor_ok = g["anchor"].as_array().map(|a| {
            a.len() == 2
                && a.iter().all(|v| {
                    v.as_f64()
                        .map(|x| (0.0..=1.0).contains(&x))
                        .unwrap_or(false)
                })
        }) == Some(true);
        if !anchor_ok {
            errors.push(format!(
                "glyph '{key}': anchor must be [x,y] with components in [0,1] (got {})",
                g["anchor"]
            ));
        }
    }

    // 3. Atlas gate (when built).
    let atlas_json = glyph_dir.join(
        manifest["atlas"]["rects"]
            .as_str()
            .unwrap_or("atlas/world-glyphs.json"),
    );
    let atlas_webp = glyph_dir.join(
        manifest["atlas"]["image"]
            .as_str()
            .unwrap_or("atlas/world-glyphs.webp"),
    );
    let atlas_built = atlas_json.exists();
    if atlas_built {
        let atlas = read_json(&atlas_json)?;
        let width = atlas["meta"]["width"].as_i64().unwrap_or(-1);
        let height = atlas["meta"]["height"].as_i64().unwrap_or(-1);
        let is_pow2 = |n: i64| n > 0 && (n & (n - 1)) == 0;
        if !is_pow2(width) || !is_pow2(height) || width > 4096 || height > 4096 {
            errors.push(format!(
                "atlas: dims {width}×{height} not power-of-two ≤ 4096²"
            ));
        }
        for key in glyphs.keys() {
            let r = &atlas["icons"][key];
            if r.is_null() {
                errors.push(format!(
                    "atlas: glyph '{key}' has no rect in world-glyphs.json (rebuild: cargo run -q -p developer-tools --bin map -- build-glyph-atlas)"
                ));
                continue;
            }
            let (x, y, w, h) = (
                r["x"].as_f64().unwrap_or(-1.0),
                r["y"].as_f64().unwrap_or(-1.0),
                r["width"].as_f64().unwrap_or(0.0),
                r["height"].as_f64().unwrap_or(0.0),
            );
            if x < 0.0 || y < 0.0 || x + w > width as f64 || y + h > height as f64 {
                errors.push(format!(
                    "atlas: glyph '{key}' rect exceeds {width}×{height} bounds"
                ));
            }
            let (ax, ay) = (
                r["anchorX"].as_f64().unwrap_or(-1.0),
                r["anchorY"].as_f64().unwrap_or(-1.0),
            );
            if !(ax >= 0.0 && ax <= w && ay >= 0.0 && ay <= h) {
                errors.push(format!("atlas: glyph '{key}' anchor outside its rect"));
            }
        }
        if !atlas_webp.exists() {
            errors.push("atlas: world-glyphs.json present but world-glyphs.webp missing".into());
        } else {
            let head = fs::read(&atlas_webp)?;
            if head.len() < 12 || &head[0..4] != b"RIFF" || &head[8..12] != b"WEBP" {
                errors.push("atlas: world-glyphs.webp is not a RIFF/WEBP file".into());
            }
        }
    }

    if errors.is_empty() {
        let atlas_note = if atlas_built {
            ", atlas rects verified"
        } else {
            ", no atlas built"
        };
        println!(
            "verify-map-glyphs: OK ({} glyphs, golden + everon iconKeys covered{atlas_note})",
            glyphs.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-map-glyphs: FAIL");
        for e in &errors {
            eprintln!("  {e}");
        }
        Ok(1)
    }
}
