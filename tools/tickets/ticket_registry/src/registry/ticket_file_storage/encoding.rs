//! The on-disk encoding of one ticket: JSON value to TOML text and back.

use super::*;
use crate::error::{Error, Result, ResultExt};
use ticket_model::TicketId;

/// The string a JSON `null` is written as, since TOML has no null.
pub(super) const NULL_SENTINEL: &str = "__tbd_null__";

/// The table key that records the ticket's position in the registry.
pub(super) const ORD_KEY: &str = "__ord";

/// The table key that records the object's own key order.
pub(super) const KEYS_KEY: &str = "__keys";

/// The ticket folder of the checkout at `root` ([`repository_layout::TICKETS_DIR`]).
pub fn tickets_dir(root: &Path) -> PathBuf {
    root.join(repository_layout::TICKETS_DIR)
}

/// The registry root marker of the checkout at `root` ([`repository_root::ROOT_MARKER`]).
pub fn root_marker_path(root: &Path) -> PathBuf {
    root.join(repository_root::ROOT_MARKER)
}

/// The file of ticket `id` in the checkout at `root`: `<tickets dir>/<id>.toml`.
pub fn parent_toml_path(root: &Path, id: &TicketId) -> PathBuf {
    tickets_dir(root).join(format!("{id}.toml"))
}

/// The next free parent ticket number: the highest parent number among the rows' ids plus one,
/// or 1 when no row carries a parent id.
pub fn derive_next_id(tickets: &[Value]) -> u64 {
    let max = tickets
        .iter()
        .filter_map(|t| t.get("id").and_then(Value::as_str))
        .filter_map(|id| TicketId::new(id).parent_number())
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
                    return Err(Error::msg(format!("integer {u} does not fit toml i64")));
                }
                toml::Value::Integer(u as i64)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                return Err(Error::msg(format!("unhandled json number {n}")));
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

/// One ticket's JSON object as TOML text, with its key order in `__keys`, its registry position
/// `ord` in `__ord`, and every `null` written as the sentinel string.
///
/// # Errors
/// When `ticket` is not an object, holds an integer beyond `i64`, or the TOML cannot be rendered.
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

/// One ticket file's text back as its registry position and JSON value, in the recorded key
/// order; the position is `i64::MAX` when the file records none.
///
/// # Errors
/// When the text is not TOML or holds a non-finite float.
pub fn ticket_from_toml_str(text: &str) -> Result<(i64, Value)> {
    let table: toml::Table = text.parse().context("parse ticket toml")?;
    let ord = table
        .get(ORD_KEY)
        .and_then(|v| v.as_integer())
        .unwrap_or(i64::MAX);
    Ok((ord, toml_to_json(&toml::Value::Table(table))?))
}
