//! Comparing a live JSON body with the value its generated contract type re-serialises.
//!
//! **Role:** lists every difference between a live body and its round trip through the
//! generated type, walking the body beside its contract's schema: every value compares exactly,
//! except a string the schema declares `format: date-time`, which compares as the instant it
//! names; that live string must also be the API's own spelling of its instant
//! ([`rfc3339_utc::format`]: UTC, `Z`, trailing fractional zeros trimmed), so the comparison
//! never relaxes the wire format.
//!
//! **Position:** test support; the dimension runner's contract parity pass calls
//! [`round_trip_differences`] with the spec's contract, and the coverage binary's self-tests
//! call [`differences_beside_schema`] with synthetic schemas; schema documents load through
//! [`super::contracts::load_schema_document`].
//!
//! **Signals & state:** none; pure functions over one loaded schema document.
//!
//! **Invariants:** a schema is followed through local `$ref`s, `allOf`, `anyOf`, `oneOf`,
//! `properties`, `additionalProperties`, `items`, `prefixItems` and `additionalItems`, at most
//! [`MAX_SCHEMA_DEPTH`] references deep; a value whose schema does not resolve compares exactly,
//! so an unresolved schema never loosens the comparison; a key the round trip drops or adds and
//! an array whose length changes are always differences.

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use website_api::core::wire_format::rfc3339_utc;

use super::contracts::load_schema_document;
use super::spec::Contract;

/// How many `$ref` and composition steps are followed from one value's schema.
pub const MAX_SCHEMA_DEPTH: usize = 32;

/// The schema document of `contract` and the node that shapes its whole body, or `None` for a
/// contract that names no schema.
fn body_schema(contract: &Contract) -> Result<Option<(Value, Value)>, String> {
    let (file, definition, array) = match contract {
        Contract::Schema { file, definition } | Contract::EventStream { file, definition } => {
            (*file, *definition, false)
        }
        Contract::SchemaItems { file, definition } => (*file, Some(*definition), true),
        Contract::NoBody | Contract::Binary(_) | Contract::RefusalEnvelope { .. } => {
            return Ok(None);
        }
    };
    let document = load_schema_document(file)?;
    let entry = match definition {
        None => document.clone(),
        Some(name) if document["definitions"].get(name).is_some() => {
            json!({ "$ref": format!("#/definitions/{name}") })
        }
        Some(name) => json!({ "$ref": format!("#/$defs/{name}") }),
    };
    let node = if array {
        json!({ "type": "array", "items": entry })
    } else {
        entry
    };
    Ok(Some((document, node)))
}

/// Every difference between the live `body` and `again`, its round trip through the generated
/// type, read beside the schema `contract` names.
pub fn round_trip_differences(contract: &Contract, body: &Value, again: &Value) -> Vec<String> {
    match body_schema(contract) {
        Ok(Some((document, node))) => differences_beside_schema(&document, &node, body, again),
        Ok(None) => differences_beside_schema(&Value::Null, &Value::Null, body, again),
        Err(problem) => vec![problem],
    }
}

/// Every difference between `live` and `again` read beside `schema`, whose local `$ref`s
/// resolve inside `document`.
pub fn differences_beside_schema(
    document: &Value,
    schema: &Value,
    live: &Value,
    again: &Value,
) -> Vec<String> {
    let mut differences = Vec::new();
    compare(document, vec![schema], "", live, again, &mut differences);
    differences
}

/// Every schema object that applies to one value: `nodes`, the targets of their local
/// `$ref`s and the members of their `allOf`, `anyOf` and `oneOf`, transitively.
fn applicable<'a>(document: &'a Value, nodes: Vec<&'a Value>) -> Vec<&'a Value> {
    let mut found = Vec::new();
    let mut pending: Vec<(&Value, usize)> = nodes.into_iter().map(|node| (node, 0)).collect();
    while let Some((node, depth)) = pending.pop() {
        if depth > MAX_SCHEMA_DEPTH || !node.is_object() {
            continue;
        }
        found.push(node);
        let target = node
            .get("$ref")
            .and_then(Value::as_str)
            .and_then(|reference| reference.strip_prefix('#'))
            .and_then(|pointer| document.pointer(pointer));
        if let Some(target) = target {
            pending.push((target, depth + 1));
        }
        for keyword in ["allOf", "anyOf", "oneOf"] {
            for member in node
                .get(keyword)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                pending.push((member, depth + 1));
            }
        }
    }
    found
}

fn declares_date_time(schemas: &[&Value]) -> bool {
    schemas
        .iter()
        .any(|schema| schema.get("format").and_then(Value::as_str) == Some("date-time"))
}

/// The schemas of property `key`: its `properties` entry, else an object `additionalProperties`.
fn property_schemas<'a>(schemas: &[&'a Value], key: &str) -> Vec<&'a Value> {
    schemas
        .iter()
        .filter_map(|schema| {
            schema
                .get("properties")
                .and_then(|properties| properties.get(key))
                .or_else(|| schema.get("additionalProperties"))
        })
        .collect()
}

/// The schemas of element `index`: its positional entry (`prefixItems`, or a draft-07 `items`
/// list), else the schema of every further element (`items` or `additionalItems`).
fn item_schemas<'a>(schemas: &[&'a Value], index: usize) -> Vec<&'a Value> {
    schemas
        .iter()
        .filter_map(|schema| {
            let prefix = schema.get("prefixItems").and_then(Value::as_array);
            let listed = schema.get("items").and_then(Value::as_array);
            match (prefix, listed) {
                (Some(prefix), _) => prefix.get(index).or_else(|| schema.get("items")),
                (None, Some(listed)) => listed.get(index).or_else(|| schema.get("additionalItems")),
                (None, None) => schema.get("items"),
            }
        })
        .collect()
}

/// The JSON pointer of `key` under `pointer`.
fn child_pointer(pointer: &str, key: &str) -> String {
    format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"))
}

fn compare<'a>(
    document: &'a Value,
    schemas: Vec<&'a Value>,
    pointer: &str,
    live: &Value,
    again: &Value,
    differences: &mut Vec<String>,
) {
    let schemas = applicable(document, schemas);
    let at = if pointer.is_empty() { "/" } else { pointer };
    if let (Value::String(sent), Value::String(back)) = (live, again)
        && declares_date_time(&schemas)
    {
        differences.extend(instant_differences(at, sent, back));
        return;
    }
    match (live, again) {
        (Value::Object(sent), Value::Object(back)) => {
            for key in sent.keys().filter(|key| !back.contains_key(*key)) {
                let at = child_pointer(pointer, key);
                differences.push(format!("`{at}`: the generated type drops it"));
            }
            for (key, value) in back.iter().filter(|(key, _)| !sent.contains_key(*key)) {
                let at = child_pointer(pointer, key);
                differences.push(format!("`{at}`: the generated type adds {value}"));
            }
            for (key, value) in sent {
                if let Some(other) = back.get(key) {
                    let children = property_schemas(&schemas, key);
                    let at = child_pointer(pointer, key);
                    compare(document, children, &at, value, other, differences);
                }
            }
        }
        (Value::Array(sent), Value::Array(back)) if sent.len() == back.len() => {
            for (index, (value, other)) in sent.iter().zip(back).enumerate() {
                let children = item_schemas(&schemas, index);
                let at = format!("{pointer}/{index}");
                compare(document, children, &at, value, other, differences);
            }
        }
        _ if live == again => {}
        _ => differences.push(format!("`{at}`: sent {live}, back {again}")),
    }
}

/// The differences of one `date-time` string: both sides must parse as RFC 3339, name the same
/// instant, and the live side must be the API's spelling of that instant.
fn instant_differences(at: &str, sent: &str, back: &str) -> Vec<String> {
    let parse = |text: &str| DateTime::parse_from_rfc3339(text).map(|t| t.with_timezone(&Utc));
    let (sent_instant, back_instant) = match (parse(sent), parse(back)) {
        (Ok(sent_instant), Ok(back_instant)) => (sent_instant, back_instant),
        (Err(error), _) => {
            return vec![format!(
                "`{at}`: the live `{sent}` is not RFC 3339: {error}"
            )];
        }
        (_, Err(error)) => {
            return vec![format!(
                "`{at}`: the round trip `{back}` is not RFC 3339: {error}"
            )];
        }
    };
    let mut differences = Vec::new();
    if sent_instant != back_instant {
        differences.push(format!(
            "`{at}`: sent {sent}, back {back}: different instants"
        ));
    }
    let spelling = rfc3339_utc::format(&sent_instant);
    if sent != spelling {
        differences.push(format!(
            "`{at}`: the live `{sent}` is not the API spelling `{spelling}` of its instant"
        ));
    }
    differences
}
