//! Whole-folder load and save of untyped ticket files, and the id listings.
//!
//! **Role:** reads every parent file into a registry value, writes a registry value back as
//! parent and child files, and lists the ticket ids on disk or in a value.
//! **Position:** under the ticket file storage module; [`crate::registry::load_registry`] loads
//! untyped folders here and `save_registry` writes them back with [`save_toml_tree`].
//! **Signals & state:** none; reads and writes the ticket folder.
//! **Invariants:** a save deletes every `T-*.toml` the value does not name, so it only serves
//! untyped trees, never the typed live tree; a folder with no parent file refuses to load.

use super::*;
use crate::error::{Error, Result, ResultExt};

pub(super) fn child_ids_from_parent(ticket: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(plan) = ticket.get("slice_plan").and_then(Value::as_object) {
        for k in plan.keys() {
            if !ids.iter().any(|x| x == k) {
                ids.push(k.clone());
            }
        }
    }
    if let Some(slices) = ticket.get("slices").and_then(Value::as_array) {
        for s in slices {
            if let Some(id) = s.as_str()
                && !ids.iter().any(|x| x == id)
            {
                ids.push(id.to_string());
            }
        }
    }
    if let Some(slices) = ticket.get("children").and_then(Value::as_array) {
        for s in slices {
            if let Some(id) = s.as_str()
                && !ids.iter().any(|x| x == id)
            {
                ids.push(id.to_string());
            }
        }
    }
    ids
}

pub(super) fn child_doc(parent_id: &str, child_id: &str, parent: &Value) -> Value {
    let mut map = Map::new();
    map.insert("id".into(), Value::String(child_id.into()));
    map.insert("parent".into(), Value::String(parent_id.into()));
    if let Some(entry) = parent
        .get("slice_plan")
        .and_then(Value::as_object)
        .and_then(|p| p.get(child_id))
        && let Some(obj) = entry.as_object()
    {
        for (k, v) in obj {
            map.insert(k.clone(), v.clone());
        }
    }
    Value::Object(map)
}

/// Write the TOML tree for `registry` (uses `tickets[]` only; `next_id` is not stored): one file
/// per parent row, one per child its `slice_plan`, `slices` or `children` names, the root marker
/// when absent, and every other `T-*.toml` deleted.
///
/// # Errors
/// When `registry` has no `tickets` array, a row has no id, or a file cannot be written or
/// deleted.
pub fn save_toml_tree(root: &Path, registry: &Value) -> Result<()> {
    let dir = tickets_dir(root);
    fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let marker = root_marker_path(root);
    if !marker.is_file() {
        fs::write(&marker, "# ticket-registry root marker\n")
            .with_context(|| format!("write {}", marker.display()))?;
    }

    let tickets = registry
        .get("tickets")
        .and_then(Value::as_array)
        .context("registry.tickets missing")?;

    let mut desired: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, ticket) in tickets.iter().enumerate() {
        let id = ticket
            .get("id")
            .and_then(Value::as_str)
            .with_context(|| format!("ticket[{i}] missing id"))?;
        let path = parent_toml_path(root, &ticket_model::TicketId::new(id));
        fs::write(&path, ticket_to_toml_string(ticket, i as i64)?)
            .with_context(|| format!("write {}", path.display()))?;
        desired.insert(format!("{id}.toml"));

        for cid in child_ids_from_parent(ticket) {
            let child = child_doc(id, &cid, ticket);
            let cpath = dir.join(format!("{cid}.toml"));
            fs::write(&cpath, ticket_to_toml_string(&child, 0)?)
                .with_context(|| format!("write {}", cpath.display()))?;
            desired.insert(format!("{cid}.toml"));
        }
    }

    for ent in fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("T-") && name.ends_with(".toml") && !desired.contains(name.as_ref()) {
            fs::remove_file(ent.path())
                .with_context(|| format!("remove stale {}", ent.path().display()))?;
        }
    }
    Ok(())
}

pub(super) fn is_parent_toml_name(name: &str) -> bool {
    name.starts_with("T-")
        && name.ends_with(".toml")
        && ticket_model::TicketId::new(name.trim_end_matches(".toml")).is_parent()
}

/// The registry value of the untyped folder under `root`: `next_id` and the parent rows in
/// `__ord` order.
///
/// # Errors
/// When the folder or a file cannot be read, a file is not TOML, or no parent file exists.
pub fn load_toml_tree(root: &Path) -> Result<Value> {
    let dir = tickets_dir(root);
    let mut loaded: Vec<(i64, Value)> = Vec::new();
    let mut found = false;
    for ent in fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !is_parent_toml_name(&name) {
            continue;
        }
        found = true;
        let text = fs::read_to_string(ent.path())
            .with_context(|| format!("read {}", ent.path().display()))?;
        loaded.push(ticket_from_toml_str(&text)?);
    }
    if !found {
        return Err(Error::msg(format!(
            "no parent T-*.toml files in {}",
            dir.display()
        )));
    }
    loaded.sort_by_key(|(ord, _)| *ord);
    let tickets: Vec<Value> = loaded.into_iter().map(|(_, t)| t).collect();
    let next_id = derive_next_id(&tickets);
    let mut root_obj = Map::new();
    root_obj.insert("next_id".into(), Value::Number(next_id.into()));
    root_obj.insert("tickets".into(), Value::Array(tickets));
    Ok(Value::Object(root_obj))
}

/// All on-disk ticket ids (parents + children), in [`ticket_model::store::ticket_id_order_key`]
/// order. Used by the no-ticket-lost proof.
///
/// # Errors
/// When the ticket folder cannot be read.
#[allow(dead_code)]
pub fn on_disk_ids(root: &Path) -> Result<Vec<String>> {
    let dir = tickets_dir(root);
    let mut ids = Vec::new();
    for ent in fs::read_dir(&dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("T-") && name.ends_with(".toml") {
            ids.push(name.trim_end_matches(".toml").to_string());
        }
    }
    ids.sort_by(|a, b| {
        ticket_model::store::ticket_id_order_key(a.as_str())
            .cmp(&ticket_model::store::ticket_id_order_key(b.as_str()))
    });
    Ok(ids)
}

/// The ids a registry value names: the parent row ids, and every id (parents with the children
/// their rows list).
#[allow(dead_code)]
pub fn corpus_ids(
    registry: &Value,
) -> (
    std::collections::BTreeSet<String>,
    std::collections::BTreeSet<String>,
) {
    use std::collections::BTreeSet;
    let mut parents = BTreeSet::new();
    let mut all = BTreeSet::new();
    if let Some(arr) = registry.get("tickets").and_then(Value::as_array) {
        for t in arr {
            if let Some(id) = t.get("id").and_then(Value::as_str) {
                parents.insert(id.to_string());
                all.insert(id.to_string());
                for cid in child_ids_from_parent(t) {
                    all.insert(cid);
                }
            }
        }
    }
    (parents, all)
}
