//! `cargo xtask ballistics trim-export`: the vanilla mortar catalog and its calibration bundle.
//!
//! **Role:** Reads one gameplay export generation and the ballistics oracle's output for it, and
//! writes the ballistics catalog, the calibration bundle, the four refused bundles and the
//! provenance READMEs, byte for byte the same on every run over the same inputs.
//!
//! **Position:** The producer of `contracts/catalogs/ballistics/` and
//! `contracts/fixtures/ballistics/vanilla_mortars.v1/`; `cargo xtask schema validate` checks
//! what it writes, the map engine's calibration tests load it, and administrators upload the pair
//! through `POST /api/v1/ballistics-catalogs`.
//!
//! **Signals & state:** none beyond the files it writes; every document is assembled in memory
//! before the first file is written.
//!
//! **Invariants:** Every input byte is hash-verified (export manifest, oracle sidecars). The bundle
//! carries the SHA-256 of the exact catalog bytes written. Gravity is the oracle's. Every native
//! row of every kept table is in the bundle; a row whose elevation no evidence fixes refuses the
//! trim.
use super::calibration_assembly::shell_calibration;
use super::catalog_extraction::{WeaponSelection, extract_catalog};
use super::contract_documents::{
    BallisticsCatalog, CalibrationBundle, OracleRun, ResourceRecord, document_bytes,
};
use super::game_tables::{native_tables, wind_tables};
use super::gameplay_export::GameplayExport;
use super::negative_variants::negative_variants;
use super::oracle_output::OracleOutput;
use super::provenance_readme::{fixture_readme, negative_readme};
use super::record_per_line_json::record_per_line_bytes;
use super::row_elevations::ElevationEvidence;
use anyhow::{Context, Result, bail};
use content_digest::sha256_hex;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Catalog id of the vanilla mortar catalog.
pub(crate) const CATALOG_ID: &str = "vanilla_mortars";
/// Catalog version the trim writes.
pub(crate) const CATALOG_VERSION: u32 = 1;
const CATALOG_TITLE: &str = "Vanilla Arma Reforger mortars";
/// Gameplay export generations, relative to the checkout root.
const GAMEPLAY_GENERATIONS_DIR: &str = "assets/equipment/gameplay/generations";
/// Ballistics oracle output, one folder per generation, relative to the checkout root.
const ORACLE_OUTPUT_DIR: &str = "assets/scratch/ballistics_oracle";

/// Where the trim reads and writes.
pub(crate) struct TrimLocations {
    pub(crate) export_dir: PathBuf,
    pub(crate) oracle_dir: PathBuf,
    pub(crate) catalog_path: PathBuf,
    pub(crate) fixture_dir: PathBuf,
}

impl TrimLocations {
    /// The checkout's export and oracle folders for `generation_id` and its contract folders;
    /// `oracle_dir` overrides the oracle folder.
    pub(crate) fn in_checkout(
        root: &Path,
        generation_id: &str,
        oracle_dir: Option<PathBuf>,
    ) -> Self {
        let name = format!("{CATALOG_ID}.v{CATALOG_VERSION}");
        Self {
            export_dir: root
                .join(GAMEPLAY_GENERATIONS_DIR)
                .join(generation_id)
                .join("export"),
            oracle_dir: oracle_dir
                .unwrap_or_else(|| root.join(ORACLE_OUTPUT_DIR).join(generation_id)),
            catalog_path: developer_tools::repository_layout::contract_catalogs_dir(root)
                .join("ballistics")
                .join(format!("{name}.catalog.json")),
            fixture_dir: developer_tools::repository_layout::contract_fixtures_dir(root)
                .join("ballistics")
                .join(name),
        }
    }
}

/// What one trim wrote, for the README and the command's summary.
pub(crate) struct TrimReport {
    pub(crate) catalog: BallisticsCatalog,
    pub(crate) bundle_resources: Vec<ResourceRecord>,
    pub(crate) oracle: OracleSummary,
    pub(crate) catalog_sha256: String,
    pub(crate) calibration_sha256: String,
    /// `(file name, defect, sha256)` of each refused bundle.
    pub(crate) negatives: Vec<(String, String, String)>,
    pub(crate) native_table_count: usize,
    pub(crate) wind_table_count: usize,
    pub(crate) evidence_counts: BTreeMap<ElevationEvidence, usize>,
    pub(crate) sample_counts: BTreeMap<String, usize>,
}

/// The oracle run facts the README records.
pub(crate) struct OracleSummary {
    pub(crate) plugin_revision: String,
    pub(crate) run_at: String,
    pub(crate) gravity_raw_m_s2: f64,
    pub(crate) gravity_source: String,
    pub(crate) output_sha256: String,
    pub(crate) file_sha256: Vec<(String, String)>,
    /// The forward-angle lattice step, in 6400-mil units.
    pub(crate) lattice_step_mils: f64,
}

/// Trims generation `generation_id` of the export at `locations` into the contract documents.
pub(crate) fn trim_export(
    locations: &TrimLocations,
    generation_id: &str,
    selections: &[WeaponSelection],
) -> Result<TrimReport> {
    if generation_id.len() != 16
        || !generation_id
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'A'..=b'F'))
    {
        bail!("generation id {generation_id:?} is not 16 uppercase hexadecimal digits");
    }
    let export = GameplayExport::open(&locations.export_dir, generation_id)?;
    let oracle = OracleOutput::open(&locations.oracle_dir, generation_id, export.game_build())?;
    let extracted = extract_catalog(&export, selections)?;
    let catalog = BallisticsCatalog {
        schema_version: 1,
        catalog_id: CATALOG_ID.to_owned(),
        catalog_version: CATALOG_VERSION,
        title: CATALOG_TITLE.to_owned(),
        game_build: export.game_build().to_owned(),
        export_generation_id: export.generation_id().to_owned(),
        gravity_m_s2: oracle.gravity_m_s2,
        gravity_source: "oracle".to_owned(),
        resources: extracted.resources.clone(),
        weapons: extracted.weapons.clone(),
        shells: extracted
            .shells
            .iter()
            .map(|shell| shell.shell.clone())
            .collect(),
    };
    let catalog_bytes = document_bytes(&catalog)?;
    let catalog_sha256 = sha256_hex(&catalog_bytes);

    let mut bundle = CalibrationBundle {
        schema_version: 1,
        catalog_id: CATALOG_ID.to_owned(),
        catalog_version: CATALOG_VERSION,
        catalog_sha256: catalog_sha256.clone(),
        game_build: export.game_build().to_owned(),
        export_generation_id: export.generation_id().to_owned(),
        resources: Vec::new(),
        oracle_run: OracleRun {
            plugin_revision: oracle.plugin_revision.clone(),
            gravity_reported_m_s2: oracle.gravity_m_s2,
            run_at: oracle.run_at.clone(),
            output_sha256: oracle.output_sha256.clone(),
        },
        native_tables: Vec::new(),
        wind_tables: Vec::new(),
        oracle_samples: Vec::new(),
    };
    let mut evidence_counts = BTreeMap::new();
    for shell in &extracted.shells {
        let table_resource = export.resource(&shell.ballistic_table_guid)?;
        let wind_resource = export.resource(&shell.wind_table_guid)?;
        let calibration = shell_calibration(
            shell,
            &native_tables(&table_resource)?,
            &wind_tables(&wind_resource)?,
            &oracle,
        )?;
        for resource in [&table_resource, &wind_resource] {
            bundle.resources.push(ResourceRecord {
                guid: resource.guid.clone(),
                resource_name: resource.resource_name.clone(),
                sha256: resource.sha256.clone(),
            });
        }
        bundle.native_tables.extend(calibration.native_tables);
        bundle.wind_tables.extend(calibration.wind_tables);
        bundle.oracle_samples.extend(calibration.samples);
        for (evidence, count) in calibration.evidence_counts {
            *evidence_counts.entry(evidence).or_default() += count;
        }
    }
    bundle.resources.sort();
    bundle.resources.dedup();
    let calibration_bytes = record_per_line_bytes(&bundle)?;
    let variants = negative_variants(&catalog, &bundle)?;
    let mut negative_files = Vec::new();
    for variant in &variants {
        negative_files.push((
            variant.file_name,
            variant.defect.clone(),
            record_per_line_bytes(&variant.bundle)?,
        ));
    }

    let mut sample_counts = BTreeMap::new();
    for sample in &bundle.oracle_samples {
        *sample_counts.entry(sample.kind.clone()).or_default() += 1;
    }
    let report = TrimReport {
        catalog,
        bundle_resources: bundle.resources.clone(),
        oracle: OracleSummary {
            plugin_revision: oracle.plugin_revision.clone(),
            run_at: oracle.run_at.clone(),
            gravity_raw_m_s2: oracle.gravity_raw_m_s2,
            gravity_source: oracle.gravity_source.clone(),
            output_sha256: oracle.output_sha256.clone(),
            file_sha256: oracle.file_sha256.clone(),
            lattice_step_mils: oracle.lattice.step_mils,
        },
        catalog_sha256,
        calibration_sha256: sha256_hex(&calibration_bytes),
        negatives: negative_files
            .iter()
            .map(|(file, defect, bytes)| ((*file).to_owned(), defect.clone(), sha256_hex(bytes)))
            .collect(),
        native_table_count: bundle.native_tables.len(),
        wind_table_count: bundle.wind_tables.len(),
        evidence_counts,
        sample_counts,
    };

    let negative_dir = locations.fixture_dir.join("negative");
    for directory in [
        locations
            .catalog_path
            .parent()
            .context("catalog path has no folder")?,
        &negative_dir,
    ] {
        fs::create_dir_all(directory).with_context(|| format!("create {}", directory.display()))?;
    }
    let mut writes: Vec<(PathBuf, Vec<u8>)> = vec![
        (locations.catalog_path.clone(), catalog_bytes),
        (
            locations.fixture_dir.join("calibration.json"),
            calibration_bytes,
        ),
        (
            locations.fixture_dir.join("README.md"),
            fixture_readme(&report).into_bytes(),
        ),
        (
            negative_dir.join("README.md"),
            negative_readme(&report).into_bytes(),
        ),
    ];
    for (file, _, bytes) in negative_files {
        writes.push((negative_dir.join(file), bytes));
    }
    for (path, bytes) in writes {
        fs::write(&path, bytes).with_context(|| format!("write {}", path.display()))?;
    }
    Ok(report)
}

#[cfg(test)]
#[path = "tests/trim_export/mod.rs"]
mod tests;
