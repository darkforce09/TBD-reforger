use super::*;
use crate::repository_layout::terrain_registry_path;

/// T-378: only an intentional density rebuild may wipe `objects/density/`.
/// Non-density phases must leave the 625 committed bins alone.
pub fn may_clear_density_dir(density_phase: bool) -> bool {
    density_phase
}

/// Clear `density_dir` only when `rebuilding` is true (density phase / redensify).
/// Returns whether a clear was attempted. Non-density callers must pass `false`.
pub fn clear_density_dir_if_rebuilding(density_dir: &Path, rebuilding: bool) -> Result<bool> {
    if !may_clear_density_dir(rebuilding) {
        return Ok(false);
    }
    match std::fs::remove_dir_all(density_dir) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(e.into()),
    }
}

/// Cumulative kind filter per import phase (t090_phased_object_import.md).
pub fn phase_kinds(phase: &str) -> Option<&'static [&'static str]> {
    Some(match phase {
        "P1_buildings" => &["building"],
        "P2_trees" => &["building", "tree", "water"],
        "P3_vegetation" => &["building", "tree", "water", "vegetation"],
        "P4_rocks" => &["building", "tree", "water", "vegetation", "rock"],
        "P5_props" => &[
            "building",
            "tree",
            "water",
            "vegetation",
            "rock",
            "prop",
            "vehicle",
        ],
        _ => return None,
    })
}

pub fn terrain_row(terrain: &str) -> Result<Value> {
    let reg: Value = serde_json::from_str(&std::fs::read_to_string(terrain_registry_path(
        &repo_root(),
    ))?)?;
    reg["terrains"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t["terrainId"] == terrain).cloned())
        .ok_or_else(|| anyhow::anyhow!("terrain '{terrain}' not in terrain-registry.json"))
}

pub fn gz9(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(9));
    enc.write_all(bytes)?;
    Ok(enc.finish()?)
}

pub fn gunzip(bytes: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read as _;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes).read_to_end(&mut out)?;
    Ok(out)
}

pub(super) fn compact(v: &Value) -> String {
    serde_json::to_string(v).expect("json")
}

pub(super) fn pretty_nl(v: &Value) -> String {
    serde_json::to_string_pretty(v).expect("json") + "\n"
}

/// The full build-world-objects.mjs port. `out_base = None` → the real terrain dir.
pub fn build_world_objects(
    terrain: &str,
    phase: &str,
    out_base: Option<&Path>,
    patch_manifest: bool,
    ops_log: bool,
) -> Result<BuildSummary> {
    build_world_objects_opt(terrain, phase, out_base, patch_manifest, ops_log, false)
}
