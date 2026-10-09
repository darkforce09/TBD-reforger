#![allow(dead_code)] // the JSON write path stays compiled so the tests of its refusal can call it
//! The parents-only JSON projection of the typed ticket files.
//!
//! **Role:** builds the [`crate::registry::Registry`] value from typed ticket files: one row per
//! parent ticket, each program carrying a synthesised `slice_plan` read from its children's
//! files, with `children` mirrored as `slices` and `active` as `active_slice`.
//! **Position:** called by [`crate::registry::load_registry`] when the ticket files carry
//! `kind =`; the read verbs (brief, show, get, the queue view), `ticket sync` and `ticket check`
//! consume the value instead of the typed corpus.
//! **Signals & state:** none; reads `.ai/tickets/` on every call.
//! **Invariants:** children never appear as rows, only inside their parent's `slice_plan`; rows
//! sort by `order` (absent as 99999) then by id; a tree with no typed parent file refuses.
//! `crate::ops` over `ticket_model::Corpus` owns every mutation: [`save_tree`] stays compiled only
//! so the tests that assert [`crate::registry::save_registry`] refuses a typed tree can call it.

use crate::error::{Error, Result, ResultExt};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
use ticket_model::{
    Domain, ScopeV2, Ticket, TicketFile, TicketId, parse_ticket_toml, render_ticket_toml,
};

/// Whether ticket file text is in the typed form: some line starts with `kind =`.
pub fn is_phase2_text(text: &str) -> bool {
    text.lines().any(|l| l.starts_with("kind ="))
}

/// Whether the ticket folder under `root` holds typed files, judged by the first `T-*.toml`
/// the folder listing yields; `false` when the folder is missing or holds no ticket file.
pub fn tree_is_phase2(root: &Path) -> bool {
    let dir = crate::registry::ticket_file_storage::tickets_dir(root);
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return false;
    };
    for ent in rd.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("T-")
            && name.ends_with(".toml")
            && let Ok(text) = std::fs::read_to_string(ent.path())
        {
            return is_phase2_text(&text);
        }
    }
    false
}

/// The build targets a child's `slice_plan` entry carries, derived from its scope domain:
/// `queue.json` and the `slice_plan` readers speak the four-value target set, so
/// website→website, mod→mod, schema→shared, engine and repo→root.
fn targets_from_scope(scope: &ScopeV2) -> Vec<String> {
    vec![
        match scope.domain {
            Domain::Website => "website",
            Domain::Mod => "mod",
            Domain::Schema => "shared",
            Domain::Engine | Domain::Repo => "root",
        }
        .into(),
    ]
}

fn attach_slice_plan(dir: &Path, t: &Ticket, val: &mut Value) {
    let Ticket::Program(p) = t else {
        return;
    };
    let mut plan = serde_json::Map::new();
    for cid in &p.children {
        let path = dir.join(format!("{cid}.toml"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(child) = parse_ticket_toml(&text) else {
            continue;
        };
        let (executor, status, spec, notes, shipped_at, targets) = match &child {
            Ticket::Work(w) => (
                w.executor.clone(),
                w.status.name().as_str().to_string(),
                w.spec.clone(),
                w.notes.clone(),
                w.shipped_at.clone(),
                targets_from_scope(&w.scope),
            ),
            Ticket::Program(cp) => (
                cp.executor.clone(),
                cp.status.name().as_str().to_string(),
                cp.spec.clone(),
                cp.notes.clone(),
                None,
                vec!["root".into()],
            ),
        };
        let mut entry = serde_json::Map::new();
        entry.insert("targets".into(), serde_json::json!(targets));
        entry.insert(
            "executor".into(),
            serde_json::Value::String(executor.unwrap_or_else(|| "claude-code".into())),
        );
        entry.insert("status".into(), serde_json::Value::String(status));
        if let Some(s) = spec.filter(|s| !s.is_empty()) {
            entry.insert("spec".into(), serde_json::Value::String(s));
        }
        if let Some(n) = notes.filter(|s| !s.is_empty()) {
            entry.insert("notes".into(), serde_json::Value::String(n));
        }
        if let Some(s) = shipped_at.filter(|s| !s.is_empty()) {
            entry.insert("shipped_at".into(), serde_json::Value::String(s));
        }
        plan.insert(cid.clone(), serde_json::Value::Object(entry));
    }
    if let Some(obj) = val.as_object_mut()
        && !plan.is_empty()
    {
        obj.insert("slice_plan".into(), serde_json::Value::Object(plan));
    }
}

/// One ticket as a registry row: its [`TicketFile`] form as JSON, with `children` mirrored as
/// `slices` and `active` as `active_slice` for the readers that use those names.
pub fn ticket_to_value(t: &Ticket) -> Value {
    let file = TicketFile::from_ticket(t);
    let mut v = serde_json::to_value(&file).expect("ticket file json");
    if let Some(obj) = v.as_object_mut() {
        if let Some(children) = obj.get("children").cloned() {
            obj.insert("slices".into(), children);
        }
        if let Some(active) = obj.get("active").cloned() {
            obj.insert("active_slice".into(), active);
        }
    }
    v
}

/// A registry row back as a typed [`Ticket`]; only the tests call it, against the whole loaded
/// registry, so the mirrored-key case stays handled on the read path.
///
/// [`ticket_to_value`] mirrors `children` as `slices` and `active` as `active_slice`, and
/// [`TicketFile`] declares those names as serde aliases, so a value carrying both spellings would
/// fail as a duplicate field. The mirror is dropped when the canonical key is present; a value
/// carrying only the aliased spelling deserialises through the alias.
///
/// # Errors
/// When the value does not deserialise into a [`TicketFile`] or that file is not a legal ticket.
pub fn value_to_ticket(v: &Value) -> Result<Ticket> {
    let mut v = v.clone();
    if let Some(obj) = v.as_object_mut() {
        if obj.contains_key("children") {
            obj.remove("slices");
        }
        if obj.contains_key("active") {
            obj.remove("active_slice");
        }
    }
    let file: TicketFile = serde_json::from_value(v).context("ticket value → file")?;
    file.into_ticket().map_err(Error::msg)
}

/// The registry value of the typed tree under `root`: `next_id` and the parent rows in
/// `order`-then-id order, each program with its synthesised `slice_plan`.
///
/// # Errors
/// When the folder cannot be read, a typed parent file does not parse (the message names the
/// file), or no typed parent file exists.
pub fn load_phase2_tree(root: &Path) -> Result<Value> {
    let dir = crate::registry::ticket_file_storage::tickets_dir(root);
    let mut tickets = Vec::new();
    let mut rows = Vec::new();
    for ent in std::fs::read_dir(&dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("T-") || !name.ends_with(".toml") {
            continue;
        }
        if !TicketId::new(name.trim_end_matches(".toml")).is_parent() {
            continue;
        }
        let text = std::fs::read_to_string(ent.path())?;
        if !is_phase2_text(&text) {
            continue;
        }
        let t = parse_ticket_toml(&text)
            .map_err(|e| Error::msg(format!("{}: {e}", ent.path().display())))?;
        let ord = t.status().order().unwrap_or(99_999);
        let mut val = ticket_to_value(&t);
        attach_slice_plan(&dir, &t, &mut val);
        rows.push((ord, t.id().to_string(), val));
    }
    if rows.is_empty() {
        return Err(Error::msg(format!(
            "no phase-2 parent tickets in {}",
            dir.display()
        )));
    }
    rows.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| {
            ticket_model::store::ticket_id_order_key(a.1.as_str())
                .cmp(&ticket_model::store::ticket_id_order_key(b.1.as_str()))
        })
    });
    for (_, _, v) in rows {
        tickets.push(v);
    }
    let next_id = crate::registry::ticket_file_storage::derive_next_id(&tickets);
    Ok(serde_json::json!({
        "next_id": next_id,
        "tickets": tickets,
    }))
}

/// Write every parent row of `registry` as a typed ticket file, keep the child files its
/// programs list, and delete every other `T-*.toml` in the folder.
///
/// No command calls it: mutations go through `crate::ops` and `Corpus::write_back`, which write
/// and delete only the ids an operation names, and [`crate::registry::save_registry`] refuses a
/// typed tree before reaching here. The stale-file pass below is why: a mangled `children` list
/// would delete every child file it no longer names.
///
/// # Errors
/// When `registry` has no `tickets` array, a row is not a legal ticket, or a file cannot be
/// written or deleted.
pub fn save_tree(root: &Path, registry: &Value) -> Result<()> {
    let dir = crate::registry::ticket_file_storage::tickets_dir(root);
    std::fs::create_dir_all(&dir)?;
    let tickets = registry
        .get("tickets")
        .and_then(Value::as_array)
        .context("tickets")?;
    let mut desired: BTreeSet<String> = BTreeSet::new();
    for t in tickets {
        let ticket = value_to_ticket(t).with_context(|| {
            format!(
                "save {}",
                t.get("id").and_then(Value::as_str).unwrap_or("?")
            )
        })?;
        let id = ticket.id().to_string();
        let path = dir.join(format!("{id}.toml"));
        std::fs::write(&path, render_ticket_toml(&ticket).map_err(Error::msg)?)?;
        desired.insert(format!("{id}.toml"));
        let kids = match &ticket {
            Ticket::Program(p) => p.children.clone(),
            Ticket::Work(_) => vec![],
        };
        for cid in kids {
            desired.insert(format!("{cid}.toml"));
        }
    }
    for ent in std::fs::read_dir(&dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("T-") && name.ends_with(".toml") && !desired.contains(name.as_ref()) {
            std::fs::remove_file(ent.path())?;
        }
    }
    Ok(())
}
