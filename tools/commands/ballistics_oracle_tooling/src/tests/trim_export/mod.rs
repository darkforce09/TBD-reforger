//! Trim tests over a synthetic export: byte determinism, schema validity, the elevation evidence,
//! and the refusals of an unmatched row and a tampered oracle output.
use super::*;
use repository_layout::definition_path;
use serde_json::Value;
use tool_test_support::test_repo_root;

mod synthetic_export;
use synthetic_export::{GENERATION_ID, SELECTION, synthetic_export};

/// Every file under `directory` with its bytes, in path order.
fn files_under(directory: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files: Vec<(PathBuf, Vec<u8>)> = walkdir::WalkDir::new(directory)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| {
            let path = entry.path().to_path_buf();
            let bytes = fs::read(&path).expect("read output");
            (
                path.strip_prefix(directory)
                    .expect("relative")
                    .to_path_buf(),
                bytes,
            )
        })
        .collect();
    files.sort();
    files
}

fn schema_errors(schema_file: &str, document: &Value) -> Vec<String> {
    let root = test_repo_root();
    let schema: Value = serde_json::from_slice(
        &fs::read(definition_path(&root, schema_file)).expect("read schema"),
    )
    .expect("parse schema");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    validator
        .iter_errors(document)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect()
}

fn read_value(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read document")).expect("parse document")
}

#[test]
fn ballistics_trim_export_two_runs_write_identical_bytes() {
    let (root, locations) = synthetic_export("determinism", None);
    trim_export(&locations, GENERATION_ID, &SELECTION).expect("first trim");
    let first = files_under(&root.join("out"));
    trim_export(&locations, GENERATION_ID, &SELECTION).expect("second trim");
    let second = files_under(&root.join("out"));
    assert_eq!(
        first.len(),
        8,
        "catalog, bundle, README, negative README and four refused bundles"
    );
    assert_eq!(
        first, second,
        "a second trim over the same inputs wrote different bytes"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn ballistics_trim_export_documents_validate_and_pin_each_other() {
    let (root, locations) = synthetic_export("documents", None);
    let report = trim_export(&locations, GENERATION_ID, &SELECTION).expect("trim");
    let catalog_bytes = fs::read(&locations.catalog_path).expect("read catalog");
    let catalog: Value = serde_json::from_slice(&catalog_bytes).expect("parse catalog");
    let bundle = read_value(&locations.fixture_dir.join("calibration.json"));
    assert_eq!(
        schema_errors("ballistics-catalog.schema.json", &catalog),
        Vec::<String>::new()
    );
    assert_eq!(
        schema_errors("ballistics-calibration.schema.json", &bundle),
        Vec::<String>::new()
    );
    assert_eq!(
        bundle["catalog_sha256"],
        sha256_hex(&catalog_bytes).as_str()
    );
    assert_eq!(catalog["gravity_m_s2"], 9.81);
    assert_eq!(bundle["oracle_run"]["gravity_reported_m_s2"], 9.81);
    assert_eq!(catalog["shells"][0]["charges"][1]["init_speed_coef"], 1.5);
    assert_eq!(
        catalog["shells"][0]["standard_dispersion_m"], 20.0,
        "the default charge's range card page"
    );
    let coefficients: Vec<f64> = bundle["native_tables"]
        .as_array()
        .expect("native tables")
        .iter()
        .filter_map(|table| table["init_speed_coef"].as_f64())
        .collect();
    assert_eq!(
        coefficients,
        vec![1.0, 1.5, 1.0, 1.5],
        "only charge coefficients are kept"
    );
    assert!(
        bundle["oracle_samples"]
            .as_array()
            .expect("samples")
            .iter()
            .all(|sample| sample["kind"] != "forward_angle"
                || sample["outputs"]["time_of_flight_s"].as_f64() >= Some(0.0)),
        "a sentinel forward sample reached the bundle"
    );
    for (file, _, _) in &report.negatives {
        let negative = read_value(&locations.fixture_dir.join("negative").join(file));
        assert_eq!(
            schema_errors("ballistics-calibration.schema.json", &negative),
            Vec::<String>::new(),
            "{file}"
        );
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn ballistics_trim_export_matches_every_row_by_a_forward_sample_or_a_lattice_end() {
    let (root, locations) = synthetic_export("evidence", None);
    let report = trim_export(&locations, GENERATION_ID, &SELECTION).expect("trim");
    let bundle = read_value(&locations.fixture_dir.join("calibration.json"));
    let elevations: Vec<(u64, f64)> = bundle["native_tables"][0]["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| {
            (
                row["lattice_index"].as_u64().expect("index"),
                row["elevation_mils_6400"].as_f64().expect("elevation"),
            )
        })
        .collect();
    assert_eq!(
        elevations,
        vec![
            (0, 1600.0),
            (1, 1587.5),
            (2, 1550.0),
            (3, 1537.5),
            (4, 1525.0),
            (5, 1500.0),
            (6, 1400.0),
            (7, 1200.0),
            (8, 1000.0),
            (9, 800.0)
        ],
        "every native row is in the bundle at its own elevation"
    );
    let counts: Vec<(ElevationEvidence, usize)> = report
        .evidence_counts
        .iter()
        .map(|(evidence, count)| (*evidence, *count))
        .collect();
    assert_eq!(
        counts,
        vec![
            (ElevationEvidence::ForwardSample, 32),
            (ElevationEvidence::LatticeEnd, 8)
        ],
        "four tables of ten rows: both lattice ends of each, every other row one forward sample"
    );
    let readme = fs::read_to_string(locations.fixture_dir.join("README.md")).expect("README");
    assert!(
        readme.contains(&report.catalog_sha256)
            && readme.contains("on a 12.5-mil elevation lattice")
            && readme.contains("- Every native row is matched; none is interpolated or left out."),
        "README lacks the provenance:\n{readme}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn ballistics_trim_export_refuses_an_unmatched_row() {
    let (root, locations) = synthetic_export("unmatched", Some((1.0, 6, 5.0)));
    let error = trim_export(&locations, GENERATION_ID, &SELECTION)
        .err()
        .expect("a row skewed by 5 m must be refused");
    let message = format!("{error:#}");
    assert!(
        message.contains("unmatched row") && message.contains("row 6"),
        "{message}"
    );
    assert!(
        !locations.catalog_path.exists(),
        "a refused trim wrote the catalog"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn ballistics_trim_export_refuses_oracle_output_that_differs_from_its_sidecar() {
    let (root, locations) = synthetic_export("tampered-oracle", None);
    let simulation = locations.oracle_dir.join("simulation.json");
    let text = fs::read_to_string(&simulation).expect("read simulation");
    assert!(
        text.contains("400.0"),
        "the synthetic simulation holds a 400.0 m sample"
    );
    // Same length, different bytes: only the SHA-256 comparison can see it.
    fs::write(&simulation, text.replacen("400.0", "401.0", 1)).expect("tamper simulation");
    let error = trim_export(&locations, GENERATION_ID, &SELECTION)
        .err()
        .expect("a tampered oracle output must be refused");
    assert!(
        format!("{error:#}").contains("does not match its sidecar"),
        "{error:#}"
    );
    let _ = fs::remove_dir_all(root);
}
