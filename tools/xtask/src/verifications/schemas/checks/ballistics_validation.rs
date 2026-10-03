//! The ballistics section of `schema validate`: the committed game ballistics catalog and its
//! calibration bundle against their schemas and against each other.
//!
//! **Role:** Validates `contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json` against
//! `ballistics-catalog.schema.json` and `contracts/fixtures/ballistics/vanilla_mortars.v1/calibration.json`
//! against `ballistics-calibration.schema.json`, then checks the pair's provenance and coverage:
//! the same catalog id, version, game build and export generation; the bundle's `catalog_sha256`
//! equal to the SHA-256 of the catalog's bytes; the catalog's gravity equal to the oracle's; every
//! shell charge covered by a native table and a simulation sample; and the catalog's own
//! referential rules that a JSON Schema cannot state.
//!
//! **Position:** Called by `contract_validation::validate_all`; reads the schemas through the
//! suite's `schema` loader and prints `PASS`, `FAIL` or `NOT RUN` lines in the suite's format. The
//! flight-model tolerances are not checked here: the `ballistics_calibration` evaluator owns them.
//!
//! **Signals & state:** none; each call reads the two documents from disk.
//!
//! **Invariants:** Both documents absent is [`BallisticsOutcome::NotRun`], printed as `NOT RUN` and
//! never as `PASS`. One document without the other is a failure. Every schema compile or read
//! error aborts the suite rather than passing.
use super::*;
use content_digest::sha256_hex;

/// Committed catalog, relative to `contracts/catalogs/`.
const CATALOG_RELATIVE_PATH: &str = "ballistics/vanilla_mortars.v1.catalog.json";
/// Committed calibration bundle, relative to `contracts/fixtures/`.
const CALIBRATION_RELATIVE_PATH: &str = "ballistics/vanilla_mortars.v1/calibration.json";
/// Tolerance for coefficient and gravity equality between two documents that copy one value.
const COPIED_VALUE_TOLERANCE: f64 = 1e-9;

/// Whether the ballistics section examined anything.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum BallisticsOutcome {
    /// Both documents were read, schema-checked and cross-checked (failures counted separately).
    Checked,
    /// Neither document exists yet; the reason names both paths.
    NotRun(String),
}

/// The catalog and calibration paths under the checkout `root`.
pub(super) fn ballistics_document_paths(root: &Path) -> (PathBuf, PathBuf) {
    (
        contract_catalogs_dir(root).join(CATALOG_RELATIVE_PATH),
        developer_tools::repository_layout::contract_fixtures_dir(root)
            .join(CALIBRATION_RELATIVE_PATH),
    )
}

/// Runs the ballistics section over the documents under `root`.
pub(super) fn validate(
    root: &Path,
    schema: &dyn Fn(&str) -> Result<Value>,
    compile: &dyn Fn(&Value) -> Result<jsonschema::Validator>,
    check: &dyn Fn(&str, &jsonschema::Validator, &Value),
    failures: &std::cell::Cell<usize>,
) -> Result<BallisticsOutcome> {
    println!("Ballistics catalog and calibration:");
    let v_catalog = compile(&schema("ballistics-catalog.schema.json")?)?;
    let v_calibration = compile(&schema("ballistics-calibration.schema.json")?)?;
    compile(&schema("fire-mission.schema.json")?)?;
    println!("  PASS  ballistics-catalog, ballistics-calibration and fire-mission schemas compile");

    let (catalog_path, calibration_path) = ballistics_document_paths(root);
    match (catalog_path.exists(), calibration_path.exists()) {
        (false, false) => {
            let reason = format!(
                "{} and {} are absent",
                catalog_path.display(),
                calibration_path.display()
            );
            println!("  NOT RUN  ballistics documents: {reason}");
            return Ok(BallisticsOutcome::NotRun(reason));
        }
        (true, false) | (false, true) => {
            failures.set(failures.get() + 1);
            let (present, absent) = if catalog_path.exists() {
                (&catalog_path, &calibration_path)
            } else {
                (&calibration_path, &catalog_path)
            };
            println!(
                "  FAIL  {} is committed without {}",
                present.display(),
                absent.display()
            );
            return Ok(BallisticsOutcome::Checked);
        }
        (true, true) => {}
    }

    let catalog_bytes =
        fs::read(&catalog_path).with_context(|| format!("read {}", catalog_path.display()))?;
    let catalog: Value = serde_json::from_slice(&catalog_bytes)
        .with_context(|| format!("parse {}", catalog_path.display()))?;
    let bundle = read_json(&calibration_path)?;
    check(CATALOG_RELATIVE_PATH, &v_catalog, &catalog);
    check(CALIBRATION_RELATIVE_PATH, &v_calibration, &bundle);

    let problems = provenance_failures(&catalog_bytes, &catalog, &bundle);
    if problems.is_empty() {
        println!("  PASS  catalog and calibration provenance and coverage");
    } else {
        failures.set(failures.get() + 1);
        println!("  FAIL  catalog and calibration provenance and coverage");
        for problem in &problems {
            println!("        {problem}");
        }
    }
    Ok(BallisticsOutcome::Checked)
}

fn copied_value_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= COPIED_VALUE_TOLERANCE * left.abs().max(1.0)
}

fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map(Vec::as_slice).unwrap_or(&[])
}

/// Every provenance, coverage and referential problem of a catalog and its bundle. Empty means
/// the pair is consistent; tolerance against the flight model is out of scope.
pub(super) fn provenance_failures(
    catalog_bytes: &[u8],
    catalog: &Value,
    bundle: &Value,
) -> Vec<String> {
    let mut problems = Vec::new();
    for key in [
        "catalog_id",
        "catalog_version",
        "game_build",
        "export_generation_id",
    ] {
        if catalog[key].is_null() || catalog[key] != bundle[key] {
            problems.push(format!(
                "{key}: catalog {} but calibration {}",
                catalog[key], bundle[key]
            ));
        }
    }
    let actual_sha256 = sha256_hex(catalog_bytes);
    if bundle["catalog_sha256"].as_str() != Some(actual_sha256.as_str()) {
        problems.push(format!(
            "catalog_sha256: calibration {} but the catalog bytes hash to {actual_sha256}",
            bundle["catalog_sha256"]
        ));
    }
    match (
        catalog["gravity_m_s2"].as_f64(),
        bundle["oracle_run"]["gravity_reported_m_s2"].as_f64(),
    ) {
        (Some(catalog_gravity), Some(oracle_gravity))
            if copied_value_equal(catalog_gravity, oracle_gravity) => {}
        (catalog_gravity, oracle_gravity) => problems.push(format!(
            "gravity: catalog {catalog_gravity:?} but oracle run {oracle_gravity:?}"
        )),
    }
    catalog_reference_failures(catalog, &mut problems);
    resource_hash_failures(catalog, bundle, &mut problems);
    coverage_failures(catalog, bundle, &mut problems);
    problems
}

/// Unique weapon and shell ids, weapon shell references, elevation limits, charges and fuzes.
fn catalog_reference_failures(catalog: &Value, problems: &mut Vec<String>) {
    let mut shell_ids = BTreeSet::new();
    for shell in array(catalog, "shells") {
        let shell_id = shell["shell_id"].as_str().unwrap_or("?");
        if !shell_ids.insert(shell_id) {
            problems.push(format!("shell {shell_id}: declared twice"));
        }
        let charges = array(shell, "charges");
        let mut rings = BTreeSet::new();
        for charge in charges {
            if !rings.insert(charge["rings"].as_i64().unwrap_or(-1)) {
                problems.push(format!(
                    "shell {shell_id}: charge rings {} declared twice",
                    charge["rings"]
                ));
            }
        }
        let defaults = charges
            .iter()
            .filter(|charge| charge["is_default"] == Value::Bool(true))
            .count();
        if defaults != 1 {
            problems.push(format!(
                "shell {shell_id}: {defaults} default charges, expected exactly 1"
            ));
        }
        let fuze = &shell["time_fuze"];
        if !fuze.is_null() {
            let (min, default, max) = (
                fuze["min_s"].as_f64(),
                fuze["default_s"].as_f64(),
                fuze["max_s"].as_f64(),
            );
            let ordered = matches!((min, default, max), (Some(min), Some(default), Some(max)) if min < max && min <= default && default <= max);
            if !ordered {
                problems.push(format!(
                    "shell {shell_id}: time fuze needs min_s < max_s with default_s between"
                ));
            }
        }
    }
    let mut weapon_ids = BTreeSet::new();
    for weapon in array(catalog, "weapons") {
        let weapon_id = weapon["weapon_id"].as_str().unwrap_or("?");
        if !weapon_ids.insert(weapon_id) {
            problems.push(format!("weapon {weapon_id}: declared twice"));
        }
        let (low, high) = (
            weapon["elevation_min_deg"].as_f64(),
            weapon["elevation_max_deg"].as_f64(),
        );
        if !matches!((low, high), (Some(low), Some(high)) if low < high) {
            problems.push(format!(
                "weapon {weapon_id}: elevation_min_deg must be below elevation_max_deg"
            ));
        }
        for shell_id in array(weapon, "shell_ids").iter().filter_map(Value::as_str) {
            if !shell_ids.contains(shell_id) {
                problems.push(format!(
                    "weapon {weapon_id}: fires unknown shell {shell_id}"
                ));
            }
        }
    }
}

/// A resource GUID declared in both documents carries one SHA-256, and no document repeats a GUID
/// with two hashes.
fn resource_hash_failures(catalog: &Value, bundle: &Value, problems: &mut Vec<String>) {
    let mut hash_by_guid: BTreeMap<&str, &str> = BTreeMap::new();
    for (document, resources) in [
        ("catalog", array(catalog, "resources")),
        ("calibration", array(bundle, "resources")),
    ] {
        for resource in resources {
            let (Some(guid), Some(sha256)) =
                (resource["guid"].as_str(), resource["sha256"].as_str())
            else {
                continue;
            };
            match hash_by_guid.insert(guid, sha256) {
                Some(previous) if previous != sha256 => problems.push(format!(
                    "resource {guid}: {document} sha256 {sha256} differs from {previous}"
                )),
                _ => {}
            }
        }
    }
}

/// Every (shell, charge) has a native table and a simulation sample at its coefficient, and every
/// table and sample names a catalog shell.
fn coverage_failures(catalog: &Value, bundle: &Value, problems: &mut Vec<String>) {
    let shell_ids: BTreeSet<&str> = array(catalog, "shells")
        .iter()
        .filter_map(|shell| shell["shell_id"].as_str())
        .collect();
    let native_tables = array(bundle, "native_tables");
    let simulation_samples: Vec<&Value> = array(bundle, "oracle_samples")
        .iter()
        .filter(|sample| sample["kind"] == "simulation")
        .collect();
    let covers = |entry: &Value, shell_id: &str, coefficient: f64| {
        entry["shell_id"].as_str() == Some(shell_id)
            && entry["init_speed_coef"]
                .as_f64()
                .is_some_and(|value| copied_value_equal(value, coefficient))
    };
    for shell in array(catalog, "shells") {
        let shell_id = shell["shell_id"].as_str().unwrap_or("?");
        for charge in array(shell, "charges") {
            let Some(coefficient) = charge["init_speed_coef"].as_f64() else {
                continue;
            };
            let rings = &charge["rings"];
            if !native_tables
                .iter()
                .any(|table| covers(table, shell_id, coefficient))
            {
                problems.push(format!(
                    "shell {shell_id} charge {rings}: no native table at coefficient {coefficient}"
                ));
            }
            if !simulation_samples
                .iter()
                .any(|sample| covers(sample, shell_id, coefficient))
            {
                problems.push(format!("shell {shell_id} charge {rings}: no simulation sample at coefficient {coefficient}"));
            }
        }
    }
    for (section, entries) in [
        ("native_tables", native_tables),
        ("wind_tables", array(bundle, "wind_tables")),
        ("oracle_samples", array(bundle, "oracle_samples")),
    ] {
        for (index, entry) in entries.iter().enumerate() {
            let shell_id = entry["shell_id"].as_str().unwrap_or("?");
            if !shell_ids.contains(shell_id) {
                problems.push(format!("{section}[{index}]: unknown shell {shell_id}"));
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/checks/ballistics_validation_tests.rs"]
mod tests;
