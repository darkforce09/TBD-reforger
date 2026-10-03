//! The ballistics oracle's Workbench output for one export generation, verified.
//!
//! **Role:** Reads `forward_angles.json` and `simulation.json` with their `_meta.json` sidecars
//! from the oracle directory, verifies each file's size and SHA-256 against its sidecar, checks
//! that both runs completed without errors for the same generation, game build and plugin
//! revision, and exposes the per-shell samples, the forward elevation lattice and the gravity the
//! physics world reported.
//!
//! **Position:** Read by the trim after the export; its samples become the bundle's
//! `oracle_samples`, its forward samples fix native row elevations, and its gravity becomes the
//! catalog's `gravity_m_s2`.
//!
//! **Signals & state:** [`OracleOutput`] owns the parsed, number-normalized documents.
//!
//! **Invariants:** A missing file, a hash or size that differs from the sidecar, an incomplete run,
//! any recorded error, and a generation, build or revision that differs between the files or from
//! the export are errors. `output_sha256` is the SHA-256 of the `sha256sum` listing of the two
//! output files in name order, so it pins both.
use super::engine_numbers::normalize_engine_numbers;
use super::gameplay_export::read_json_file;
use crate::error::{Result, ResultExt, refuse};
use content_digest::sha256_hex;
use serde_json::Value;
use std::path::Path;

/// Forward-angle and altitude output of the oracle.
pub(crate) const FORWARD_ANGLES_FILE: &str = "forward_angles.json";
/// Projectile simulation output of the oracle.
pub(crate) const SIMULATION_FILE: &str = "simulation.json";

/// The forward-angle elevation lattice the oracle sampled, in 6400-mil units.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ForwardLattice {
    pub(crate) first_mils: f64,
    pub(crate) last_mils: f64,
    pub(crate) step_mils: f64,
}

/// A verified oracle run.
pub(crate) struct OracleOutput {
    pub(crate) plugin_revision: String,
    pub(crate) run_at: String,
    /// The magnitude the physics world reported, as the oracle wrote it.
    pub(crate) gravity_raw_m_s2: f64,
    /// [`Self::gravity_raw_m_s2`] as the shortest decimal of its 32-bit float.
    pub(crate) gravity_m_s2: f64,
    pub(crate) gravity_source: String,
    pub(crate) output_sha256: String,
    /// `(file name, sha256)` of both output files, in name order.
    pub(crate) file_sha256: Vec<(String, String)>,
    pub(crate) lattice: ForwardLattice,
    forward: Value,
    simulation: Value,
}

fn verified_output(
    directory: &Path,
    file: &str,
    generation_id: &str,
    game_build: &str,
) -> Result<(String, Value, Value)> {
    let path = directory.join(file);
    if !path.is_file() {
        refuse!(
            "oracle output {} is missing: run the ballistics oracle for generation {generation_id} first",
            path.display()
        );
    }
    let (bytes, document) = read_json_file(&path)?;
    let meta_path = directory.join(file.replace(".json", "_meta.json"));
    let (_, meta) = read_json_file(&meta_path)?;
    let sha256 = sha256_hex(&bytes);
    if meta["file"] != file
        || meta["sha256"] != sha256.as_str()
        || meta["bytes"].as_u64() != Some(bytes.len() as u64)
    {
        refuse!(
            "{} ({} bytes, sha256 {sha256}) does not match its sidecar {}",
            path.display(),
            bytes.len(),
            meta_path.display()
        );
    }
    for (label, value) in [("sidecar", &meta), ("output", &document)] {
        if value["status"] != "complete" {
            refuse!("{file} {label} status is {}, not complete", value["status"]);
        }
        if value["export_generation_id"] != generation_id || value["game_build"] != game_build {
            refuse!(
                "{file} {label} is for generation {} build {}, not generation {generation_id} build {game_build}",
                value["export_generation_id"],
                value["game_build"]
            );
        }
        if value["plugin_revision"] != meta["plugin_revision"] {
            refuse!("{file} {label} plugin revision differs from its sidecar");
        }
    }
    if document["shell_error_count"].as_u64() != Some(0)
        || document["errors"]
            .as_array()
            .is_some_and(|errors| !errors.is_empty())
    {
        refuse!("{file} records oracle errors");
    }
    for shell in document["shells"].as_array().into_iter().flatten() {
        for key in ["errors", "shell_errors"] {
            if shell[key]
                .as_array()
                .is_some_and(|errors| !errors.is_empty())
            {
                refuse!("{file} records errors for shell {}", shell["prefab_guid"]);
            }
        }
    }
    Ok((sha256, document, meta))
}

impl OracleOutput {
    /// Reads and verifies the oracle output in `directory` for the export `generation_id` of
    /// `game_build`.
    pub(crate) fn open(directory: &Path, generation_id: &str, game_build: &str) -> Result<Self> {
        let (forward_sha256, forward, forward_meta) =
            verified_output(directory, FORWARD_ANGLES_FILE, generation_id, game_build)?;
        let (simulation_sha256, simulation, simulation_meta) =
            verified_output(directory, SIMULATION_FILE, generation_id, game_build)?;
        let plugin_revision = forward_meta["plugin_revision"]
            .as_str()
            .context("oracle sidecar names no plugin revision")?
            .to_owned();
        if simulation_meta["plugin_revision"] != plugin_revision.as_str() {
            refuse!("the two oracle outputs come from different plugin revisions");
        }
        let run_at = [&forward_meta, &simulation_meta]
            .iter()
            .filter_map(|meta| meta["run_at"].as_str())
            .min()
            .context("oracle sidecars record no run_at")?
            .to_owned();
        let gravity = &simulation["gravity"];
        let gravity_raw_m_s2 = gravity["magnitude_m_s2"]
            .as_f64()
            .filter(|value| *value > 0.0)
            .context("simulation.json reports no positive gravity magnitude")?;
        let lattice = &forward["elevation_lattice"];
        let lattice_number = |key: &str| {
            lattice[key]
                .as_f64()
                .with_context(|| format!("forward_angles.json elevation_lattice has no {key}"))
        };
        if lattice["mils_per_circle"].as_u64() != Some(6400) {
            refuse!("forward_angles.json lattice is not in 6400-mil units");
        }
        let lattice = ForwardLattice {
            first_mils: lattice_number("first_mils")?,
            last_mils: lattice_number("last_mils")?,
            step_mils: lattice_number("step_mils")?,
        };
        let file_sha256 = vec![
            (FORWARD_ANGLES_FILE.to_owned(), forward_sha256),
            (SIMULATION_FILE.to_owned(), simulation_sha256),
        ];
        let listing: String = file_sha256
            .iter()
            .map(|(file, sha256)| format!("{sha256}  {file}\n"))
            .collect();
        Ok(Self {
            plugin_revision,
            run_at,
            gravity_raw_m_s2,
            gravity_m_s2: super::engine_numbers::engine_number(gravity_raw_m_s2),
            gravity_source: gravity["source"]
                .as_str()
                .unwrap_or("unrecorded")
                .to_owned(),
            output_sha256: sha256_hex(listing.as_bytes()),
            file_sha256,
            lattice,
            forward: normalize_engine_numbers(&forward),
            simulation: normalize_engine_numbers(&simulation),
        })
    }

    fn shell<'a>(document: &'a Value, file: &str, prefab_guid: &str) -> Result<&'a Value> {
        document["shells"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|shell| shell["prefab_guid"] == prefab_guid)
            .with_context(|| format!("{file} holds no samples for shell {prefab_guid}"))
    }

    fn samples<'a>(
        document: &'a Value,
        file: &str,
        prefab_guid: &str,
        key: &str,
    ) -> Result<&'a [Value]> {
        Ok(Self::shell(document, file, prefab_guid)?[key]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default())
    }

    /// The forward-angle samples of a shell, every coefficient.
    pub(crate) fn forward_samples(&self, prefab_guid: &str) -> Result<&[Value]> {
        Self::samples(
            &self.forward,
            FORWARD_ANGLES_FILE,
            prefab_guid,
            "forward_angle_samples",
        )
    }

    /// The altitude-difference samples of a shell.
    pub(crate) fn altitude_samples(&self, prefab_guid: &str) -> Result<&[Value]> {
        Self::samples(
            &self.forward,
            FORWARD_ANGLES_FILE,
            prefab_guid,
            "altitude_difference_samples",
        )
    }

    /// The simulation samples of a shell.
    pub(crate) fn simulation_samples(&self, prefab_guid: &str) -> Result<&[Value]> {
        Self::samples(&self.simulation, SIMULATION_FILE, prefab_guid, "samples")
    }
}
