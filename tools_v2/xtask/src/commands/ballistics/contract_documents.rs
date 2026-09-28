//! The serialized shapes of the ballistics catalog and its calibration bundle.
//!
//! **Role:** Serde projections of `ballistics-catalog.schema.json` and
//! `ballistics-calibration.schema.json`, with fields in schema order so the written JSON reads in
//! the order the schemas document.
//!
//! **Position:** Filled by the catalog, table and oracle readers of `ballistics trim-export` and
//! written by its document writer; `cargo xtask schema validate` checks the written files against
//! the schemas.
//!
//! **Signals & state:** none; plain data.
//!
//! **Invariants:** Every optional schema property is skipped when absent rather than written as
//! `null`. Oracle sample `inputs` and `outputs` stay free-form JSON objects under the engine's
//! parameter names, as the schema allows.
//!
//! @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalog
//! @contract ballistics-calibration.schema.json#/definitions/CalibrationBundle
use serde::Serialize;
use serde_json::{Map, Value};

/// One game resource with the SHA-256 of its exported bytes.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ResourceRecord {
    pub(crate) guid: String,
    pub(crate) resource_name: String,
    pub(crate) sha256: String,
}

/// A ballistics catalog document.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct BallisticsCatalog {
    pub(crate) schema_version: u32,
    pub(crate) catalog_id: String,
    pub(crate) catalog_version: u32,
    pub(crate) title: String,
    pub(crate) game_build: String,
    pub(crate) export_generation_id: String,
    pub(crate) gravity_m_s2: f64,
    pub(crate) gravity_source: String,
    pub(crate) resources: Vec<ResourceRecord>,
    pub(crate) weapons: Vec<WeaponSystem>,
    pub(crate) shells: Vec<Shell>,
}

/// One weapon of the catalog.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct WeaponSystem {
    pub(crate) weapon_id: String,
    pub(crate) display_name: String,
    pub(crate) prefab_guid: String,
    pub(crate) caliber_mm: f64,
    pub(crate) mils_per_circle: u32,
    pub(crate) elevation_min_deg: f64,
    pub(crate) elevation_max_deg: f64,
    pub(crate) muzzle_init_speed_coef: f64,
    pub(crate) dispersion_diameter_m: f64,
    pub(crate) dispersion_range_m: f64,
    pub(crate) shell_ids: Vec<String>,
}

/// One shell of the catalog.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Shell {
    pub(crate) shell_id: String,
    pub(crate) display_name: String,
    pub(crate) prefab_guid: String,
    pub(crate) role: String,
    pub(crate) init_speed_m_s: f64,
    pub(crate) init_speed_variation: f64,
    pub(crate) mass_kg: f64,
    pub(crate) air_drag: f64,
    pub(crate) side_air_drag_scale: f64,
    pub(crate) wind_influence_multiplier: f64,
    pub(crate) dispersion_multiplier: f64,
    pub(crate) time_to_live_s: f64,
    pub(crate) standard_dispersion_m: f64,
    pub(crate) charges: Vec<Charge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) time_fuze: Option<TimeFuze>,
}

/// One charge (ring count) of a shell.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Charge {
    pub(crate) rings: u32,
    pub(crate) init_speed_coef: f64,
    pub(crate) is_default: bool,
}

/// A shell's time fuze settings.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct TimeFuze {
    pub(crate) min_s: f64,
    pub(crate) max_s: f64,
    pub(crate) default_s: f64,
}

/// A calibration bundle document.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct CalibrationBundle {
    pub(crate) schema_version: u32,
    pub(crate) catalog_id: String,
    pub(crate) catalog_version: u32,
    pub(crate) catalog_sha256: String,
    pub(crate) game_build: String,
    pub(crate) export_generation_id: String,
    pub(crate) resources: Vec<ResourceRecord>,
    pub(crate) oracle_run: OracleRun,
    pub(crate) native_tables: Vec<NativeTable>,
    pub(crate) wind_tables: Vec<WindTable>,
    pub(crate) oracle_samples: Vec<OracleSample>,
}

/// The oracle plugin run the samples and the gravity come from.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct OracleRun {
    pub(crate) plugin_revision: String,
    pub(crate) gravity_reported_m_s2: f64,
    pub(crate) run_at: String,
    pub(crate) output_sha256: String,
}

/// One game ballistic table at one muzzle speed coefficient.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct NativeTable {
    pub(crate) shell_id: String,
    pub(crate) init_speed_coef: f64,
    pub(crate) rows: Vec<NativeTableRow>,
}

/// One native table row with its oracle-assigned elevation.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct NativeTableRow {
    pub(crate) lattice_index: usize,
    pub(crate) elevation_mils_6400: f64,
    pub(crate) range_m: f64,
    pub(crate) column_1: f64,
    pub(crate) time_of_flight_s: f64,
}

/// One game wind table at one coefficient and wind speed.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct WindTable {
    pub(crate) shell_id: String,
    pub(crate) init_speed_coef: f64,
    pub(crate) wind_speed_m_s: f64,
    pub(crate) rows: Vec<WindTableRow>,
}

/// One wind table row.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct WindTableRow {
    pub(crate) elevation_rad: f64,
    pub(crate) range_m: f64,
    pub(crate) apex_m: f64,
    pub(crate) values: Vec<f64>,
}

/// One engine oracle call.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct OracleSample {
    pub(crate) kind: String,
    pub(crate) shell_id: String,
    pub(crate) init_speed_coef: f64,
    pub(crate) inputs: Map<String, Value>,
    pub(crate) outputs: Map<String, Value>,
}

/// `value` as pretty JSON with a final newline: the byte form every committed document takes.
pub(crate) fn document_bytes<T: Serialize>(value: &T) -> anyhow::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}
