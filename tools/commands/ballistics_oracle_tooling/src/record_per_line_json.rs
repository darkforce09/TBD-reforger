//! JSON written with one record per line: the layout of the calibration bundles.
//!
//! **Role:** Serializes a document as indented JSON in which every array element that is a leaf
//! record (an object or array holding no list of objects or arrays) is written whole on one line,
//! so a bundle of thousands of table rows and oracle samples reads, diffs and greps one case per
//! line at under half the size of fully indented JSON.
//!
//! **Position:** Used by the trim for the calibration bundle and its refused variants; the small
//! catalog keeps the fully indented layout.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** The output parses to exactly the value serialized (only whitespace differs from
//! `serde_json::to_vec_pretty`); indentation is two spaces; the document ends with a newline.
use crate::error::Result;
use serde::Serialize;
use serde_json::Value;

fn holds_record_list(value: &Value) -> bool {
    match value {
        Value::Array(items) => items
            .iter()
            .any(|item| item.is_object() || item.is_array() || holds_record_list(item)),
        Value::Object(fields) => fields.values().any(holds_record_list),
        _ => false,
    }
}

fn write_value(out: &mut String, value: &Value, indent: usize, array_element: bool) -> Result<()> {
    let is_container = value.is_object() || value.is_array();
    if !is_container || (array_element && !holds_record_list(value)) {
        out.push_str(&serde_json::to_string(value)?);
        return Ok(());
    }
    let inner = " ".repeat(indent + 2);
    let (open, close, entries): (char, char, Vec<(Option<&String>, &Value)>) = match value {
        Value::Object(fields) => (
            '{',
            '}',
            fields.iter().map(|(key, item)| (Some(key), item)).collect(),
        ),
        Value::Array(items) => ('[', ']', items.iter().map(|item| (None, item)).collect()),
        _ => unreachable!("scalars returned above"),
    };
    out.push(open);
    if entries.is_empty() {
        out.push(close);
        return Ok(());
    }
    for (index, (key, item)) in entries.iter().enumerate() {
        out.push('\n');
        out.push_str(&inner);
        if let Some(key) = key {
            out.push_str(&serde_json::to_string(key)?);
            out.push_str(": ");
        }
        write_value(out, item, indent + 2, key.is_none())?;
        if index + 1 < entries.len() {
            out.push(',');
        }
    }
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push(close);
    Ok(())
}

/// `value` as indented JSON with one leaf record per line and a final newline.
pub(crate) fn record_per_line_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut out = String::new();
    write_value(&mut out, &serde_json::to_value(value)?, 0, false)?;
    out.push('\n');
    Ok(out.into_bytes())
}
