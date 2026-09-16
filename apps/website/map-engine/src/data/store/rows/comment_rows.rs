//! Role: comment rows.
//! Position: `doc/store` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Any;
use super::Arc;
use super::HashMap;
use super::Map;
use super::MapRef;
use super::Out;
use super::ReadTxn;
use yrs::types::ToJson;

/// Comment row using the supplied domain data.
pub(super) fn comment_row(
    id: &str,
    title: &str,
    tooltip: &str,
    x: f64,
    z: f64,
) -> HashMap<String, Any> {
    let mut pos: HashMap<String, Any> = HashMap::new();
    pos.insert("x".to_string(), Any::Number(x));
    pos.insert("z".to_string(), Any::Number(z));
    let mut row: HashMap<String, Any> = HashMap::new();
    row.insert("id".to_string(), Any::String(id.into()));
    row.insert("title".to_string(), Any::String(title.into()));
    row.insert("tooltip".to_string(), Any::String(tooltip.into()));
    row.insert("position".to_string(), Any::Map(Arc::new(pos)));
    row
}

/// Read comment map using the supplied domain data.
pub(super) fn read_comment_map<T: ReadTxn>(
    txn: &T,
    comments: &MapRef,
    id: &str,
) -> Option<HashMap<String, Any>> {
    match comments.get(txn, id) {
        Some(Out::Any(Any::Map(m))) => Some((*m).clone()),
        Some(Out::YMap(row)) => Some(
            row.iter(txn)
                .map(|(k, out)| match out {
                    Out::Any(a) => (k.to_string(), a),
                    other => (k.to_string(), other.to_json(txn)),
                })
                .collect(),
        ),
        _ => None,
    }
}

/// Comment xz using the supplied domain data.
pub(super) fn comment_xz(row: &HashMap<String, Any>) -> (f64, f64) {
    let Some(Any::Map(pos)) = row.get("position") else {
        return (0.0, 0.0);
    };
    let num = |k: &str| match pos.get(k) {
        Some(Any::Number(n)) => *n,
        #[allow(clippy::cast_precision_loss)]
        Some(Any::BigInt(i)) => *i as f64,
        _ => 0.0,
    };
    (num("x"), num("z"))
}

/// Comment str using the supplied domain data.
pub(super) fn comment_str(row: &HashMap<String, Any>, key: &str) -> String {
    match row.get(key) {
        Some(Any::String(s)) => s.to_string(),
        _ => String::new(),
    }
}
