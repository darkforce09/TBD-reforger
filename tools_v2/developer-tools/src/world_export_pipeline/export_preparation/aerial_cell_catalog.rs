use super::*;

/// Enumerate Everon SAP supertexture cells → staging/sap/cell-catalog.json (fast index; the
/// full decode + fail-fast lives in the stitch step).
pub fn catalog_sap_cells(terrain: &str) -> Result<u8> {
    use super::super::enfusion_texture_decoder::{
        CELL_COUNT, CELL_M, CELL_PX, GRID, WORLD_M, cell_grid, cell_path,
    };
    use crate::enfusion_pak::PakVfs;
    if terrain != "everon" {
        // E2c-allow: the SAP lane is Eden-only this slice (matches the .mjs guard)
        eprintln!("only everon supported this slice (got {terrain})"); // E2c-allow
        return Ok(1);
    }
    let out_dir = repo_root().join("packages/map-assets/everon/staging/sap"); // E2c-allow
    let vfs = PakVfs::open_default()?;
    let cells = super::super::enfusion_texture_decoder::list_eden_cells(&vfs);
    if cells.len() as u32 != CELL_COUNT {
        eprintln!(
            "FAIL: found {} Eden cells, expected {CELL_COUNT}",
            cells.len()
        );
        return Ok(1);
    }
    let entries: Vec<Value> = cells
        .iter()
        .map(|(n, _)| {
            let (gx, gy) = cell_grid(*n);
            json!({
                "id": n,
                "eddsPath": cell_path(*n),
                "gridX": gx,
                "gridY": gy,
                "widthPx": CELL_PX,
                "heightPx": CELL_PX,
                "worldMinX": gx * CELL_M,
                "worldMinZ": gy * CELL_M,
                "pixelX": gx * CELL_PX,
                "pixelY": (GRID - 1 - gy) * CELL_PX,
            })
        })
        .collect();
    let generated_at = {
        let full = iso_from_system_time(std::time::SystemTime::now());
        // toISOString().replace(/\.\d+Z$/, "Z") — seconds precision.
        format!("{}Z", &full[..19])
    };
    let catalog = json!({
        "terrain": terrain,
        "slice": "T-090.1.2",
        "generatedAt": generated_at,
        "grid": GRID,
        "cellCount": entries.len(),
        "cellMeters": CELL_M,
        "cellPx": CELL_PX,
        "metersPerPixel": super::super::json_number_formatting::js_num(f64::from(CELL_M) / f64::from(CELL_PX)),
        "worldBounds": [0, 0, WORLD_M, WORLD_M],
        "orthoPx": [GRID * CELL_PX, GRID * CELL_PX],
        "gridMapping": "row-major N=y*50+x; cell gridY=0 = world Z=0 (south); ortho north-up (south at image bottom, pixelY=(49-gridY)*256)",
        "source": "sap-supertexture-stitch",
        "cells": entries,
    });
    // T-537: refuse an empty cell catalog overwrite.
    super::super::refuse_empty_write(
        "catalog-sap-cells",
        catalog["cellCount"].as_u64() != Some(u64::from(CELL_COUNT))
            || catalog["cells"].as_array().map(|a| a.len()).unwrap_or(0) == 0,
        &format!(
            "cellCount {} != expected {CELL_COUNT} — refusing empty/partial cell-catalog.json overwrite",
            catalog["cellCount"]
        ),
    )?;
    std::fs::create_dir_all(&out_dir)?;
    let out = out_dir.join("cell-catalog.json");
    std::fs::write(&out, serde_json::to_string_pretty(&catalog)? + "\n")?;
    println!(
        "wrote {} ({} cells, ortho {}x{})",
        out.display(),
        catalog["cellCount"],
        GRID * CELL_PX,
        GRID * CELL_PX
    );
    Ok(0)
}
