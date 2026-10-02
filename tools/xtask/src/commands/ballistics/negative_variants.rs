//! Calibration bundles that must be refused, each derived from the good bundle by one defect.
//!
//! **Role:** Produces the four `negative/` bundles the calibration evaluator must reject, each
//! failing for exactly one reason: a native row skewed by +5 m, a wrong game build, a shell with
//! no tables or samples, and a catalog hash that does not match the catalog bytes.
//!
//! **Position:** Called by the trim after the good bundle is assembled; the trim writes the results
//! next to it under `negative/`.
//!
//! **Signals & state:** none; pure functions over clones of the good bundle.
//!
//! **Invariants:** Every variant changes one thing and keeps every other field of the good bundle,
//! so a refusal names only its own defect. The skewed row is the maximum-range row of the first
//! shell's default charge: no elevation reaches 5 m beyond the maximum range, so the defect fails
//! at any angular tolerance.
use super::contract_documents::{BallisticsCatalog, CalibrationBundle};
use anyhow::{Context, Result};
use content_digest::sha256_hex;

/// Metres added to the skewed row's range.
pub(crate) const SKEWED_ROW_OFFSET_M: f64 = 5.0;
/// The game build the wrong-build variant claims.
const WRONG_GAME_BUILD: &str = "0.0.0.0";

/// One refused bundle: its file name, the defect it carries and the bundle.
pub(crate) struct NegativeVariant {
    pub(crate) file_name: &'static str,
    pub(crate) defect: String,
    pub(crate) bundle: CalibrationBundle,
}

/// The four refused bundles derived from `bundle`, which calibrates `catalog`.
pub(crate) fn negative_variants(
    catalog: &BallisticsCatalog,
    bundle: &CalibrationBundle,
) -> Result<Vec<NegativeVariant>> {
    let first_shell = catalog
        .shells
        .first()
        .context("the catalog has no shells")?;
    let default_coefficient = first_shell
        .charges
        .iter()
        .find(|charge| charge.is_default)
        .map(|charge| charge.init_speed_coef)
        .context("the first shell has no default charge")?;
    let mut skewed = bundle.clone();
    let table = skewed
        .native_tables
        .iter_mut()
        .find(|table| {
            table.shell_id == first_shell.shell_id && table.init_speed_coef == default_coefficient
        })
        .context("the first shell's default charge has no native table")?;
    let row = table
        .rows
        .last_mut()
        .context("the default charge table has no rows")?;
    row.range_m += SKEWED_ROW_OFFSET_M;
    let skewed_defect = format!(
        "shell {} coefficient {default_coefficient} row {} (maximum range) moved from {} m to {} m",
        first_shell.shell_id,
        row.lattice_index,
        row.range_m - SKEWED_ROW_OFFSET_M,
        row.range_m
    );

    let mut wrong_build = bundle.clone();
    wrong_build.game_build = WRONG_GAME_BUILD.to_owned();

    let removed = catalog.shells.last().context("the catalog has no shells")?;
    let mut missing = bundle.clone();
    missing
        .native_tables
        .retain(|table| table.shell_id != removed.shell_id);
    missing
        .wind_tables
        .retain(|table| table.shell_id != removed.shell_id);
    missing
        .oracle_samples
        .retain(|sample| sample.shell_id != removed.shell_id);

    let mut stale = bundle.clone();
    stale.catalog_sha256 = sha256_hex(&serde_json::to_vec(catalog)?);

    Ok(vec![
        NegativeVariant {
            file_name: "skewed_native_row.calibration.json",
            defect: skewed_defect,
            bundle: skewed,
        },
        NegativeVariant {
            file_name: "wrong_game_build.calibration.json",
            defect: format!(
                "game_build {WRONG_GAME_BUILD} instead of {}",
                bundle.game_build
            ),
            bundle: wrong_build,
        },
        NegativeVariant {
            file_name: "missing_shell.calibration.json",
            defect: format!(
                "no native table, wind table or oracle sample for shell {}",
                removed.shell_id
            ),
            bundle: missing,
        },
        NegativeVariant {
            file_name: "stale_catalog_sha.calibration.json",
            defect:
                "catalog_sha256 of the catalog serialized on one line, not of the committed bytes"
                    .to_owned(),
            bundle: stale,
        },
    ])
}
