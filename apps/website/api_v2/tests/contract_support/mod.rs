//! JSON Schema validation of live responses and request bodies against the published contracts
//! in `contracts_v2/definitions`.
//!
//! Compiled into each suite that writes `mod contract_support;`; it adds no test binary.

#![allow(dead_code)]

use std::path::PathBuf;

use serde_json::{Value, json};

fn definitions_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../contracts_v2/definitions")
}

/// Draft-07 defines no `uuid` format, so a draft-07 validator passes any string for it. The
/// contracts use it for every identifier, and it is enforced here as the hyphenated form the API
/// serializes.
fn is_hyphenated_uuid(value: &str) -> bool {
    value.len() == 36 && uuid::Uuid::try_parse(value).is_ok()
}

/// The schema file, or one of its definitions, as a format-checking validator.
pub fn validator(schema_file: &str, definition: Option<&str>) -> jsonschema::Validator {
    let path = definitions_dir().join(schema_file);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let document: Value = serde_json::from_str(&raw).expect("schema is JSON");
    let schema = match definition {
        None => document,
        Some(name) => {
            assert!(
                document["definitions"].get(name).is_some(),
                "{schema_file} defines no {name}"
            );
            json!({
                "$schema": document["$schema"],
                "$ref": format!("#/definitions/{name}"),
                "definitions": document["definitions"],
            })
        }
    };
    jsonschema::options()
        .should_validate_formats(true)
        .with_format("uuid", is_hyphenated_uuid)
        .build(&schema)
        .unwrap_or_else(|error| panic!("{schema_file}: {error}"))
}

/// Assert `value` satisfies the contract, naming every violation.
pub fn assert_valid(schema_file: &str, definition: Option<&str>, value: &Value) {
    let validator = validator(schema_file, definition);
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "{schema_file}{} rejects the value:\n  {}\n{value:#}",
        definition
            .map(|name| format!("#{name}"))
            .unwrap_or_default(),
        errors.join("\n  ")
    );
}

/// Assert `value` violates the contract: the schema is strict enough to see the difference.
pub fn assert_invalid(schema_file: &str, definition: Option<&str>, value: &Value) {
    assert!(
        !validator(schema_file, definition).is_valid(value),
        "{schema_file} accepts a value it must reject: {value}"
    );
}

/// Assert the type generated from the same contract decodes `value`.
pub fn assert_decodes<T: serde::de::DeserializeOwned>(what: &str, value: &Value) {
    if let Err(error) = serde_json::from_value::<T>(value.clone()) {
        panic!("{what}: the generated contract type rejects the value: {error}\n{value:#}");
    }
}
