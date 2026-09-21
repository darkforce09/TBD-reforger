#![allow(dead_code)] // the Value write path stays compiled so its refusal pins can run
//! Parents-only `Value` projection of the ticket tree.
//!
//! The read verbs that render tickets — brief, show, get, sync, the queue view — consume a
//! `Value` rather than the typed corpus, and this module builds it. Children are deliberately
//! absent: the projection answers "what does the registry look like", and a child's fields live
//! inside its parent's `slice_plan`.
//!
//! The Value WRITE path beside it is not the writer. `crate::ops` over `crate::Corpus` owns every
//! mutation; [`save_tree`] stays compiled only so the pins that assert it refuses a typed tree
//! have something to call.

use crate::{Domain, ScopeV2, Ticket, TicketFile, parse_ticket_toml, render_ticket_toml};
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub fn is_phase2_text(text: &str) -> bool {
    text.lines().any(|l| l.starts_with("kind ="))
}

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

/// Legacy `targets` synthesis from the v2 scope — the queue.json / slice_plan
/// consumers still speak the 4-value target set. Same outputs as the v1 mapping:
/// website→website, mod→mod, schema→shared, engine/repo→root.
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

/// MIGRATION/TEST-ONLY since T-916.2 (the file-top `allow(dead_code)` pattern): the registry
/// mutators write through `crate::ops` + `Corpus::write_back` now, so no mutator path
/// reaches this Value→typed conversion anymore — `save_registry` refuses phase-2 trees, and
/// `mutators_never_reach_the_value_writer_pin` keeps both facts pinned. It stays compiled
/// because the T-912.2 alias-clash regression pin below still exercises it against the whole
/// loaded registry: the mirrored-keys condition must STAY representable-and-handled on the
/// read path even though no writer consumes the result.
pub fn value_to_ticket(v: &Value) -> Result<Ticket> {
    // T-912.2 fix for a T-911.2 round-trip regression that broke EVERY registry mutator:
    // `ticket_to_value` mirrors `children` → `slices` and `active` → `active_slice` for the
    // legacy readers (`ticket advance-slice` reads `slices`, `ticket brief` reads
    // `active_slice`), and `TicketFile` declares those legacy names as serde ALIASES — so a
    // value carrying both spellings deserialized as `duplicate field \`children\`` and
    // `ticket ship`/`set-status`/`mark-ready`/`reorder` all refused to save (measured at the
    // T-912.1 tip: `ticket ship T-905` → `save T-067: ticket value → file: duplicate field
    // \`children\``). Strip the mirror when the canonical key is present; a value carrying
    // ONLY the legacy spelling still lands through the alias.
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
    file.into_ticket().map_err(anyhow::Error::msg)
}

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
        if !crate::registry::ticket_file_storage::is_parent_id(name.trim_end_matches(".toml")) {
            continue;
        }
        let text = std::fs::read_to_string(ent.path())?;
        if !is_phase2_text(&text) {
            continue;
        }
        let t = parse_ticket_toml(&text)
            .map_err(|e| anyhow::anyhow!("{}: {e}", ent.path().display()))?;
        let ord = t.status().order().unwrap_or(99_999);
        let mut val = ticket_to_value(&t);
        attach_slice_plan(&dir, &t, &mut val);
        rows.push((ord, t.id().to_string(), val));
    }
    if rows.is_empty() {
        bail!("no phase-2 parent tickets in {}", dir.display());
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    for (_, _, v) in rows {
        tickets.push(v);
    }
    let next_id = crate::registry::ticket_file_storage::derive_next_id(&tickets);
    Ok(serde_json::json!({
        "next_id": next_id,
        "tickets": tickets,
    }))
}

/// MIGRATION/TEST-ONLY since T-916.2 — the retired Value write path. Live mutations go through
/// `crate::ops` + `Corpus::write_back` (surgical per-file temp+rename writes); nothing
/// live calls this, and `registry::save_registry` refuses phase-2 trees so it cannot be
/// reached by accident. Note what retiring it kills: the final stale-file pass below deletes
/// EVERY `T-*.toml` not in {parents ∪ children[]}, which is how a mangled `children[]` once
/// cascade-deleted child files (the hazard class t915_ticketboard_design.md Decisions #3
/// names); the typed path deletes only ids an op explicitly returns, and
/// `check_children_integrity` now surfaces the stray files this pass used to erase silently.
///
/// Write encoding-C parents from the in-memory registry. Existing child files are kept.
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
        std::fs::write(
            &path,
            render_ticket_toml(&ticket).map_err(anyhow::Error::msg)?,
        )?;
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

#[cfg(test)]
#[path = "tests/typed_projection/mod.rs"]
mod tests;
