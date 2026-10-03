//! The game mod's cached mission artifact in a profile.
//!
//! **Role:** stages a compiled mission document as the mod's last verified artifact in the
//! profile's `TBD_MissionArtifactCache/` (`document.json` plus `identity.json`, the deployment wire
//! the mod caches), and clears it.
//! **Position:** under [`crate::website_api_client`]; `mod test-mission`, the playtest's offline
//! artifact and the compiled world boot stage through it.
//! **Signals & state:** none beyond the cache files it writes.
//! **Invariants:** a server with no platform credential boots the staged mission offline; the mod
//! hashes the bytes again and loads them only when they match the identity's SHA-256.

use std::fs;
use std::path::Path;

use crate::error::{Result, ResultExt};
use serde_json::json;

use content_digest::sha256_hex;

/// The cache directory inside a profile directory (`$profile:` in the mod).
pub(crate) const ARTIFACT_CACHE_DIRECTORY: &str = "TBD_MissionArtifactCache";

/// What the staged identity says about the document.
pub(crate) struct StagedArtifact<'a> {
    pub(crate) artifact_id: &'a str,
    pub(crate) mission_id: &'a str,
    pub terrain_key: &'a str,
}

/// Write `document` and its identity under `profile_directory`, the identity last and only after
/// the old one is gone, as the mod itself writes the cache. The document's SHA-256 is returned.
pub(crate) fn stage_artifact_cache(
    profile_directory: &Path,
    document: &[u8],
    staged: &StagedArtifact<'_>,
) -> Result<String> {
    let directory = profile_directory.join(ARTIFACT_CACHE_DIRECTORY);
    fs::create_dir_all(&directory).with_context(|| format!("create {}", directory.display()))?;
    let identity_path = directory.join("identity.json");
    if identity_path.exists() {
        fs::remove_file(&identity_path)
            .with_context(|| format!("remove {}", identity_path.display()))?;
    }
    fs::write(directory.join("document.json"), document)
        .with_context(|| format!("write {}", directory.join("document.json").display()))?;
    let sha256 = sha256_hex(document);
    let identity = json!({
        "deployment_id": "",
        "state": "confirmed",
        "mission_id": staged.mission_id,
        "artifact_id": staged.artifact_id,
        "artifact_sha256": sha256,
        "artifact_bytes": document.len(),
        "terrain_key": staged.terrain_key,
        "scenario_id": "",
        "event_id": "",
        "event_mission_id": "",
    });
    fs::write(&identity_path, format!("{identity:#}\n"))
        .with_context(|| format!("write {}", identity_path.display()))?;
    Ok(sha256)
}

/// Remove a staged artifact, so the next boot reads the platform deployment instead.
pub(crate) fn clear_artifact_cache(profile_directory: &Path) -> Result<bool> {
    let directory = profile_directory.join(ARTIFACT_CACHE_DIRECTORY);
    if !directory.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(&directory).with_context(|| format!("remove {}", directory.display()))?;
    Ok(true)
}
