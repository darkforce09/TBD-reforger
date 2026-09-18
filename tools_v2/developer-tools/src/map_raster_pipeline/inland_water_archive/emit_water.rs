use super::*;

/// `map water --terrain <id|dir>` — both water binaries from one staging directory.
///
/// # Errors
/// Any staging file being missing or malformed, or either write failing. Returns exit code 1
/// (rather than an error) when the terrain or staging directory is simply absent, matching the
/// rest of the `map` lane.
pub fn emit_water(terrain: &str) -> Result<u8> {
    let dir = terrain_dir(terrain);
    if !dir.is_dir() {
        eprintln!("water: no terrain directory at {}", dir.display());
        return Ok(1);
    }
    let staging = dir.join(STAGING_WATER);
    if !staging.is_dir() {
        eprintln!(
            "water: no staging export at {} — run the Workbench inland-water exporter first",
            staging.display()
        );
        return Ok(1);
    }

    let vectors_src = staging.join(VECTORS_JSON);
    let archive = build_water_vectors(
        &std::fs::read_to_string(&vectors_src)
            .with_context(|| format!("read {}", vectors_src.display()))?,
    )?;
    super::super::refuse_empty_write(
        "water",
        archive.lakes.is_empty() && archive.rivers.is_empty() && archive.ponds.is_empty(),
        "no lakes, rivers or ponds — refusing to write an empty water_vectors.rkyv",
    )?;
    let vectors_out = dir.join(WATER_VECTORS_RKYV);
    let vectors_bytes = write_water_vectors(&vectors_out, &archive)?;

    let meta_src = staging.join(META_JSON);
    let meta = parse_meta(
        &std::fs::read_to_string(&meta_src)
            .with_context(|| format!("read {}", meta_src.display()))?,
    )?;
    let bath_out = dir.join(BATHYMETRY_TBDB);
    let bath_bytes = write_bathymetry(
        &bath_out,
        &meta,
        &staging.join(DEPTH_TXT),
        &staging.join(MASK_TXT),
    )?;

    println!(
        "water: {} lakes + {} rivers + {} ponds → {} ({vectors_bytes} bytes)",
        archive.lakes.len(),
        archive.rivers.len(),
        archive.ponds.len(),
        vectors_out.display()
    );
    println!(
        "water: {}×{} × {} mips @ {} m/unit → {} ({bath_bytes} bytes)",
        meta.width,
        meta.height,
        mip_count(meta.width, meta.height),
        meta.depth_scale,
        bath_out.display()
    );
    Ok(0)
}
