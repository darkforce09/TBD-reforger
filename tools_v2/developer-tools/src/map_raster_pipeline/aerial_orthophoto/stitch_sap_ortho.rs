use super::*;

/// stitch-sap-ortho.mjs port: decode all 2500 cells, assemble north-up, bridge seams, write
/// PNG + TBD_SatExport_meta.json.
pub fn stitch_sap_ortho(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let out_dir = sap_dir();
    let t0 = std::time::Instant::now();
    let vfs = PakVfs::open_default()?;
    let cells = enfusion_texture_decoder::list_eden_cells(&vfs);
    if cells.len() as u32 != enfusion_texture_decoder::CELL_COUNT {
        eprintln!(
            "FAIL: found {} Eden cells, expected {} — aborting (no holes)",
            cells.len(),
            enfusion_texture_decoder::CELL_COUNT
        );
        return Ok(1);
    }
    let cell_px = enfusion_texture_decoder::CELL_PX as usize;
    let grid = enfusion_texture_decoder::GRID as usize;
    let ortho_px = grid * cell_px;
    let stride = ortho_px * 4;
    let mut canvas = vec![0u8; ortho_px * ortho_px * 4];
    let mut decoded = 0u32;
    for (n, _) in &cells {
        let cell = match enfusion_texture_decoder::decode_cell_rgba(&vfs, *n) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("FAIL: cell {n} decode error: {e} — aborting (no grey fill)");
                return Ok(1);
            }
        };
        if cell.side != cell_px || cell.rgba.len() != cell_px * cell_px * 4 {
            eprintln!("FAIL: cell {n} wrong size (side {}) — aborting", cell.side);
            return Ok(1);
        }
        let (gx, gy) = enfusion_texture_decoder::cell_grid(*n);
        let px = gx as usize * cell_px;
        let py_top = (grid - 1 - gy as usize) * cell_px;
        let cell_stride = cell_px * 4;
        for row in 0..cell_px {
            let dst_row = py_top + (cell_px - 1 - row);
            let dst = dst_row * stride + px * 4;
            canvas[dst..dst + cell_stride]
                .copy_from_slice(&cell.rgba[row * cell_stride..(row + 1) * cell_stride]);
        }
        decoded += 1;
        if decoded.is_multiple_of(250) {
            eprintln!(
                "  decoded {decoded}/{}",
                enfusion_texture_decoder::CELL_COUNT
            );
        }
    }
    let seams = bridge_seams(&mut canvas, ortho_px, 4)?;
    eprintln!("  seam repair: bridged {seams} interior seams/axis (apron feather HW=4)");

    // T-537: refuse writing meta/PNG claiming success with an incomplete or empty stitch.
    super::super::refuse_empty_write(
        "stitch-sap-ortho cells",
        decoded == 0 || decoded != enfusion_texture_decoder::CELL_COUNT,
        &format!(
            "decoded {decoded} cells (expected {}) — refusing empty/partial SAP overwrite",
            enfusion_texture_decoder::CELL_COUNT
        ),
    )?;

    std::fs::create_dir_all(&out_dir)?;
    // RGBA → RGB (drop alpha; the ortho is opaque).
    let mut rgb = Vec::with_capacity(ortho_px * ortho_px * 3);
    for px in canvas.chunks_exact(4) {
        rgb.extend_from_slice(&px[..3]);
    }
    super::super::refuse_empty_write(
        "stitch-sap-ortho rgb",
        rgb.is_empty() || rgb.len() != ortho_px * ortho_px * 3,
        "RGB buffer empty or wrong size — refusing SAP ortho/meta overwrite",
    )?;
    let png_path = out_dir.join("everon-sap-ortho.png");
    image_operations::save_png_rgb(
        &png_path,
        &Rgb8 {
            w: ortho_px,
            h: ortho_px,
            data: rgb,
        },
    )?;

    let elapsed = t0.elapsed().as_secs();
    let generated_at = {
        let full = iso_from_system_time(std::time::SystemTime::now());
        format!("{}Z", &full[..19])
    };
    let meta = json!({
        "slice": "T-090.1.2",
        "source": "sap-supertexture-stitch",
        "captureMethodId": 6,
        "terrain": terrain,
        "dimensions": [ortho_px, ortho_px],
        "metersPerPixel": 1,
        "worldBounds": [0, 0, enfusion_texture_decoder::WORLD_M, enfusion_texture_decoder::WORLD_M],
        "grid": grid,
        "cellsDecoded": decoded,
        "cellPx": cell_px,
        "cellMeters": enfusion_texture_decoder::CELL_M,
        "gridMapping": "row-major N=y*50+x; cell gridY=0 = world Z=0 (south); assembled north-up (south at image bottom)",
        "decoder": "tbd-tools world::edds (bcdec_rs BC7 + Rust LZ4) — T-165.9",
        "seamRepair": "T-090.1.2.2",
        "seamRepairStrategy": format!("A-apron-bridge-{HW}px"),
        "seamRepairParams": { "halfWidthPx": HW, "anchorOffsetPx": ANCHOR, "interiorSeamsOnly": true },
        "pngPath": "assets_v2/scratch/everon/sap/everon-sap-ortho.png",
        "buildSeconds": elapsed,
        "generatedAt": generated_at,
    });
    std::fs::write(
        out_dir.join("TBD_SatExport_meta.json"),
        serde_json::to_string_pretty(&meta)? + "\n",
    )?;
    println!(
        "wrote {} ({ortho_px}x{ortho_px}, {decoded} cells, {elapsed}s)",
        png_path.display()
    );
    Ok(0)
}

/// blend-sap-seams CLI fallback: bridge the EXISTING ortho PNG in place.
pub fn blend_sap_seams_cli(terrain: &str) -> Result<u8> {
    if terrain != "everon" {
        eprintln!("only everon supported this slice (got {terrain})");
        return Ok(1);
    }
    let sap = sap_dir();
    let png_path = sap.join("everon-sap-ortho.png");
    let meta_path = sap.join("TBD_SatExport_meta.json");
    if !png_path.exists() {
        eprintln!(
            "FAIL: {} missing — run the stitch first",
            png_path.display()
        );
        return Ok(1);
    }
    eprintln!(
        "blend-sap-seams (CLI fallback): decoding {} …",
        png_path.display()
    );
    let mut ortho = image_operations::load_png_rgb(&png_path)?;
    let n = bridge_seams(&mut ortho.data, ORTHO_PX, 3)?;
    image_operations::save_png_rgb(&png_path, &ortho)?;
    if meta_path.exists() {
        let mut meta: Value = serde_json::from_str(&std::fs::read_to_string(&meta_path)?)?;
        meta["seamRepair"] = json!("T-090.1.2.2");
        meta["seamRepairStrategy"] = json!(format!("A-apron-bridge-{HW}px"));
        meta["seamRepairParams"] =
            json!({ "halfWidthPx": HW, "anchorOffsetPx": ANCHOR, "interiorSeamsOnly": true });
        std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)? + "\n")?;
    }
    println!(
        "blend-sap-seams: bridged {n} interior seams/axis in {}",
        png_path.display()
    );
    Ok(0)
}
