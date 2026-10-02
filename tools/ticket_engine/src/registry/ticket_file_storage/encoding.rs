//! The on-disk encoding of one ticket: JSON value to TOML text and back.

use super::*;
use anyhow::Context;

pub(super) const NULL_SENTINEL: &str = "__tbd_null__";

pub(super) const ORD_KEY: &str = "__ord";

pub(super) const KEYS_KEY: &str = "__keys";

pub fn tickets_dir(root: &Path) -> PathBuf {
    root.join(crate::repository::TICKETS_DIR)
}

pub fn root_marker_path(root: &Path) -> PathBuf {
    root.join(crate::repository::ROOT_MARKER)
}

pub fn parent_toml_path(root: &Path, id: &str) -> PathBuf {
    tickets_dir(root).join(format!("{id}.toml"))
}

pub fn is_parent_id(id: &str) -> bool {
    let rest = match id.strip_prefix("T-") {
        Some(r) => r,
        None => return false,
    };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

pub fn parent_numeric_id(id: &str) -> Option<u64> {
    if !is_parent_id(id) {
        return None;
    }
    id.strip_prefix("T-")?.parse().ok()
}

pub fn derive_next_id(tickets: &[Value]) -> u64 {
    let max = tickets
        .iter()
        .filter_map(|t| t.get("id").and_then(Value::as_str))
        .filter_map(parent_numeric_id)
        .max()
        .unwrap_or(0);
    max + 1
}

pub(super) fn encode_object(map: &Map<String, Value>) -> Result<toml::Value> {
    let mut table = toml::map::Map::new();
    let keys: Vec<toml::Value> = map.keys().map(|k| toml::Value::String(k.clone())).collect();
    table.insert(KEYS_KEY.into(), toml::Value::Array(keys));
    for (k, val) in map {
        table.insert(k.clone(), encode_json_value(val)?);
    }
    Ok(toml::Value::Table(table))
}

pub(super) fn encode_json_value(v: &Value) -> Result<toml::Value> {
    Ok(match v {
        Value::Null => toml::Value::String(NULL_SENTINEL.into()),
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(u) = n.as_u64() {
                if u > i64::MAX as u64 {
                    bail!("integer {u} does not fit toml i64");
                }
                toml::Value::Integer(u as i64)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                bail!("unhandled json number {n}");
            }
        }
        Value::String(s) => toml::Value::String(s.clone()),
        Value::Array(arr) => toml::Value::Array(
            arr.iter()
                .map(encode_json_value)
                .collect::<Result<Vec<_>>>()?,
        ),
        Value::Object(map) => encode_object(map)?,
    })
}

pub(super) fn decode_table(table: &toml::map::Map<String, toml::Value>) -> Result<Value> {
    let order: Vec<String> = match table.get(KEYS_KEY).and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        None => table
            .keys()
            .filter(|k| *k != ORD_KEY && *k != KEYS_KEY)
            .cloned()
            .collect(),
    };
    let mut map = Map::new();
    for k in order {
        if k == ORD_KEY || k == KEYS_KEY {
            continue;
        }
        let Some(val) = table.get(&k) else {
            continue;
        };
        map.insert(k, toml_to_json(val)?);
    }
    Ok(Value::Object(map))
}

pub(super) fn toml_to_json(v: &toml::Value) -> Result<Value> {
    Ok(match v {
        toml::Value::String(s) if s == NULL_SENTINEL => Value::Null,
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::Number((*i).into()),
        toml::Value::Float(f) => {
            let n = serde_json::Number::from_f64(*f)
                .with_context(|| format!("non-finite float {f}"))?;
            Value::Number(n)
        }
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
        toml::Value::Array(arr) => {
            Value::Array(arr.iter().map(toml_to_json).collect::<Result<Vec<_>>>()?)
        }
        toml::Value::Table(table) => decode_table(table)?,
    })
}

pub fn ticket_to_toml_string(ticket: &Value, ord: i64) -> Result<String> {
    let obj = ticket
        .as_object()
        .with_context(|| "ticket is not an object")?;
    let mut table = match encode_object(obj)? {
        toml::Value::Table(t) => t,
        toml::Value::String(_)
        | toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_)
        | toml::Value::Array(_) => unreachable!(),
    };
    table.insert(ORD_KEY.into(), toml::Value::Integer(ord));
    toml::to_string_pretty(&toml::Value::Table(table)).context("serialize ticket toml")
}

pub fn ticket_from_toml_str(text: &str) -> Result<(i64, Value)> {
    let parsed: toml::Value = text.parse().context("parse ticket toml")?;
    let table = parsed
        .as_table()
        .with_context(|| "ticket toml root is not a table")?;
    let ord = table
        .get(ORD_KEY)
        .and_then(|v| v.as_integer())
        .unwrap_or(i64::MAX);
    Ok((ord, toml_to_json(&parsed)?))
}
