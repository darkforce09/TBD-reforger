//! Runs a `ballistics` command against the active checkout.
use super::catalog_extraction::VANILLA_MORTARS;
use super::cli::BallisticsCmd;
use super::trim_export::{TrimLocations, trim_export};
use crate::core::repository_root::find_repo_root;
use anyhow::Result;

/// Runs `cmd` and returns the process exit code.
pub(crate) fn run(cmd: BallisticsCmd) -> Result<u8> {
    match cmd {
        BallisticsCmd::TrimExport { generation, oracle } => {
            let root = find_repo_root()?;
            let locations = TrimLocations::in_checkout(&root, &generation, oracle);
            let report = trim_export(&locations, &generation, &VANILLA_MORTARS)?;
            println!("catalog      {}", locations.catalog_path.display());
            println!("  sha256     {}", report.catalog_sha256);
            println!(
                "calibration  {}",
                locations.fixture_dir.join("calibration.json").display()
            );
            println!("  sha256     {}", report.calibration_sha256);
            println!(
                "tables       {} native, {} wind",
                report.native_table_count, report.wind_table_count
            );
            for (evidence, count) in &report.evidence_counts {
                println!("  rows       {count} fixed by {evidence:?}");
            }
            for (kind, count) in &report.sample_counts {
                println!("samples      {count} {kind}");
            }
            println!(
                "gravity      {} m/s² (oracle reported {})",
                report.catalog.gravity_m_s2, report.oracle.gravity_raw_m_s2
            );
            Ok(0)
        }
    }
}
