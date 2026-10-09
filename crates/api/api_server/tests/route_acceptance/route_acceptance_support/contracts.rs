//! Checking response and request bodies against the published contracts.
//!
//! **Role:** validates a JSON value against a [`Contract`] (a schema root, a draft-07
//! `definitions` entry, a 2020-12 `$defs` entry, an array of such entries, or the exact refusal
//! envelope).
//!
//! **Position:** test support; wraps `tests/contract_support` for draft-07 definitions and
//! builds the `$defs` validator itself; used by the dimension runner.
//!
//! **Signals & state:** none.
//!
//! **Invariants:** validation checks formats, with `uuid` enforced as the hyphenated form the
//! API serialises; a contract naming a missing file or entry is an error, never a pass; an
//! array contract refuses a body that is not an array and validates every element.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::spec::Contract;
use crate::contract_support;

fn definitions_dir() -> PathBuf {
    repository_root::find_repository_root_from(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("the repository root above the API package")
        .join("contracts/definitions")
}

/// The parsed schema document `file` of `contracts/definitions/`.
pub(crate) fn load_schema_document(file: &str) -> Result<Value, String> {
    let path = definitions_dir().join(file);
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("{file} is not JSON: {e}"))
}

fn is_hyphenated_uuid(value: &str) -> bool {
    value.len() == 36 && uuid::Uuid::try_parse(value).is_ok()
}

fn validator(file: &str, definition: Option<&str>) -> Result<jsonschema::Validator, String> {
    let document = load_schema_document(file)?;
    let Some(name) = definition else {
        return Ok(contract_support::validator(file, None));
    };
    if document["definitions"].get(name).is_some() {
        return Ok(contract_support::validator(file, Some(name)));
    }
    if document["$defs"].get(name).is_none() {
        return Err(format!("{file} defines no `{name}`"));
    }
    let schema = json!({
        "$schema": document["$schema"],
        "$ref": format!("#/$defs/{name}"),
        "$defs": document["$defs"],
    });
    jsonschema::options()
        .should_validate_formats(true)
        .with_format("uuid", is_hyphenated_uuid)
        .build(&schema)
        .map_err(|error| format!("{file}: {error}"))
}

/// Every violation of `value` against the schema entry `file#definition`.
pub(crate) fn violations(file: &str, definition: Option<&str>, value: &Value) -> Vec<String> {
    match validator(file, definition) {
        Ok(validator) => validator
            .iter_errors(value)
            .map(|error| format!("{error} at `{}`", error.instance_path()))
            .collect(),
        Err(error) => vec![error],
    }
}

/// Every violation of an array `value` whose elements are each shaped by `file#definition`.
fn item_violations(file: &str, definition: &str, value: &Value) -> Vec<String> {
    let Some(items) = value.as_array() else {
        let kind = match value {
            Value::Object(_) => "an object",
            Value::Null => "null",
            _ => "a scalar",
        };
        return vec![format!("the body is {kind}, not a JSON array")];
    };
    let validator = match validator(file, Some(definition)) {
        Ok(validator) => validator,
        Err(error) => return vec![error],
    };
    items
        .iter()
        .enumerate()
        .flat_map(|(index, item)| {
            validator
                .iter_errors(item)
                .map(|error| format!("{error} at `/{index}{}`", error.instance_path()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Every way the JSON `value` contradicts `contract` (for an event stream, `value` is its
/// first frame's `data`).
pub(crate) fn json_violations(contract: &Contract, value: &Value) -> Vec<String> {
    match contract {
        Contract::Schema { file, definition } | Contract::EventStream { file, definition } => {
            violations(file, *definition, value)
        }
        Contract::SchemaItems { file, definition } => item_violations(file, definition, value),
        Contract::RefusalEnvelope { error } => {
            let expected = json!({ "error": error });
            if *value == expected {
                Vec::new()
            } else {
                vec![format!("the refusal is not exactly {expected}")]
            }
        }
        Contract::NoBody | Contract::Binary(_) => {
            vec!["the contract describes no JSON body".to_string()]
        }
    }
}
