//! Constraint parity of the stored registry row definitions with the catalogue definitions they
//! copy.
//!
//! **Role:** `arsenal-envelopes.schema.json` restates, for each stored registry row, the fields of
//! the catalogue definition the row is imported from, with that definition's constraints: the
//! `kind` and `edge_type` vocabularies, the resource-name pattern, the numeric minimums. This
//! module names every catalogue constraint a row copy does not carry verbatim, so the two schemas
//! cannot drift apart silently.
//!
//! **Position:** used by `contract_parity_registry_row_constraints_match_the_catalogue_schemas` in
//! `tests/contract_parity_goldens.rs`; reads `arsenal-envelopes.schema.json`,
//! `registry-items.schema.json` and `registry-compat.schema.json` from `contracts/definitions`.
//!
//! **Signals & state:** none; pure functions over committed schema files.
//!
//! **Invariants:** every catalogue property is present on the row; every keyword of a catalogue
//! property other than an annotation (`description`, `title`, `examples`, `$comment`), with its
//! same-document `$ref` followed, appears on the row property with an equal value; every name the
//! catalogue requires is required on the row, and both carry the same `additionalProperties`. A
//! row may add keywords (it omits an empty `icon_url` or `evidence`, so it adds `minLength`) and
//! storage columns; it never drops or changes a copied constraint.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::Value;

use super::json_difference::excerpt;

/// The schema file holding the row definitions.
pub const ROW_SCHEMA_FILE: &str = "arsenal-envelopes.schema.json";

/// Keywords that document a property without constraining it.
const ANNOTATIONS: &[&str] = &["description", "title", "examples", "$comment"];

/// How many `$ref` hops one property may take before the chain counts as circular.
const REFERENCE_HOPS: usize = 8;

/// One row definition and the catalogue definition whose fields it copies.
pub struct RowCopy {
    /// The row definition, a JSON pointer into [`ROW_SCHEMA_FILE`].
    pub row: &'static str,
    /// The catalogue schema file under `contracts/definitions`.
    pub catalogue_file: &'static str,
    /// The catalogue definition, a JSON pointer into `catalogue_file`.
    pub catalogue: &'static str,
}

/// Every row definition that copies a catalogue definition.
pub const ROW_COPIES: &[RowCopy] = &[
    RowCopy {
        row: "/definitions/RegistryItemRow",
        catalogue_file: "registry-items.schema.json",
        catalogue: "/$defs/item",
    },
    RowCopy {
        row: "/definitions/RegistryCompatRow",
        catalogue_file: "registry-compat.schema.json",
        catalogue: "/$defs/edge",
    },
];

/// One committed schema file of `contracts/definitions`, parsed.
///
/// # Panics
///
/// When the file is missing or is not JSON.
pub fn read_schema(file: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../contracts/definitions")
        .join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

/// Every catalogue constraint the row copy does not carry verbatim, one report line each.
pub fn constraint_differences(
    copy: &RowCopy,
    row_document: &Value,
    catalogue_document: &Value,
) -> Vec<String> {
    let at = |detail: String| {
        format!(
            "{ROW_SCHEMA_FILE}#{} against {}#{}: {detail}",
            copy.row, copy.catalogue_file, copy.catalogue
        )
    };
    let (Some(row), Some(catalogue)) = (
        row_document.pointer(copy.row),
        catalogue_document.pointer(copy.catalogue),
    ) else {
        return vec![at("a definition is missing".to_string())];
    };
    let mut lines = Vec::new();
    let row_required = required_names(row);
    for name in required_names(catalogue) {
        if !row_required.contains(&name) {
            lines.push(at(format!(
                "`{name}` is required by the catalogue, not by the row"
            )));
        }
    }
    if row.get("additionalProperties") != catalogue.get("additionalProperties") {
        lines.push(at(
            "`additionalProperties` differs between the row and the catalogue".to_string(),
        ));
    }
    let catalogue_properties = catalogue.get("properties").and_then(Value::as_object);
    let Some(catalogue_properties) = catalogue_properties.filter(|found| !found.is_empty()) else {
        lines.push(at(
            "the catalogue definition declares no property".to_string()
        ));
        return lines;
    };
    for (name, catalogue_property) in catalogue_properties {
        let Some(row_property) = row.get("properties").and_then(|found| found.get(name)) else {
            lines.push(at(format!("the row lacks the catalogue property `{name}`")));
            continue;
        };
        let constraint_pair =
            constraints(catalogue_document, catalogue_property).and_then(|original| {
                constraints(row_document, row_property).map(|copied| (original, copied))
            });
        match constraint_pair {
            Ok((original, copied)) => {
                for (keyword, value) in &original {
                    match copied.get(keyword) {
                        Some(copied_value) if copied_value == value => {}
                        Some(copied_value) => lines.push(at(format!(
                            "`{name}`.{keyword}: {}",
                            describe_change(value, copied_value)
                        ))),
                        None => lines.push(at(format!(
                            "`{name}` drops the catalogue's `{keyword}` {}",
                            excerpt(value)
                        ))),
                    }
                }
            }
            Err(problem) => lines.push(at(format!("`{name}`: {problem}"))),
        }
    }
    lines
}

/// The `required` names of a definition.
fn required_names(definition: &Value) -> Vec<String> {
    definition
        .get("required")
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The constraining keywords of one property schema, its same-document `$ref` followed.
fn constraints(document: &Value, property: &Value) -> Result<BTreeMap<String, Value>, String> {
    let mut kept = BTreeMap::new();
    let mut current = property;
    for _ in 0..=REFERENCE_HOPS {
        let object = current
            .as_object()
            .ok_or_else(|| format!("the property schema {} is not an object", excerpt(current)))?;
        for (keyword, value) in object {
            if keyword != "$ref" && !ANNOTATIONS.contains(&keyword.as_str()) {
                kept.entry(keyword.clone()).or_insert_with(|| value.clone());
            }
        }
        let Some(reference) = object.get("$ref") else {
            return Ok(kept);
        };
        let pointer = reference
            .as_str()
            .and_then(|text| text.strip_prefix('#'))
            .ok_or_else(|| format!("`$ref` {} leaves its document", excerpt(reference)))?;
        current = document
            .pointer(pointer)
            .ok_or_else(|| format!("`$ref` #{pointer} resolves to nothing"))?;
    }
    Err(format!("a `$ref` chain exceeds {REFERENCE_HOPS} hops"))
}

/// How the copied value differs from the original: for arrays, the values each side lacks.
fn describe_change(original: &Value, copied: &Value) -> String {
    let (Some(original_values), Some(copied_values)) = (original.as_array(), copied.as_array())
    else {
        return format!(
            "the catalogue says {} but the row says {}",
            excerpt(original),
            excerpt(copied)
        );
    };
    let dropped: Vec<String> = original_values
        .iter()
        .filter(|value| !copied_values.contains(value))
        .map(excerpt)
        .collect();
    let added: Vec<String> = copied_values
        .iter()
        .filter(|value| !original_values.contains(value))
        .map(excerpt)
        .collect();
    if dropped.is_empty() && added.is_empty() {
        return "the row lists the catalogue's values in another order or with repeats".to_string();
    }
    format!(
        "the row drops [{}] and adds [{}]",
        dropped.join(", "),
        added.join(", ")
    )
}
