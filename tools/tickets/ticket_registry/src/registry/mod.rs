//! The ticket registry as one JSON value, and the helpers that read its rows.
//!
//! **Role:** loads every parent ticket into one `{ "next_id", "tickets" }` value
//! ([`load_registry`]), writes `queue.json` ([`write_json_ascii`]) and reads single fields of a
//! row, falling back from the active slice's `slice_plan` entry to the ticket's own field.
//! **Position:** reads `.ai/tickets/` through [`typed_projection`] (typed files) or
//! [`ticket_file_storage`] (untyped files); `crate::verbs`, `crate::sync`, `crate::validation`
//! and the xtask `ticket`, `platform` and `mod` command groups consume the value.
//! **Signals & state:** none; every call reads the files or the value it is given.
//! **Invariants:** a folder that holds no registry refuses instead of answering an empty one;
//! [`save_registry`] refuses a typed tree, so the typed operations stay the one writer of ticket
//! files.

use crate::error::{Error, Result, ResultExt};

use serde_json::Value;
use std::fs;
use std::path::Path;
use ticket_model::TicketId;

use repository_layout::TICKETS_DIR;

/// The registry as one JSON object: `next_id` and the `tickets` array of parent rows, in
/// [`ticket_sort_key`] order.
pub type Registry = Value;

/// The whole registry as one `Value`, however the ticket files on disk are shaped.
///
/// A tree whose files carry the typed schema projects through [`typed_projection`]; an
/// untyped tree loads through [`ticket_file_storage`]. Neither present means the caller is not
/// looking at a registry, which refuses rather than yielding an empty one: an empty registry
/// reads as "no tickets" to every consumer, and that is a silent wrong answer.
pub fn load_registry(root: &Path) -> Result<Registry> {
    if crate::registry::typed_projection::tree_is_phase2(root) {
        return crate::registry::typed_projection::load_phase2_tree(root);
    }
    let tickets_dir = root.join(TICKETS_DIR);
    let has_toml = repository_root::is_repository_root(root)
        || tickets_dir.read_dir().ok().is_some_and(|rd| {
            rd.filter_map(|e| e.ok()).any(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.starts_with("T-") && n.ends_with(".toml")
            })
        });
    if has_toml {
        return crate::registry::ticket_file_storage::load_toml_tree(root);
    }
    Err(Error::msg(format!(
        "no ticket registry ({TICKETS_DIR}/ROOT or {TICKETS_DIR}/T-*.toml) under {}",
        root.display()
    )))
}

/// Write a whole registry back as ticket files — and REFUSE to do so on a typed tree.
///
/// The live tree is typed, and its one writer is `crate::ops` over [`ticket_model::Corpus`], which
/// touches exactly the files an operation names. This whole-tree writer would be a second one,
/// and a second writer is how a mangled `children` list once erased child files. The refusal is
/// the point.
#[allow(dead_code)]
pub fn save_registry(root: &Path, data: &Registry) -> Result<()> {
    if crate::registry::typed_projection::tree_is_phase2(root) {
        return Err(Error::msg(
            "save_registry on a typed tree: ticket mutations go through the typed ops, which \
             write one file per changed ticket",
        ));
    }
    crate::registry::ticket_file_storage::save_toml_tree(root, data)
}

/// `data` as JSON text with two-space indentation and a trailing newline, keeping every
/// non-ASCII character as itself and escaping only quotes, backslashes and control characters.
#[allow(dead_code)]
pub fn format_json_unicode_preserve(data: &Value) -> Result<String> {
    let mut buf = String::new();
    write_value(&mut buf, data, 0)?;
    buf.push('\n');
    Ok(buf)
}

#[allow(dead_code)]
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

/// Write `data` to `path` in the `queue.json` form: two-space indentation, a trailing newline,
/// and every character outside printable ASCII written as a `\uXXXX` escape.
///
/// # Errors
/// When the value cannot be serialised or the file cannot be written.
pub fn write_json_ascii(path: &Path, data: &Value) -> Result<()> {
    let mut out = Vec::new();
    {
        let formatter = serde_json::ser::PrettyFormatter::with_indent(b"  ");
        let mut ser = serde_json::Serializer::with_formatter(&mut out, formatter);
        serde::Serialize::serialize(data, &mut ser)?;
    }
    let s = String::from_utf8(out).context("queue.json utf8")?;
    // Every code point outside printable ASCII becomes a `\uXXXX` escape.
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

/// The `tickets` rows of `reg`, or an empty slice when the value has no `tickets` array.
pub fn tickets(reg: &Registry) -> &[Value] {
    reg.get("tickets")
        .and_then(|t| t.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

/// The row whose `id` is `id`, or `None` when no row carries it.
pub fn ticket_by_id<'a>(reg: &'a Registry, id: &TicketId) -> Option<&'a Value> {
    tickets(reg)
        .iter()
        .find(|t| t.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
}

/// The field `key` of a row as text: a string as it stands, any other value in its JSON
/// spelling without surrounding quotes, and an empty string when the field is absent.
pub fn str_field(t: &Value, key: &str) -> String {
    match t.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(v) => v.to_string().trim_matches('"').to_string(),
        None => String::new(),
    }
}

/// The field `key` of a row when it is a string, or `None` when it is absent or not a string.
pub fn opt_str<'a>(t: &'a Value, key: &str) -> Option<&'a str> {
    t.get(key).and_then(|v| v.as_str())
}

/// The integer `order` of a row, or `default` when it is absent or not an integer.
pub fn order_or(t: &Value, default: i64) -> i64 {
    t.get("order").and_then(|o| o.as_i64()).unwrap_or(default)
}

/// Whether a row carries a set `order`: absent, `null`, `false`, zero and the empty string are
/// unset; any other value is set.
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

/// Whether a required field holds a value: absent, `null`, `false`, zero, the empty string and
/// an empty array or object do not; anything else does.
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

/// Sort key for registry rows: `order` ascending, with an absent order sorting as 99999, then
/// the id in [`ticket_model::store::ticket_id_order_key`] order.
pub fn ticket_sort_key(t: &Value) -> (i64, (u64, String)) {
    let order = t.get("order").and_then(|o| o.as_i64()).unwrap_or(99999);
    (
        order,
        ticket_model::store::ticket_id_order_key(str_field(t, "id")),
    )
}

/// The spec path of a row: the active slice's `slice_plan` spec when it is non-empty, else the
/// ticket's own `spec`, else an empty string.
pub fn slice_spec(t: &Value) -> String {
    let active = opt_str(t, "active_slice");
    let plan = t.get("slice_plan").and_then(|p| p.as_object());
    if let (Some(active), Some(plan)) = (active, plan)
        && let Some(row) = plan.get(active)
        && let Some(spec) = row.get("spec").and_then(|s| s.as_str())
        && !spec.is_empty()
    {
        return spec.to_string();
    }
    opt_str(t, "spec").unwrap_or("").to_string()
}

/// The executor of a row: the active slice's `slice_plan` executor, else the ticket's own
/// `executor`, else `claude-code`.
pub fn slice_executor(t: &Value) -> String {
    let active = opt_str(t, "active_slice");
    let plan = t.get("slice_plan").and_then(|p| p.as_object());
    if let (Some(active), Some(plan)) = (active, plan)
        && let Some(row) = plan.get(active)
    {
        if let Some(ex) = row.get("executor").and_then(|e| e.as_str()) {
            return ex.to_string();
        }
        return opt_str(t, "executor").unwrap_or("claude-code").to_string();
    }
    opt_str(t, "executor").unwrap_or("claude-code").to_string()
}

/// The build targets of a row: the active slice's non-empty `slice_plan` targets, else the
/// ticket's own `targets`, else `["website"]`.
pub fn slice_targets(t: &Value) -> Vec<String> {
    let active = opt_str(t, "active_slice");
    let plan = t.get("slice_plan").and_then(|p| p.as_object());
    if let (Some(active), Some(plan)) = (active, plan)
        && let Some(row) = plan.get(active)
    {
        if let Some(arr) = row.get("targets").and_then(|t| t.as_array())
            && !arr.is_empty()
        {
            return arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        return string_list(t, "targets").unwrap_or_else(|| vec!["website".into()]);
    }
    string_list(t, "targets").unwrap_or_else(|| vec!["website".into()])
}

/// The string items of the array field `key`, skipping non-string items, or `None` when the
/// field is absent or not an array.
pub fn string_list(t: &Value, key: &str) -> Option<Vec<String>> {
    t.get(key)?.as_array().map(|a| {
        a.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    })
}

/// The ids of the `slice_plan` entries whose `status` is `shipped`, in plan order; empty when
/// the row has no plan.
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

/// The artifact folder slug of a slice id: the `T-` prefix dropped, dots turned into
/// underscores, lower-cased and prefixed with `t`.
pub fn slice_id_to_artifact_slug(slice_id: &TicketId) -> String {
    let mut s = slice_id.as_str().trim().to_string();
    if s.to_uppercase().starts_with("T-") {
        s = s[2..].to_string();
    }
    format!("t{}", s.replace('.', "_").to_lowercase())
}

/// The handoff document path of a slice: `slice_id` when given, else the row's
/// `active_slice`, else the row's own id, mapped through
/// [`slice_id_to_artifact_slug`] and [`ticket_model::repository::handoff_doc`].
pub fn slice_handoff_path(t: &Value, slice_id: Option<&TicketId>) -> String {
    let sid = slice_id
        .cloned()
        .or_else(|| opt_str(t, "active_slice").map(TicketId::new))
        .unwrap_or_else(|| TicketId::new(str_field(t, "id")));
    let slug = slice_id_to_artifact_slug(&sid);
    ticket_model::repository::handoff_doc(&slug)
}

pub mod ticket_file_storage;
pub mod ticket_status_history;
pub mod typed_projection;

pub mod shipping_status;
pub mod ticket_titles;
