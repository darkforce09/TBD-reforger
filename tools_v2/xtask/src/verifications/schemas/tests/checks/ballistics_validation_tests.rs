//! The ballistics section of `schema validate`: a consistent catalog and calibration pair passes
//! both schemas and the provenance checks, each provenance defect is named, and absent documents
//! are NOT RUN rather than a pass.

use std::cell::Cell;

use serde_json::{Value, json};

use super::{
    BallisticsOutcome, ballistics_document_paths, provenance_failures, read_json, repo_root,
    sha256_hex, validate,
};
use developer_tools::repository_layout::definition_path;

const SHA_OF_RESOURCE: &str = "0000000000000000000000000000000000000000000000000000000000000001";

fn sample_catalog() -> Value {
    json!({
        "schema_version": 1,
        "catalog_id": "test_mortars",
        "catalog_version": 1,
        "title": "Test mortars",
        "game_build": "1.8.0.13",
        "export_generation_id": "6A6F008DC5395616",
        "gravity_m_s2": 9.807,
        "gravity_source": "oracle",
        "resources": [
            {"guid": "0123456789ABCDEF", "resource_name": "{0123456789ABCDEF}Prefabs/Shell.et", "sha256": SHA_OF_RESOURCE}
        ],
        "weapons": [{
            "weapon_id": "m252",
            "display_name": "M252",
            "prefab_guid": "0123456789ABCDEE",
            "caliber_mm": 81,
            "mils_per_circle": 6400,
            "elevation_min_deg": 45,
            "elevation_max_deg": 85,
            "muzzle_init_speed_coef": 1,
            "dispersion_diameter_m": 1,
            "dispersion_range_m": 48,
            "shell_ids": ["m821"]
        }],
        "shells": [{
            "shell_id": "m821",
            "display_name": "M821 HE",
            "prefab_guid": "0123456789ABCDEF",
            "role": "he",
            "init_speed_m_s": 66,
            "init_speed_variation": 3,
            "mass_kg": 4.06,
            "air_drag": 0.000462,
            "side_air_drag_scale": 10,
            "wind_influence_multiplier": 1,
            "dispersion_multiplier": 1,
            "time_to_live_s": 60,
            "standard_dispersion_m": 15,
            "charges": [
                {"rings": 0, "init_speed_coef": 1, "is_default": true},
                {"rings": 1, "init_speed_coef": 1.531, "is_default": false}
            ]
        }]
    })
}

fn sample_bundle(catalog_bytes: &[u8]) -> Value {
    let table = |coefficient: f64| {
        json!({"shell_id": "m821", "init_speed_coef": coefficient, "rows": [
            {"lattice_index": 0, "elevation_mils_6400": 1100, "range_m": 350.2, "column_1": 0, "time_of_flight_s": 11.9}
        ]})
    };
    let sample = |coefficient: f64| {
        json!({"kind": "simulation", "shell_id": "m821", "init_speed_coef": coefficient,
               "inputs": {"elevation": 1.2}, "outputs": {"time_s": 11.9}})
    };
    json!({
        "schema_version": 1,
        "catalog_id": "test_mortars",
        "catalog_version": 1,
        "catalog_sha256": sha256_hex(catalog_bytes),
        "game_build": "1.8.0.13",
        "export_generation_id": "6A6F008DC5395616",
        "resources": [
            {"guid": "0123456789ABCDEF", "resource_name": "{0123456789ABCDEF}Prefabs/Shell.et", "sha256": SHA_OF_RESOURCE}
        ],
        "oracle_run": {
            "plugin_revision": "1",
            "gravity_reported_m_s2": 9.807,
            "run_at": "2026-09-28T12:00:00Z",
            "output_sha256": SHA_OF_RESOURCE
        },
        "native_tables": [table(1.0), table(1.531)],
        "wind_tables": [],
        "oracle_samples": [sample(1.0), sample(1.531)]
    })
}

fn consistent_pair() -> (Vec<u8>, Value, Value) {
    let catalog = sample_catalog();
    let bytes = serde_json::to_vec_pretty(&catalog).expect("serialise catalog");
    let bundle = sample_bundle(&bytes);
    (bytes, catalog, bundle)
}

fn validator(file_name: &str) -> jsonschema::Validator {
    let root = repo_root().expect("repo root");
    let schema = read_json(&definition_path(&root, file_name)).expect("read schema");
    jsonschema::validator_for(&schema).expect("schema compiles")
}

fn assert_one_problem_containing(problems: &[String], needle: &str) {
    assert!(
        problems.iter().any(|problem| problem.contains(needle)),
        "expected a problem containing {needle:?}, got {problems:?}"
    );
}

#[test]
fn ballistics_consistent_pair_passes_schemas_and_provenance() {
    let (bytes, catalog, bundle) = consistent_pair();
    let catalog_errors: Vec<String> = validator("ballistics-catalog.schema.json")
        .iter_errors(&catalog)
        .map(|error| error.to_string())
        .collect();
    assert!(catalog_errors.is_empty(), "{catalog_errors:?}");
    let bundle_errors: Vec<String> = validator("ballistics-calibration.schema.json")
        .iter_errors(&bundle)
        .map(|error| error.to_string())
        .collect();
    assert!(bundle_errors.is_empty(), "{bundle_errors:?}");
    assert_eq!(
        provenance_failures(&bytes, &catalog, &bundle),
        Vec::<String>::new()
    );
}

#[test]
fn ballistics_catalog_schema_refuses_braced_guid_and_missing_default_charge() {
    let mut catalog = sample_catalog();
    catalog["weapons"][0]["prefab_guid"] = json!("{0123456789ABCDEE}");
    assert!(!validator("ballistics-catalog.schema.json").is_valid(&catalog));
    let mut catalog = sample_catalog();
    catalog["shells"][0]["charges"][0]["is_default"] = json!(false);
    assert!(!validator("ballistics-catalog.schema.json").is_valid(&catalog));
}

#[test]
fn ballistics_calibration_schema_requires_the_oracle_run() {
    let (_, _, mut bundle) = consistent_pair();
    bundle.as_object_mut().expect("object").remove("oracle_run");
    assert!(!validator("ballistics-calibration.schema.json").is_valid(&bundle));
}

#[test]
fn ballistics_stale_catalog_sha_is_named() {
    let (bytes, catalog, bundle) = consistent_pair();
    let mut edited = bytes.clone();
    edited.push(b'\n');
    assert_one_problem_containing(
        &provenance_failures(&edited, &catalog, &bundle),
        "catalog_sha256",
    );
}

#[test]
fn ballistics_game_build_mismatch_is_named() {
    let (bytes, catalog, mut bundle) = consistent_pair();
    bundle["game_build"] = json!("1.8.0.14");
    assert_one_problem_containing(
        &provenance_failures(&bytes, &catalog, &bundle),
        "game_build",
    );
}

#[test]
fn ballistics_gravity_mismatch_is_named() {
    let (bytes, catalog, mut bundle) = consistent_pair();
    bundle["oracle_run"]["gravity_reported_m_s2"] = json!(9.81);
    assert_one_problem_containing(&provenance_failures(&bytes, &catalog, &bundle), "gravity");
}

#[test]
fn ballistics_charge_without_native_table_or_simulation_sample_is_named() {
    let (bytes, catalog, mut bundle) = consistent_pair();
    bundle["native_tables"].as_array_mut().expect("array").pop();
    bundle["oracle_samples"]
        .as_array_mut()
        .expect("array")
        .pop();
    let problems = provenance_failures(&bytes, &catalog, &bundle);
    assert_one_problem_containing(&problems, "charge 1: no native table");
    assert_one_problem_containing(&problems, "charge 1: no simulation sample");
}

#[test]
fn ballistics_unknown_shell_and_resource_hash_conflict_are_named() {
    let (bytes, catalog, mut bundle) = consistent_pair();
    bundle["oracle_samples"][0]["shell_id"] = json!("m999");
    bundle["resources"][0]["sha256"] = json!(SHA_OF_RESOURCE.replace('1', "2"));
    let problems = provenance_failures(&bytes, &catalog, &bundle);
    assert_one_problem_containing(&problems, "oracle_samples[0]: unknown shell m999");
    assert_one_problem_containing(&problems, "resource 0123456789ABCDEF");
}

#[test]
fn ballistics_catalog_referential_rules_are_named() {
    let mut catalog = sample_catalog();
    catalog["weapons"][0]["shell_ids"] = json!(["m821", "m999"]);
    catalog["weapons"][0]["elevation_min_deg"] = json!(85);
    catalog["shells"][0]["charges"][1]["is_default"] = json!(true);
    catalog["shells"][0]["time_fuze"] = json!({"min_s": 40, "max_s": 10, "default_s": 24});
    let bytes = serde_json::to_vec_pretty(&catalog).expect("serialise catalog");
    let bundle = sample_bundle(&bytes);
    let problems = provenance_failures(&bytes, &catalog, &bundle);
    assert_one_problem_containing(&problems, "fires unknown shell m999");
    assert_one_problem_containing(&problems, "elevation_min_deg must be below");
    assert_one_problem_containing(&problems, "2 default charges");
    assert_one_problem_containing(&problems, "time fuze");
}

#[test]
fn ballistics_absent_documents_are_not_run_and_one_alone_fails() {
    let repository = repo_root().expect("repo root");
    let schema = |name: &str| read_json(&definition_path(&repository, name));
    let compile = |doc: &Value| {
        jsonschema::validator_for(doc).map_err(|error| anyhow::anyhow!("schema compile: {error}"))
    };
    let checked = Cell::new(0usize);
    let check = |_: &str, _: &jsonschema::Validator, _: &Value| checked.set(checked.get() + 1);
    let scratch = std::env::temp_dir().join(format!(
        "tbd-ballistics-validation-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&scratch);

    let failures = Cell::new(0usize);
    let outcome = validate(&scratch, &schema, &compile, &check, &failures).expect("validate");
    assert!(
        matches!(outcome, BallisticsOutcome::NotRun(_)),
        "{outcome:?}"
    );
    assert_eq!((failures.get(), checked.get()), (0, 0));

    let (catalog_path, _) = ballistics_document_paths(&scratch);
    std::fs::create_dir_all(catalog_path.parent().expect("parent")).expect("create catalog dir");
    std::fs::write(&catalog_path, b"{}").expect("write catalog");
    let outcome = validate(&scratch, &schema, &compile, &check, &failures).expect("validate");
    assert_eq!(outcome, BallisticsOutcome::Checked);
    assert_eq!(failures.get(), 1);
    let _ = std::fs::remove_dir_all(&scratch);
}
