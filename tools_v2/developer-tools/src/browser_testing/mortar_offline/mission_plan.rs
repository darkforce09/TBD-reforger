//! What the offline mortar gate needs on disk, and the fire mission it enters.
//!
//! **Role:** names every input the gate cannot run without — the built app, the Everon manifest,
//! elevation model, map tile index and satellite bundle, and the recorded catalog reads — and
//! refuses with the missing file named; derives the recorded catalog reads from the committed
//! vanilla catalog; chooses the shell and the gun position of the mission the gate types.
//! **Position:** the first step of `super::run` (the preflight) and the source of the numbers
//! `super::page_driver` types; the catalog it decodes is the one the gate server serves, so the
//! native solve in `super::expected_solution` uses the same document the page used.
//! **Signals & state:** none; pure functions over paths and plain values.
//! **Invariants:** a missing input is an error naming the file, never a smaller run; the recorded
//! read names come from the committed catalog's own `catalog_id` and `catalog_version`; the gun
//! stays on the map.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use axum::http::Method;
use website_map_engine::data::scenario::ballistics::catalog::{BallisticsCatalog, ShellRole};

use crate::browser_testing::server::api_fixture_corpus::corpus_file_name;

/// The committed vanilla catalog, relative to the repository root.
pub const COMMITTED_CATALOG: &str =
    "contracts_v2/catalogs/ballistics/vanilla_mortars.v1.catalog.json";

/// The recorded API corpus the gate server answers `/api/` from by default, relative to the
/// repository root.
pub const API_CORPUS_DIR: &str = "apps/website/frontend/tests/fixtures/api";

/// The terrain the offline pack holds.
pub const OFFLINE_TERRAIN: &str = "everon";

/// The target height the gate types, metres.
pub const TARGET_HEIGHT_M: f64 = 42.0;

/// The gun height the gate types, metres.
pub const GUN_HEIGHT_M: f64 = 18.5;

/// The wind the gate types: speed in metres per second and the direction it blows from, degrees.
pub const WIND: (f64, f64) = (3.5, 250.0);

/// How far south of the target the gun stands, metres.
pub const GUN_OFFSET_M: f64 = 1_200.0;

/// The two recorded reads the page makes: the catalog list and the committed catalog's version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogReads {
    /// `GET /api/v1/ballistics-catalogs` as a corpus file name.
    pub list_file: String,
    /// `GET /api/v1/ballistics-catalogs/{catalog_id}/versions/{version}` as a corpus file name.
    pub version_file: String,
}

/// The recorded reads that serve `catalog`.
///
/// # Errors
///
/// When the catalog's id cannot form a corpus file name.
pub fn catalog_reads(catalog: &BallisticsCatalog) -> Result<CatalogReads> {
    let name = |path: String| {
        corpus_file_name(&Method::GET, &path)
            .ok_or_else(|| anyhow!("no corpus file name for GET {path}"))
    };
    Ok(CatalogReads {
        list_file: name("/api/v1/ballistics-catalogs".to_string())?,
        version_file: name(format!(
            "/api/v1/ballistics-catalogs/{}/versions/{}",
            catalog.catalog_id, catalog.catalog_version
        ))?,
    })
}

/// Reads and decodes a catalog document.
///
/// # Errors
///
/// When the file is unreadable or not a catalog.
pub fn read_catalog(path: &Path) -> Result<BallisticsCatalog> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    BallisticsCatalog::from_json_slice(&bytes)
        .map_err(|e| anyhow!("decode {}: {e}", path.display()))
}

/// Every file the gate reads before it launches a browser, with what each one is: the app in
/// `dist`, the terrain under `root` and the recorded reads in `corpus`.
#[must_use]
pub fn required_files(
    root: &Path,
    dist: &Path,
    corpus: &Path,
    reads: &CatalogReads,
) -> Vec<(PathBuf, String)> {
    let terrain = root.join("assets_v2/terrains").join(OFFLINE_TERRAIN);
    vec![
        (dist.join("index.html"), "the built app".to_string()),
        (
            dist.join("service_worker.js"),
            "the service worker loader of the built app".to_string(),
        ),
        (
            terrain.join("manifest.json"),
            "the Everon manifest".to_string(),
        ),
        (
            terrain.join("dem/everon-dem-16bit.png"),
            "the Everon elevation model".to_string(),
        ),
        (
            terrain.join("satellite/everon-sat.tbd-sat"),
            "the Everon satellite bundle".to_string(),
        ),
        (
            terrain.join("tiles/map/index.json"),
            "the map tile index (`cargo xtask map tile-index --terrain everon`)".to_string(),
        ),
        (
            corpus.join(&reads.list_file),
            "the recorded catalog list (waiting on goldens)".to_string(),
        ),
        (
            corpus.join(&reads.version_file),
            "the recorded catalog version (waiting on goldens)".to_string(),
        ),
    ]
}

/// Refuses when any required file is missing, naming every one.
///
/// # Errors
///
/// The list of missing files.
pub fn require_files(files: &[(PathBuf, String)]) -> Result<()> {
    let missing: Vec<String> = files
        .iter()
        .filter(|(path, _)| !path.is_file())
        .map(|(path, what)| format!("{} ({what})", path.display()))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        bail!("assets missing: {}", missing.join("; "))
    }
}

/// The first high-explosive shell `weapon_id` fires.
///
/// # Errors
///
/// When the weapon is unknown or fires no high-explosive shell.
pub fn high_explosive_shell(catalog: &BallisticsCatalog, weapon_id: &str) -> Result<String> {
    let weapon = catalog
        .weapons
        .iter()
        .find(|w| w.weapon_id == weapon_id)
        .ok_or_else(|| anyhow!("the catalog has no weapon {weapon_id}"))?;
    weapon
        .shell_ids
        .iter()
        .find(|id| {
            catalog
                .shells
                .iter()
                .any(|s| &s.shell_id == *id && s.role == ShellRole::He && s.time_fuze.is_none())
        })
        .cloned()
        .ok_or_else(|| anyhow!("weapon {weapon_id} fires no high-explosive shell"))
}

/// The gun position for a target at `(x, y)`: [`GUN_OFFSET_M`] south, or north when south
/// would leave the map.
#[must_use]
pub fn gun_position(target_x: f64, target_y: f64) -> (f64, f64) {
    if target_y - GUN_OFFSET_M >= 0.0 {
        (target_x, target_y - GUN_OFFSET_M)
    } else {
        (target_x, target_y + GUN_OFFSET_M)
    }
}

#[cfg(test)]
#[path = "../tests/mortar_offline/mission_plan.rs"]
mod tests;
