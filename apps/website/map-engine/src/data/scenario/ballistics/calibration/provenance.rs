//! Provenance and coverage of a calibration bundle against the catalog it pins.
//!
//! **Role:** the checks judged before any flight: the bundle names the catalog's identity, game
//! build, export generation, gravity and exact bytes; no resource carries two digests; every
//! (shell, charge) of the catalog has a native table and a simulation sample; every table and
//! sample names a catalog shell.
//!
//! **Position:** `ballistics/calibration`; [`super::evaluate`] runs [`check_provenance`] first
//! and appends the flight cases after it.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:**
//! - Provenance checks are not cases: they add failures, never to the case count.
//! - `catalog_sha256` is compared with the SHA-256 of the catalog bytes as uploaded or committed
//!   ([`super::PinnedCatalog::sha256`]), never of a re-serialisation.
//! - A charge is matched by its muzzle speed coefficient within 1e-9.

use std::collections::BTreeMap;

use super::charge_flight::same_coefficient;
use super::oracle_samples::OracleSampleKind;
use super::report::{CalibrationReport, FailureKind};
use super::{CalibrationBundle, PinnedCatalog};

/// Largest difference between the catalog's gravity and the oracle's reported gravity.
const GRAVITY_MATCH_TOLERANCE_M_S2: f64 = 1e-9;

/// Every provenance and coverage failure of `bundle` against `pinned`.
pub fn check_provenance(pinned: &PinnedCatalog, bundle: &CalibrationBundle) -> CalibrationReport {
    let catalog = &pinned.catalog;
    let mut report = CalibrationReport::default();
    if bundle.catalog_id != catalog.catalog_id || bundle.catalog_version != catalog.catalog_version
    {
        report.fail(
            FailureKind::CatalogIdentityMismatch,
            "provenance/catalog".to_owned(),
            format!(
                "bundle calibrates {} v{} but the catalog is {} v{}",
                bundle.catalog_id,
                bundle.catalog_version,
                catalog.catalog_id,
                catalog.catalog_version
            ),
        );
    }
    if bundle.game_build != catalog.game_build {
        report.fail(
            FailureKind::GameBuildMismatch,
            "provenance/game_build".to_owned(),
            format!(
                "bundle game build {} but catalog game build {}",
                bundle.game_build, catalog.game_build
            ),
        );
    }
    if bundle.export_generation_id != catalog.export_generation_id {
        report.fail(
            FailureKind::ExportGenerationMismatch,
            "provenance/export_generation_id".to_owned(),
            format!(
                "bundle export generation {} but catalog export generation {}",
                bundle.export_generation_id, catalog.export_generation_id
            ),
        );
    }
    if bundle.catalog_sha256 != pinned.sha256 {
        report.fail(
            FailureKind::CatalogShaMismatch,
            "provenance/catalog_sha256".to_owned(),
            format!(
                "bundle pins catalog sha256 {} but the catalog bytes hash to {}",
                bundle.catalog_sha256, pinned.sha256
            ),
        );
    }
    let reported_gravity = bundle.oracle_run.gravity_reported_m_s2;
    let gravity_matches =
        (catalog.gravity_m_s2 - reported_gravity).abs() <= GRAVITY_MATCH_TOLERANCE_M_S2;
    if !gravity_matches {
        report.fail(
            FailureKind::GravityMismatch,
            "provenance/gravity".to_owned(),
            format!(
                "catalog gravity {} m/s² but the oracle run reported {reported_gravity} m/s²",
                catalog.gravity_m_s2
            ),
        );
    }
    check_resource_digests(pinned, bundle, &mut report);
    check_coverage(pinned, bundle, &mut report);
    report
}

/// A GUID declared in either document, or in both, carries one SHA-256.
fn check_resource_digests(
    pinned: &PinnedCatalog,
    bundle: &CalibrationBundle,
    report: &mut CalibrationReport,
) {
    let mut digest_by_guid: BTreeMap<&str, &str> = BTreeMap::new();
    let declared = [
        ("catalog", &pinned.catalog.resources),
        ("calibration", &bundle.resources),
    ];
    for (document, resources) in declared {
        for resource in resources {
            let previous = digest_by_guid.insert(&resource.guid, &resource.sha256);
            if let Some(previous) = previous.filter(|previous| *previous != resource.sha256) {
                report.fail(
                    FailureKind::ResourceShaMismatch,
                    format!("provenance/resource/{}", resource.guid),
                    format!(
                        "{document} declares sha256 {} but {previous} is declared for the same GUID",
                        resource.sha256
                    ),
                );
            }
        }
    }
}

/// Every (shell, charge) has a native table and a simulation sample; every table and sample
/// names a catalog shell.
fn check_coverage(
    pinned: &PinnedCatalog,
    bundle: &CalibrationBundle,
    report: &mut CalibrationReport,
) {
    for shell in &pinned.catalog.shells {
        for charge in &shell.charges {
            let covers = |shell_id: &str, coefficient: f64| {
                shell_id == shell.shell_id && same_coefficient(coefficient, charge.init_speed_coef)
            };
            let case_id = format!("coverage/{}/{}", shell.shell_id, charge.rings);
            if !bundle
                .native_tables
                .iter()
                .any(|table| covers(&table.shell_id, table.init_speed_coef))
            {
                report.fail(
                    FailureKind::MissingNativeTable,
                    case_id.clone(),
                    format!(
                        "no native table for shell {} at coefficient {}",
                        shell.shell_id, charge.init_speed_coef
                    ),
                );
            }
            if !bundle.oracle_samples.iter().any(|sample| {
                sample.kind == OracleSampleKind::Simulation
                    && covers(&sample.shell_id, sample.init_speed_coef)
            }) {
                report.fail(
                    FailureKind::MissingSimulationSample,
                    case_id,
                    format!(
                        "no simulation sample for shell {} at coefficient {}",
                        shell.shell_id, charge.init_speed_coef
                    ),
                );
            }
        }
    }
    let named_shells = bundle
        .native_tables
        .iter()
        .map(|table| table.shell_id.as_str())
        .chain(
            bundle
                .wind_tables
                .iter()
                .map(|table| table.shell_id.as_str()),
        )
        .chain(
            bundle
                .oracle_samples
                .iter()
                .map(|sample| sample.shell_id.as_str()),
        );
    let mut reported: Vec<&str> = Vec::new();
    for shell_id in named_shells {
        let known = pinned
            .catalog
            .shells
            .iter()
            .any(|shell| shell.shell_id == shell_id);
        if !known && !reported.contains(&shell_id) {
            reported.push(shell_id);
            report.fail(
                FailureKind::UnknownShell,
                format!("coverage/{shell_id}"),
                format!(
                    "the bundle calibrates shell {shell_id}, which the catalog does not declare"
                ),
            );
        }
    }
}
