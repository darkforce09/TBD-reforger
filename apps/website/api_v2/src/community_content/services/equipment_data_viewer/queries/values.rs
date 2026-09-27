//! Expand exact JSON values without numeric coercion or omitted array entries.
use super::super::source::document::{pointer_segment, resolve_pointer};
use anyhow::Result;
use serde_json::{Value, json, value::RawValue};
use std::collections::BTreeMap;

pub(super) fn entry(key: &str, raw: &RawValue) -> Result<Value> {
    let text = raw.get();
    let (kind, count) = match text.as_bytes().first() {
        Some(b'[') => (
            "array",
            serde_json::from_str::<Vec<Box<RawValue>>>(text)?.len(),
        ),
        Some(b'{') => (
            "object",
            serde_json::from_str::<BTreeMap<String, Box<RawValue>>>(text)?.len(),
        ),
        Some(b'"') => (
            "string",
            serde_json::from_str::<String>(text)?.chars().count(),
        ),
        Some(b'n') => ("null", 0),
        Some(b't' | b'f') => ("boolean", 1),
        _ => ("number", 1),
    };
    let expanded = text.len() <= 2048;
    Ok(
        json!({"key":key,"label":key,"entry_kind":"value","node_id":null,"class_name":null,"view":null,"native_type":null,"status":null,"origin":null,"native_unit":null,"unit_evidence":null,"value_json":if expanded{text}else{""},"value_kind":kind,"value_count":count,"expanded":expanded,"metadata_json":"{}","links":[]}),
    )
}

pub(super) fn expand(raw: &RawValue, pointer: &str, offset: usize) -> Result<(usize, Vec<Value>)> {
    let selected = resolve_pointer(raw, pointer)?;
    let text = selected.get();
    let mut entries = Vec::new();
    let total = match text.as_bytes().first() {
        Some(b'[') => {
            let values: Vec<Box<RawValue>> = serde_json::from_str(text)?;
            let total = values.len();
            for (i, value) in values.into_iter().enumerate().skip(offset).take(100) {
                entries.push(entry(&format!("{pointer}/{i}"), &value)?);
            }
            total
        }
        Some(b'{') => {
            let values: BTreeMap<String, Box<RawValue>> = serde_json::from_str(text)?;
            let total = values.len();
            for (key, value) in values.into_iter().skip(offset).take(100) {
                let mut item = entry(&format!("{pointer}/{}", pointer_segment(&key)), &value)?;
                item["label"] = json!(key);
                entries.push(item);
            }
            total
        }
        Some(b'"') => {
            let value: String = serde_json::from_str(text)?;
            let chars: Vec<_> = value.chars().collect();
            let total = chars.len().div_ceil(4096).max(1);
            for (i, chunk) in chars.chunks(4096).enumerate().skip(offset).take(20) {
                let fragment = RawValue::from_string(serde_json::to_string(
                    &chunk.iter().collect::<String>(),
                )?)?;
                let mut row = entry(
                    &format!("characters {}–{}", i * 4096, i * 4096 + chunk.len()),
                    &fragment,
                )?;
                row["value_json"] = json!(fragment.get());
                row["expanded"] = json!(true);
                entries.push(row);
            }
            if chars.is_empty() && offset == 0 {
                entries.push(entry(pointer, &selected)?);
            }
            total
        }
        _ => {
            if offset == 0 {
                entries.push(entry(pointer, &selected)?);
            }
            1
        }
    };
    if total == 0 && offset == 0 {
        entries.push(entry(pointer, &selected)?);
        return Ok((1, entries));
    }
    Ok((total, entries))
}
