use super::*;

/// The requested phase must not exceed the registry's importPhaseMax.
pub fn phase_gate(terrain: &str, phase: &str) -> Result<u8> {
    const ORDER: [&str; 10] = [
        "P1_buildings",
        "P2_trees",
        "P3_vegetation",
        "P4_rocks",
        "P5_props",
        "P6_roads_highway",
        "P7_roads_paved",
        "P8_roads_dirt",
        "P9_roads_path",
        "P10_full",
    ];
    let reg: Value = serde_json::from_str(&std::fs::read_to_string(
        repo_root().join("packages/map-assets/terrain-registry.json"),
    )?)?;
    let Some(row) = reg["terrains"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t["terrainId"] == terrain))
    else {
        eprintln!("export-terrain: terrain '{terrain}' not in terrain-registry.json");
        return Ok(1);
    };
    let Some(p_idx) = ORDER.iter().position(|p| *p == phase) else {
        eprintln!("export-terrain: unknown phase '{phase}'");
        return Ok(1);
    };
    let max = row["importPhaseMax"].as_str().unwrap_or("");
    let max_idx = ORDER.iter().position(|p| *p == max);
    if max_idx.is_none() || p_idx > max_idx.unwrap() {
        eprintln!(
            "export-terrain: phase {phase} blocked — registry importPhaseMax={} (advance only after map-verify-phase PASS + registry bump)",
            if max.is_empty() { "(none)" } else { max }
        );
        return Ok(1);
    }
    Ok(0)
}
