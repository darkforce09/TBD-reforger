use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::root::registry_path;

pub type Registry = Value;

pub fn load_registry(root: &Path) -> Result<Registry> {
    let path = registry_path(root);
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let v: Value = serde_json::from_str(&text).context("parse registry.json")?;
    Ok(v)
}

/// Match Python save_registry: indent=2, ensure_ascii=False, trailing newline.
pub fn save_registry(root: &Path, data: &Registry) -> Result<()> {
    let path = registry_path(root);
    fs::write(&path, format_json_unicode_preserve(data)?)?;
    Ok(())
}

fn format_json_unicode_preserve(data: &Value) -> Result<String> {
    // Produce indent-2 JSON with unicode preserved (like ensure_ascii=False).
    let mut buf = String::new();
    write_value(&mut buf, data, 0)?;
    buf.push('\n');
    Ok(buf)
}

fn write_value(buf: &mut String, v: &Value, indent: usize) -> Result<()> {
    match v {
        Value::Null => buf.push_str("null"),
        Value::Bool(b) => buf.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => buf.push_str(&n.to_string()),
        Value::String(s) => {
            buf.push('"');
            for ch in s.chars() {
                match ch {
                    '"' => buf.push_str("\\\""),
                    '\\' => buf.push_str("\\\\"),
                    '\n' => buf.push_str("\\n"),
                    '\r' => buf.push_str("\\r"),
                    '\t' => buf.push_str("\\t"),
                    c if (c as u32) < 0x20 => {
                        buf.push_str(&format!("\\u{:04x}", c as u32));
                    }
                    c => buf.push(c),
                }
            }
            buf.push('"');
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                buf.push_str("[]");
            } else {
                buf.push_str("[\n");
                for (i, item) in arr.iter().enumerate() {
                    buf.push_str(&"  ".repeat(indent + 1));
                    write_value(buf, item, indent + 1)?;
                    if i + 1 != arr.len() {
                        buf.push(',');
                    }
                    buf.push('\n');
                }
                buf.push_str(&"  ".repeat(indent));
                buf.push(']');
            }
        }
        Value::Object(map) => {
            if map.is_empty() {
                buf.push_str("{}");
            } else {
                buf.push_str("{\n");
                let len = map.len();
                for (i, (k, val)) in map.iter().enumerate() {
                    buf.push_str(&"  ".repeat(indent + 1));
                    buf.push('"');
                    for ch in k.chars() {
                        match ch {
                            '"' => buf.push_str("\\\""),
                            '\\' => buf.push_str("\\\\"),
                            c => buf.push(c),
                        }
                    }
                    buf.push_str("\": ");
                    write_value(buf, val, indent + 1)?;
                    if i + 1 != len {
                        buf.push(',');
                    }
                    buf.push('\n');
                }
                buf.push_str(&"  ".repeat(indent));
                buf.push('}');
            }
        }
    }
    Ok(())
}

/// queue.json style: ASCII-escaping (Python json.dump default), indent 2, trailing newline.
pub fn write_json_ascii(path: &Path, data: &Value) -> Result<()> {
    let mut out = Vec::new();
    {
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"  ");
        let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
        serde::Serialize::serialize(data, &mut ser)?;
    }
    let s = String::from_utf8(out).context("queue.json utf8")?;
    // Match Python json.dump(ensure_ascii=True): non-ASCII → \uXXXX
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        let cp = ch as u32;
        if cp < 0x20 {
            match ch {
                '\n' => escaped.push('\n'),
                '\r' => escaped.push_str("\\r"),
                '\t' => escaped.push_str("\\t"),
                _ => escaped.push_str(&format!("\\u{cp:04x}")),
            }
        } else if cp > 0x7e {
            escaped.push_str(&format!("\\u{cp:04x}"));
        } else {
            escaped.push(ch);
        }
    }
    escaped.push('\n');
    fs::write(path, escaped)?;
    Ok(())
}

pub fn tickets(reg: &Registry) -> &[Value] {
    reg.get("tickets")
        .and_then(|t| t.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

pub fn tickets_mut(reg: &mut Registry) -> Result<&mut Vec<Value>> {
    reg.get_mut("tickets")
        .and_then(|t| t.as_array_mut())
        .context("registry.tickets missing")
}

pub fn ticket_by_id<'a>(reg: &'a Registry, tid: &str) -> Option<&'a Value> {
    tickets(reg)
        .iter()
        .find(|t| t.get("id").and_then(|i| i.as_str()) == Some(tid))
}

pub fn ticket_by_id_mut<'a>(reg: &'a mut Registry, tid: &str) -> Option<&'a mut Value> {
    tickets_mut(reg)
        .ok()?
        .iter_mut()
        .find(|t| t.get("id").and_then(|i| i.as_str()) == Some(tid))
}

pub fn str_field(t: &Value, key: &str) -> String {
    match t.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(v) => v.to_string().trim_matches('"').to_string(),
        None => String::new(),
    }
}

pub fn opt_str<'a>(t: &'a Value, key: &str) -> Option<&'a str> {
    t.get(key).and_then(|v| v.as_str())
}

pub fn order_or(t: &Value, default: i64) -> i64 {
    t.get("order").and_then(|o| o.as_i64()).unwrap_or(default)
}

pub fn order_truthy(t: &Value) -> bool {
    match t.get("order") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(false)) => false,
        Some(Value::Number(n)) => {
            n.as_i64().map(|i| i != 0).unwrap_or(true)
                || n.as_f64().map(|f| f != 0.0).unwrap_or(true)
        }
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// Python truthiness for required fields
pub fn is_truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().map(|i| i != 0).unwrap_or(true),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

pub fn ticket_sort_key(t: &Value) -> (i64, String) {
    let order = t.get("order").and_then(|o| o.as_i64()).unwrap_or(99999);
    let id = str_field(t, "id");
    (order, id)
}

pub fn slice_spec(t: &Value) -> String {
    let active = opt_str(t, "active_slice");
    let plan = t.get("slice_plan").and_then(|p| p.as_object());
    if let (Some(active), Some(plan)) = (active, plan) {
        if let Some(row) = plan.get(active) {
            if let Some(spec) = row.get("spec").and_then(|s| s.as_str()) {
                if !spec.is_empty() {
                    return spec.to_string();
                }
            }
        }
    }
    opt_str(t, "spec").unwrap_or("").to_string()
}

pub fn slice_executor(t: &Value) -> String {
    let active = opt_str(t, "active_slice");
    let plan = t.get("slice_plan").and_then(|p| p.as_object());
    if let (Some(active), Some(plan)) = (active, plan) {
        if let Some(row) = plan.get(active) {
            if let Some(ex) = row.get("executor").and_then(|e| e.as_str()) {
                return ex.to_string();
            }
            return opt_str(t, "executor").unwrap_or("claude-code").to_string();
        }
    }
    opt_str(t, "executor").unwrap_or("claude-code").to_string()
}

pub fn slice_targets(t: &Value) -> Vec<String> {
    let active = opt_str(t, "active_slice");
    let plan = t.get("slice_plan").and_then(|p| p.as_object());
    if let (Some(active), Some(plan)) = (active, plan) {
        if let Some(row) = plan.get(active) {
            if let Some(arr) = row.get("targets").and_then(|t| t.as_array()) {
                if !arr.is_empty() {
                    return arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
            return string_list(t, "targets")
                .or_else(|| Some(vec!["website".into()]))
                .unwrap();
        }
    }
    string_list(t, "targets").unwrap_or_else(|| vec!["website".into()])
}

pub fn string_list(t: &Value, key: &str) -> Option<Vec<String>> {
    t.get(key)?.as_array().map(|a| {
        a.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    })
}

pub fn shipped_slices(t: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(plan) = t.get("slice_plan").and_then(|p| p.as_object()) {
        for (sid, row) in plan {
            if row.get("status").and_then(|s| s.as_str()) == Some("shipped") {
                out.push(sid.clone());
            }
        }
    }
    out
}

pub fn slice_id_to_artifact_slug(slice_id: &str) -> String {
    let mut s = slice_id.trim().to_string();
    if s.to_uppercase().starts_with("T-") {
        s = s[2..].to_string();
    }
    format!("t{}", s.replace('.', "_").to_lowercase())
}

pub fn slice_handoff_path(t: &Value, slice_id: Option<&str>) -> String {
    let sid = slice_id
        .map(|s| s.to_string())
        .or_else(|| opt_str(t, "active_slice").map(|s| s.to_string()))
        .unwrap_or_else(|| str_field(t, "id"));
    let slug = slice_id_to_artifact_slug(&sid);
    format!(".ai/artifacts/{slug}_claude_code_handoff.md")
}
