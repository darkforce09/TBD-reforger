#![allow(dead_code)] // save_tree/value_to_ticket stay compiled for the write-path pins
//! T-911.2 phase-2 tree helpers: the parents-only `Value` projection the read verbs
//! (brief/show/get/sync/queue.json) still consume, plus the retired Value write path
//! kept only for its regression pins.
//!
//! T-917.2 NOTE — the phase-1→phase-2 one-shot migrator (`migrate_live_tree`,
//! `map_value`, `infer_scope`, `override_scope`, the per-id override table, the
//! T-090-family ready fills and the T-674/T-675 synthetic children) was DELETED at the
//! schema-v2 cutover, not cfg-demoted: it compiled against the v1 `Scope` enum tree,
//! which no longer exists — compile-time physics, the same force that made the cutover
//! a single commit. Its successor (v1 Value → v2 Value) lives in
//! `xtask/src/migrate_v2.rs`, retained one-shot for corroboration like
//! `wave/legacy_plan.rs`.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
use tbd_tickets::{Domain, ScopeV2, Ticket, TicketFile, parse_ticket_toml, render_ticket_toml};

pub fn is_phase2_text(text: &str) -> bool {
    text.lines().any(|l| l.starts_with("kind ="))
}

pub fn tree_is_phase2(root: &Path) -> bool {
    let dir = crate::tickets_store::tickets_dir(root);
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return false;
    };
    for ent in rd.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("T-") && name.ends_with(".toml") {
            if let Ok(text) = std::fs::read_to_string(ent.path()) {
                return is_phase2_text(&text);
            }
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
    if let Some(obj) = val.as_object_mut() {
        if !plan.is_empty() {
            obj.insert("slice_plan".into(), serde_json::Value::Object(plan));
        }
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
/// mutators write through `tbd_tickets::ops` + `Corpus::write_back` now, so no mutator path
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
    let dir = crate::tickets_store::tickets_dir(root);
    let mut tickets = Vec::new();
    let mut rows = Vec::new();
    for ent in std::fs::read_dir(&dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("T-") || !name.ends_with(".toml") {
            continue;
        }
        if !crate::tickets_store::is_parent_id(name.trim_end_matches(".toml")) {
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
    let next_id = crate::tickets_store::derive_next_id(&tickets);
    Ok(serde_json::json!({
        "next_id": next_id,
        "tickets": tickets,
    }))
}

/// MIGRATION/TEST-ONLY since T-916.2 — the retired Value write path. Live mutations go through
/// `tbd_tickets::ops` + `Corpus::write_back` (surgical per-file temp+rename writes); nothing
/// live calls this, and `registry::save_registry` refuses phase-2 trees so it cannot be
/// reached by accident. Note what retiring it kills: the final stale-file pass below deletes
/// EVERY `T-*.toml` not in {parents ∪ children[]}, which is how a mangled `children[]` once
/// cascade-deleted child files (the hazard class t915_ticketboard_design.md Decisions #3
/// names); the typed path deletes only ids an op explicitly returns, and
/// `check_children_integrity` now surfaces the stray files this pass used to erase silently.
///
/// Write encoding-C parents from the in-memory registry. Existing child files are kept.
pub fn save_tree(root: &Path, registry: &Value) -> Result<()> {
    let dir = crate::tickets_store::tickets_dir(root);
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
mod tests {
    use super::*;
    use tbd_tickets::Status;

    fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn parse_file(root: &Path, id: &str) -> Ticket {
        let text = std::fs::read_to_string(root.join(format!(".ai/tickets/{id}.toml"))).unwrap();
        parse_ticket_toml(&text).unwrap_or_else(|e| panic!("{id}: {e}"))
    }

    #[test]
    fn ready_prose_on_t090_family() {
        let root = repo_root();
        if !tree_is_phase2(&root) {
            return;
        }
        for id in ["T-090", "T-090.4", "T-090.6", "T-090.7", "T-090.9"] {
            let t = parse_file(&root, id);
            let (spec, story, acc) = match t.status() {
                Status::Ready {
                    spec,
                    user_story,
                    acceptance,
                    ..
                }
                | Status::Running {
                    spec,
                    user_story,
                    acceptance,
                    ..
                }
                | Status::Review {
                    spec,
                    user_story,
                    acceptance,
                    ..
                } => (spec, user_story, acceptance),
                other => panic!("{id} status {:?} is not ready-class", other.name()),
            };
            assert!(!spec.trim().is_empty(), "{id} spec");
            assert!(!story.trim().is_empty(), "{id} user_story");
            assert!(acc.iter().any(|s| !s.trim().is_empty()), "{id} acceptance");
        }
    }

    #[test]
    fn t159_23_shipped_at_pin() {
        let root = repo_root();
        if !tree_is_phase2(&root) {
            return;
        }
        let t = parse_file(&root, "T-159.23");
        match t.status() {
            Status::Shipped { shipped_at, .. } => {
                assert_eq!(shipped_at.as_deref(), Some("69dc5da5"));
            }
            other => panic!("T-159.23 status {:?}", other.name()),
        }
        match t {
            Ticket::Work(w) => assert_eq!(w.shipped_at.as_deref(), Some("69dc5da5")),
            Ticket::Program(_) => panic!("T-159.23 must be Work"),
        }
    }

    #[test]
    fn mapper_minted_t674_t675_children() {
        let root = repo_root();
        if !tree_is_phase2(&root) {
            return;
        }
        for id in ["T-674.1", "T-674.2", "T-675.1", "T-675.2"] {
            let t = parse_file(&root, id);
            assert!(matches!(t, Ticket::Work(_)), "{id} must be Work");
            let parent = match &t {
                Ticket::Work(w) => w.parent.clone(),
                Ticket::Program(_) => None,
            };
            let want = if id.starts_with("T-674") {
                "T-674"
            } else {
                "T-675"
            };
            assert_eq!(parent.as_deref(), Some(want), "{id} parent");
        }
        for pid in ["T-674", "T-675"] {
            match parse_file(&root, pid) {
                Ticket::Program(p) => {
                    assert!(p.children.iter().any(|c| c == &format!("{pid}.1")));
                    assert!(p.children.iter().any(|c| c == &format!("{pid}.2")));
                }
                Ticket::Work(_) => panic!("{pid} must be Program"),
            }
        }
    }

    /// T-090.6 keeps its engine scope through the v2 cutover (the old per-id override
    /// table put it there; the v2 migrator maps `[scope.engine]` → domain engine).
    #[test]
    fn t090_6_is_engine_scope() {
        let root = repo_root();
        if !tree_is_phase2(&root) {
            return;
        }
        match parse_file(&root, "T-090.6") {
            Ticket::Work(w) => {
                assert_eq!(
                    w.scope.domain,
                    Domain::Engine,
                    "T-090.6 scope {:?}",
                    w.scope
                );
            }
            Ticket::Program(_) => panic!("T-090.6 must be Work"),
        }
    }

    /// T-917.2: the target synthesis over v2 scopes keeps the exact v1 outputs.
    #[test]
    fn targets_from_scope_v2_outputs() {
        let scope = |domain| ScopeV2 {
            domain,
            layer: "x".into(),
            component: None,
            surface: vec![],
        };
        assert_eq!(targets_from_scope(&scope(Domain::Website)), vec!["website"]);
        assert_eq!(targets_from_scope(&scope(Domain::Mod)), vec!["mod"]);
        assert_eq!(targets_from_scope(&scope(Domain::Schema)), vec!["shared"]);
        assert_eq!(targets_from_scope(&scope(Domain::Engine)), vec!["root"]);
        assert_eq!(targets_from_scope(&scope(Domain::Repo)), vec!["root"]);
    }

    /// T-912.2 regression pin, RETARGETED by T-916.2: `ticket_to_value` mirrors
    /// `children`/`active` into their legacy spellings, and `value_to_ticket` must accept its
    /// own output — the alias-vs-mirror clash made `duplicate field \`children\`` out of every
    /// loaded program and broke every registry mutator (`ticket ship T-905` → `save T-067`
    /// refuse, measured at the T-912.1 tip). Since T-916.2 no MUTATOR reaches
    /// `value_to_ticket` (see `mutators_never_reach_the_value_writer_pin`), but the mirrored
    /// Value is still what brief/show/get/sync/queue.json consume — this pin keeps the alias
    /// class dead on that read surface.
    #[test]
    fn value_to_ticket_accepts_ticket_to_value_output() {
        let root = repo_root();
        if !tree_is_phase2(&root) {
            return;
        }
        // T-067 is the program the live failure named; round-trip the whole loaded registry so
        // any ticket whose value carries mirrored keys is covered, not just one.
        let reg = load_phase2_tree(&root).expect("load phase2 tree");
        for t in reg["tickets"].as_array().expect("tickets") {
            let id = t["id"].as_str().unwrap_or("?");
            let back = value_to_ticket(t).unwrap_or_else(|e| panic!("{id}: {e:#}"));
            assert_eq!(back.id(), id);
        }
    }

    /// T-916.2 — the write path is the TYPED one. Two facts, pinned together:
    ///
    /// 1. `registry::save_registry` REFUSES a phase-2 tree (in-memory probe against the live
    ///    root — nothing is written on the refusal path), so `save_tree` / `value_to_ticket`
    ///    are unreachable as writers even if a caller sneaks back;
    /// 2. `cmds.rs` — the mutator surface — no longer names `save_registry` at all: every
    ///    verb writes through `tbd_tickets::ops` + `Corpus::write_back`. Needle assembled at
    ///    runtime (the T-912.1 tripwire trick) so this test's own source cannot satisfy it.
    #[test]
    fn mutators_never_reach_the_value_writer_pin() {
        let root = repo_root();
        if !tree_is_phase2(&root) {
            return;
        }
        let reg = crate::registry::load_registry(&root).expect("load live registry");
        let err = crate::registry::save_registry(&root, &reg)
            .expect_err("save_registry must refuse a phase-2 tree");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("typed ops") && msg.contains("migration/test-only"),
            "refusal must name the typed path: {msg}"
        );

        let cmds_src = include_str!("cmds.rs");
        let needle = format!("save_{}", "registry");
        assert!(
            !cmds_src.contains(&needle),
            "cmds.rs names `{needle}` again — mutators must write through tbd_tickets ops \
             (T-916.2), never the Value round-trip"
        );
    }
}
