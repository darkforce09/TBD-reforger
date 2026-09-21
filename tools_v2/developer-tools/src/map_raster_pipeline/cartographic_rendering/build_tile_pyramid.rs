use super::*;

use crate::repository_layout::{map_scratch_dir, terrain_dir, terrain_manifest_path};

/// build-tile-pyramid.sh port: XYZ WebP levels from a full-extent ortho (+full.webp).
#[allow(clippy::too_many_arguments)]
pub fn build_tile_pyramid(
    input: &Path,
    out: &Path,
    min_zoom: u32,
    max_zoom: u32,
    tile: usize,
    quality: f32,
    lossless: bool,
    flip_v: bool,
) -> Result<u8> {
    if !input.exists() {
        eprintln!("input not found: {}", input.display());
        return Ok(1);
    }
    let enc_desc = if lossless {
        "lossless".to_string()
    } else {
        format!("q={quality}")
    };
    let src_dyn = {
        let mut reader = image::ImageReader::open(input)?;
        reader.no_limits();
        reader.decode()?
    };
    let mut norm = Rgb8 {
        w: src_dyn.width() as usize,
        h: src_dyn.height() as usize,
        data: src_dyn.to_rgb8().into_raw(),
    };
    if flip_v {
        let stride = norm.w * 3;
        for y in 0..norm.h / 2 {
            let (top, bottom) = (y * stride, (norm.h - 1 - y) * stride);
            for i in 0..stride {
                norm.data.swap(top + i, bottom + i);
            }
        }
    }
    println!(
        "[pyramid] source {}x{}; tile={tile} enc={enc_desc} zoom {min_zoom}..{max_zoom}",
        norm.w, norm.h
    );
    let _ = std::fs::remove_dir_all(out);
    std::fs::create_dir_all(out)?;

    let mut total = 0u64;
    for z in min_zoom..=max_zoom {
        let n = 1usize << z;
        let side = n * tile;
        let level = image_operations::resize_rgb(&norm, side, side);
        println!("[pyramid] z={z}  {n}x{n} tiles ({side}px)");
        for x in 0..n {
            std::fs::create_dir_all(out.join(format!("{z}/{x}")))?;
        }
        for ty in 0..n {
            for tx in 0..n {
                let cell = image_operations::crop_rgb(&level, tx * tile, ty * tile, tile, tile)?;
                let bytes = if lossless {
                    image_operations::encode_webp_lossless_rgb(&cell)?
                } else {
                    image_operations::encode_webp_lossy_rgb(&cell, quality)
                };
                std::fs::write(out.join(format!("{z}/{tx}/{ty}.webp")), bytes)?;
            }
        }
        total += (n * n) as u64;
    }

    // full.webp (≤4096 px edge).
    let fe = norm.w.min(4096);
    let full = if fe == norm.w {
        norm
    } else {
        image_operations::resize_rgb(&norm, fe, fe)
    };
    let full_bytes = if lossless {
        image_operations::encode_webp_lossless_rgb(&full)?
    } else {
        image_operations::encode_webp_lossy_rgb(&full, quality)
    };
    std::fs::write(out.join("full.webp"), full_bytes)?;
    println!("[pyramid] wrote full.webp ({fe}px)");
    println!("[pyramid] wrote {total} tiles to {}", out.display());
    if !out.join("0/0/0.webp").exists() {
        eprintln!("[pyramid] FAIL: missing {}/0/0/0.webp", out.display());
        return Ok(1);
    }
    println!("[pyramid] OK  0/0/0.webp + full.webp present");
    Ok(0)
}

/// `cargo xtask ci map-water-everon` step 2: drop the one-shot waterComposite block from the SAP meta.
pub fn reset_water_meta(terrain: &str) -> Result<u8> {
    let p = map_scratch_dir(&repo_root(), terrain).join("sap/TBD_SatExport_meta.json");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&p)?)?;
    if let Some(obj) = m.as_object_mut() {
        obj.remove("waterComposite");
    }
    std::fs::write(&p, serde_json::to_string_pretty(&m)? + "\n")?;
    Ok(0)
}

/// `cargo xtask ci map-water-everon` step 5: manifest.tiles.satellite.unified.bytes = bundle size.
pub fn patch_unified_bytes(terrain: &str) -> Result<u8> {
    let root = terrain_dir(&repo_root(), terrain);
    let mp = root.join("manifest.json");
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&mp)?)?;
    let bundle = root.join(
        m["tiles"]["satellite"]["unified"]["path"]
            .as_str()
            .unwrap_or("satellite/everon-sat.tbd-sat"),
    );
    m["tiles"]["satellite"]["unified"]["bytes"] = json!(std::fs::metadata(&bundle)?.len());
    std::fs::write(&mp, serde_json::to_string_pretty(&m)? + "\n")?;
    Ok(0)
}

/// `cargo xtask ci map-cartographic-everon` step 3: tiles.map {source, encoding} patch.
pub fn patch_map_tiles_meta(terrain: &str) -> Result<u8> {
    let mp = terrain_manifest_path(&repo_root(), terrain);
    let mut m: Value = serde_json::from_str(&std::fs::read_to_string(&mp)?)?;
    let map_block = m["tiles"]["map"]
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("manifest tiles.map missing"))?;
    map_block.insert("source".into(), json!("workbench-cartographic"));
    map_block.insert("encoding".into(), json!("webp-lossy"));
    std::fs::write(&mp, serde_json::to_string_pretty(&m)? + "\n")?;
    Ok(0)
}

/// The program-wide cartographic aggregator: committed slice logs plus live sub-verifiers.
pub fn verify_cartographic() -> Result<u8> {
    let root = repo_root();
    let artifacts = root.join(crate::repository_layout::OPERATIONS_LOG_DIR);
    let failures = std::cell::Cell::new(0usize);
    macro_rules! pass {
        ($($a:tt)*) => { println!("  PASS  {}", format!($($a)*)) };
    }
    macro_rules! failm {
        ($($a:tt)*) => {{ failures.set(failures.get() + 1); println!("  FAIL  {}", format!($($a)*)); }};
    }

    println!("verify-cartographic: slice logs (G1 subset)");
    for i in 0..10 {
        let path = artifacts.join(format!("t152_{i}_verify_log.md"));
        let label = format!("slice log {i}");
        if !path.exists() {
            failm!("{label} missing {}", path.display());
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        let auto = match text.find("\n## Manual") {
            Some(idx) => &text[..idx],
            None => &text[..],
        };
        if auto.contains("**FAIL**") {
            failm!("{label} verify log contains **FAIL** in automated section");
            continue;
        }
        let gate_pass_rows = auto
            .lines()
            .filter(|l| l.starts_with("| **G") && l.contains("| **PASS**"))
            .count();
        let gate_fail_rows = auto
            .lines()
            .filter(|l| l.starts_with("| **G") && l.contains("| **FAIL**"))
            .count();
        let verdict_ok = gate_fail_rows == 0
            && (gate_pass_rows > 0
                || text.to_lowercase().contains("all gn pass")
                || text.to_lowercase().contains("all automated gn pass")
                || text.to_lowercase().contains("automated gn all **pass**")
                || text
                    .to_lowercase()
                    .contains(&format!("tag **t-152.{i}** allowed"))
                || (i == 0 && text.contains("**ALL Gn PASS**"))
                || (i == 2 && text.contains("**G7**") && text.contains("**PASS**")));
        if !verdict_ok {
            failm!("{label} verify log missing PASS verdict / ship marker");
            continue;
        }
        pass!(
            "{label} log OK ({})",
            path.strip_prefix(&root).unwrap_or(&path).display()
        );
    }

    let run_world = |args: &[&str]| {
        let label = format!(
            "cargo run -p developer-tools --bin world -- {}",
            args.join(" ")
        );
        let r = std::process::Command::new("cargo")
            .args(["run", "-q", "-p", "developer-tools", "--bin", "world", "--"])
            .args(args)
            .current_dir(&root)
            .output()
            .expect("spawn cargo");
        if r.status.success() {
            pass!("{label} exit 0");
        } else {
            failm!("{label} exit {}", r.status.code().unwrap_or(1));
            let err = String::from_utf8_lossy(&r.stderr);
            let tail: Vec<&str> = err.trim().lines().rev().take(8).collect();
            for l in tail.iter().rev() {
                println!("{l}");
            }
        }
    };
    let run_cargo = |args: &[&str]| {
        let label = format!("cargo xtask {}", args.join(" "));
        let r = std::process::Command::new("cargo")
            .args(["run", "-q", "-p", "xtask", "--"])
            .args(args)
            .current_dir(&root)
            .output()
            .expect("spawn cargo");
        if r.status.success() {
            pass!("{label} exit 0");
        } else {
            failm!("{label} exit {}", r.status.code().unwrap_or(1));
        }
    };

    println!("\nverify-cartographic: glyph atlas");
    run_cargo(&["schema", "map-glyphs"]);
    println!("\nverify-cartographic: export artifacts (G6 subset)");
    run_world(&["validate-exports"]);
    println!("\nverify-cartographic: P5_props phase census");
    run_world(&["verify-phase", "--terrain", "everon", "--phase", "P5_props"]); // E2c-allow
    println!("\nverify-cartographic: locations");
    run_cargo(&["schema", "locations", "--terrain", "everon"]); // E2c-allow
    println!("\nverify-cartographic: height labels");
    run_cargo(&["schema", "height-labels", "--terrain", "everon"]); // E2c-allow
    println!("\nverify-cartographic: town labels");
    run_cargo(&["schema", "town-labels", "--terrain", "everon", "--zoom=-2"]); // E2c-allow
    println!("\nverify-cartographic: road names");
    run_cargo(&["schema", "road-names", "--terrain", "everon", "--zoom", "0"]); // E2c-allow

    println!("\nverify-cartographic: wasm telemetry");
    println!("  SKIP  wasm size guard — `cargo xtask mk wasm-ci` owns the engine crates");

    println!();
    if failures.get() > 0 {
        eprintln!("verify-cartographic: FAIL ({})", failures.get());
        return Ok(1);
    }
    println!("verify-cartographic: OK");
    Ok(0)
}
