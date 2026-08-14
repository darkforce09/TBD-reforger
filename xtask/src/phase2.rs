#![allow(dead_code)] // migrate helpers stay for TBD_PHASE2_WRITE and tests
//! T-911.2: map phase-1 Value tickets onto typed Scope/Status and rewrite TOML.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use tbd_tickets::{
    EngineLayer, FROZEN_UNMAPPABLE, FrontendEditor, FrontendScope, ModLayer, ProgramTicket,
    RepoLayer, SchemaLayer, Scope, Status, StatusName, Ticket, TicketFile, WebsiteScope,
    WorkTicket, parse_ticket_toml, render_ticket_toml,
};

pub fn frozen_unmappable() -> BTreeSet<String> {
    FROZEN_UNMAPPABLE.iter().map(|s| (*s).to_string()).collect()
}

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

fn str_list(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn opt_s(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn child_ids(v: &Value) -> Vec<String> {
    let mut ids = Vec::new();
    if let Some(plan) = v.get("slice_plan").and_then(Value::as_object) {
        for k in plan.keys() {
            if !ids.iter().any(|x| x == k) {
                ids.push(k.clone());
            }
        }
    }
    for s in str_list(v, "slices") {
        if !ids.iter().any(|x| x == &s) {
            ids.push(s);
        }
    }
    for s in str_list(v, "children") {
        if !ids.iter().any(|x| x == &s) {
            ids.push(s);
        }
    }
    ids
}

fn title_from_spec(root: &Path, spec: &str, fallback: &str) -> String {
    let path = root.join(spec);
    if let Ok(text) = std::fs::read_to_string(path) {
        for line in text.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("# ") {
                return rest.trim().to_string();
            }
        }
    }
    fallback.to_string()
}

struct ReadyFill {
    story: &'static str,
    acceptance: &'static [&'static str],
}

fn fill_for(id: &str) -> Option<ReadyFill> {
    Some(match id {
        "T-090" => ReadyFill {
            story: "T-090 / T-091 — Map & terrain program (hub). Mission Creator shows Everon cartography in the editor. Active: T-090.6 (geometry placement audit). Single lane.",
            acceptance: &[
                "Automated sign-off @ T-091.0: cargo xtask ci verify-terrain-strict exit 0.",
                "Code slices add S/M gates in their own specs.",
            ],
        },
        "T-090.4" => ReadyFill {
            story: "Phase A: offline tool compares each exported map object's pivot Z against the T-091 DEM at (x,y) — fast pass over 1M+ Eden objects. This slice is Phase A only — detect, no auto-fix.",
            acceptance: &[
                "Each catalog instance gets demZ vs z with per-kind warn/fail thresholds.",
                "Missing z is warn; no auto-fix (geometry-aware pass is T-090.6).",
            ],
        },
        "T-090.6" => ReadyFill {
            story: "For every exported map object (Eden-scale 1M+), use center + rotation + simplified 3D bounds (not full meshes) to compute which parts are above terrain, buried, or inside another object — fully automated, no manual eyeballing.",
            acceptance: &[
                "OBB from spatial.halfExtentsM + rotationDeg; classify above/buried/inside.",
                "Fully automated; no manual eyeballing of 1M objects.",
            ],
        },
        "T-090.7" => ReadyFill {
            story: "Mission Creator will expose AI inside the Eden-style editor. The AI must read the world base layer (1M+ map objects) with the same certainty a human gets from selecting an entity in Workbench.",
            acceptance: &[
                "ResolvedWorldObject is the exact AI tool shape.",
                "Do not invent parallel field names in frontend AI code.",
            ],
        },
        "T-090.9" => ReadyFill {
            story: "Static world objects become read-only context you can interrogate — hover for a tooltip, click for a read-only inspect panel with Ask AI about this object, filter/search by taxonomy, a legend, and a Z-trust badge — without ever moving them.",
            acceptance: &[
                "Hover tooltip, click inspect, filter, legend, Z-trust badge.",
                "Move/delete/edit terrain props remain Workbench-only.",
            ],
        },
        _ => return None,
    })
}

fn status_from_value(v: &Value, fill: Option<ReadyFill>) -> Result<Status> {
    let raw = opt_s(v, "status").unwrap_or_default();
    let name = StatusName::parse(&raw).with_context(|| format!("bad status {raw}"))?;
    let mut order = v.get("order").and_then(Value::as_i64);
    if name == StatusName::Idea {
        order = None;
    }
    let spec = opt_s(v, "spec").unwrap_or_default();
    let mut story = opt_s(v, "user_story").unwrap_or_default();
    let mut acceptance = str_list(v, "acceptance");
    if let Some(f) = fill {
        if story.trim().is_empty() {
            story = f.story.to_string();
        }
        if acceptance.is_empty() {
            acceptance = f.acceptance.iter().map(|s| (*s).to_string()).collect();
        }
    }
    // Non-frozen ready-class rows must stay ready-class (not NeedsOperator).
    // Locked T-090 family uses fill_for; everyone else keeps spec and synthesizes story/acceptance.
    if matches!(
        name,
        StatusName::Ready | StatusName::Running | StatusName::Review
    ) {
        if story.trim().is_empty() {
            story = opt_s(v, "summary")
                .or_else(|| opt_s(v, "title"))
                .unwrap_or_else(|| "Ready slice".into());
        }
        if acceptance.is_empty() {
            acceptance = vec!["See spec.".into()];
        }
    }
    Ok(match name {
        StatusName::Idea => Status::Idea,
        StatusName::Queued => Status::Queued {
            order: order.context("queued requires order")?,
        },
        StatusName::Ready | StatusName::Running | StatusName::Review => Status::live_ready(
            name,
            order.context("ready-class requires order")?,
            spec,
            story,
            acceptance,
        )
        .map_err(anyhow::Error::msg)?,
        StatusName::Shipped => Status::Shipped {
            shipped_at: opt_s(v, "shipped_at"),
            order,
        },
        StatusName::Deferred => Status::Deferred { order },
        StatusName::Cancelled => Status::Cancelled { order },
    })
}

fn infer_scope(v: &Value) -> Option<Scope> {
    let mut targets = str_list(v, "targets");
    targets.sort();
    targets.dedup();
    match targets
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["mod"] => Some(Scope::Mod {
            layers: vec![ModLayer::Feature],
        }),
        ["website"] => Some(Scope::Website(WebsiteScope::Frontend(
            FrontendScope::Editor(FrontendEditor {
                chrome: vec![],
                capability: None,
            }),
        ))),
        ["root"] => Some(Scope::Repo {
            layers: vec![RepoLayer::Xtask],
        }),
        ["shared"] => Some(Scope::Schema {
            layers: vec![SchemaLayer::Mission],
        }),
        _ => None,
    }
}

fn override_scope(id: &str) -> Option<Scope> {
    Some(match id {
        "T-090.6" => Scope::Engine {
            layers: vec![EngineLayer::World],
        },
        "T-673" | "T-678" | "T-679" | "T-680" | "T-212" | "T-676" | "T-677" | "T-681" | "T-682"
        | "T-685" | "T-689" | "T-705" => Scope::Mod {
            layers: vec![ModLayer::Feature],
        },
        _ => return None,
    })
}

pub enum MapOutcome {
    Mapped(Box<Ticket>),
    NeedsOperator,
}

fn force_t159_23(v: &Value) -> Ticket {
    Ticket::Work(WorkTicket {
        id: "T-159.23".into(),
        title: opt_s(v, "title").unwrap_or_else(|| "Attributes modal".into()),
        summary: opt_s(v, "summary").unwrap_or_else(|| {
            "Attributes Transform+Identity; Arsenal stub filled later by T-180.9 / T-167 / T-810 / T-818."
                .into()
        }),
        status: Status::Shipped {
            shipped_at: Some("69dc5da5".into()),
            order: v.get("order").and_then(Value::as_i64),
        },
        executor: opt_s(v, "executor"),
        notes: opt_s(v, "notes"),
        spec: opt_s(v, "spec"),
        depends_on: str_list(v, "depends_on"),
        unblocks: str_list(v, "unblocks"),
        parent: opt_s(v, "parent"),
        scope: Scope::Website(WebsiteScope::Frontend(FrontendScope::Editor(
            FrontendEditor {
                chrome: vec![tbd_tickets::EditorChrome::Attr],
                capability: None,
            },
        ))),
        user_story: None,
        acceptance: vec![],
        shipped_at: Some("69dc5da5".into()),
        priority: v.get("priority").and_then(Value::as_i64),
        owns: vec![],
        pack_last: None,
    })
}

pub fn map_value(id: &str, v: &Value) -> MapOutcome {
    if frozen_unmappable().contains(id) {
        return MapOutcome::NeedsOperator;
    }
    if id == "T-159.23" {
        return MapOutcome::Mapped(Box::new(force_t159_23(v)));
    }
    let children = if id == "T-674" {
        vec!["T-674.1".into(), "T-674.2".into()]
    } else if id == "T-675" {
        vec!["T-675.1".into(), "T-675.2".into()]
    } else {
        child_ids(v)
    };
    let is_program = !children.is_empty() && opt_s(v, "parent").is_none();
    let fill = fill_for(id);
    let status = match status_from_value(v, fill) {
        Ok(s) => s,
        Err(_) => return MapOutcome::NeedsOperator,
    };

    let mut user_story = opt_s(v, "user_story");
    let mut acceptance = str_list(v, "acceptance");
    if let Some(f) = fill_for(id) {
        if user_story.as_deref().unwrap_or("").trim().is_empty() {
            user_story = Some(f.story.to_string());
        }
        if acceptance.is_empty() {
            acceptance = f.acceptance.iter().map(|s| (*s).to_string()).collect();
        }
    }
    if let Status::Ready {
        user_story: s,
        acceptance: a,
        ..
    }
    | Status::Running {
        user_story: s,
        acceptance: a,
        ..
    }
    | Status::Review {
        user_story: s,
        acceptance: a,
        ..
    } = &status
    {
        if user_story.as_deref().unwrap_or("").trim().is_empty() {
            user_story = Some(s.clone());
        }
        if acceptance.is_empty() {
            acceptance = a.clone();
        }
    }

    if is_program {
        return MapOutcome::Mapped(Box::new(Ticket::Program(ProgramTicket {
            id: id.to_string(),
            title: opt_s(v, "title").unwrap_or_else(|| id.to_string()),
            summary: opt_s(v, "summary").unwrap_or_default(),
            status,
            executor: opt_s(v, "executor"),
            notes: opt_s(v, "notes"),
            spec: opt_s(v, "spec"),
            depends_on: str_list(v, "depends_on"),
            unblocks: str_list(v, "unblocks"),
            children,
            active: opt_s(v, "active").or_else(|| opt_s(v, "active_slice")),
            user_story,
            acceptance,
            priority: v.get("priority").and_then(Value::as_i64),
            owns: str_list(v, "owns"),
            pack_last: v.get("pack_last").and_then(Value::as_bool),
        })));
    }

    let scope = override_scope(id)
        .or_else(|| infer_scope(v))
        .unwrap_or(Scope::Repo {
            layers: vec![RepoLayer::Docs],
        });
    MapOutcome::Mapped(Box::new(Ticket::Work(WorkTicket {
        id: id.to_string(),
        title: opt_s(v, "title").unwrap_or_else(|| id.to_string()),
        summary: opt_s(v, "summary").unwrap_or_default(),
        status,
        executor: opt_s(v, "executor"),
        notes: opt_s(v, "notes"),
        spec: opt_s(v, "spec"),
        depends_on: str_list(v, "depends_on"),
        unblocks: str_list(v, "unblocks"),
        parent: opt_s(v, "parent"),
        scope,
        user_story,
        acceptance,
        shipped_at: opt_s(v, "shipped_at"),
        priority: v.get("priority").and_then(Value::as_i64),
        owns: str_list(v, "owns"),
        pack_last: v.get("pack_last").and_then(Value::as_bool),
    })))
}

fn park_unmappable(id: &str, v: &Value) -> Ticket {
    let mut status = status_from_value(v, None).unwrap_or(Status::Idea);
    if matches!(status, Status::Idea) {
        status = Status::Idea;
    }
    let notes = Some(format!(
        "needs-operator: frozen unmappable Scope. {}",
        opt_s(v, "notes").unwrap_or_default()
    ));
    let children = child_ids(v);
    if !children.is_empty() {
        return Ticket::Program(ProgramTicket {
            id: id.to_string(),
            title: opt_s(v, "title").unwrap_or_else(|| id.to_string()),
            summary: opt_s(v, "summary").unwrap_or_default(),
            status,
            executor: opt_s(v, "executor"),
            notes,
            spec: opt_s(v, "spec"),
            depends_on: str_list(v, "depends_on"),
            unblocks: str_list(v, "unblocks"),
            children,
            active: opt_s(v, "active").or_else(|| opt_s(v, "active_slice")),
            user_story: opt_s(v, "user_story"),
            acceptance: str_list(v, "acceptance"),
            priority: v.get("priority").and_then(Value::as_i64),
            owns: vec![],
            pack_last: None,
        });
    }
    Ticket::Work(WorkTicket {
        id: id.to_string(),
        title: opt_s(v, "title").unwrap_or_else(|| id.to_string()),
        summary: opt_s(v, "summary").unwrap_or_default(),
        status,
        executor: opt_s(v, "executor"),
        notes,
        spec: opt_s(v, "spec"),
        depends_on: str_list(v, "depends_on"),
        unblocks: str_list(v, "unblocks"),
        parent: opt_s(v, "parent"),
        scope: Scope::Repo {
            layers: vec![RepoLayer::Docs],
        },
        user_story: opt_s(v, "user_story"),
        acceptance: str_list(v, "acceptance"),
        shipped_at: opt_s(v, "shipped_at"),
        priority: v.get("priority").and_then(Value::as_i64),
        owns: vec![],
        pack_last: None,
    })
}

fn targets_from_scope(scope: &Scope) -> Vec<String> {
    vec![match scope {
        Scope::Website(_) => "website".into(),
        Scope::Mod { .. } => "mod".into(),
        Scope::Schema { .. } => "shared".into(),
        Scope::Engine { .. } | Scope::Repo { .. } => "root".into(),
    }]
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

fn load_all_phase1_files(root: &Path) -> Result<BTreeMap<String, Value>> {
    let dir = crate::tickets_store::tickets_dir(root);
    let mut out = BTreeMap::new();
    for ent in std::fs::read_dir(&dir)? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("T-") || !name.ends_with(".toml") {
            continue;
        }
        let id = name.trim_end_matches(".toml").to_string();
        let text = std::fs::read_to_string(ent.path())?;
        let (_, v) = crate::tickets_store::ticket_from_toml_str(&text)?;
        out.insert(id, v);
    }
    Ok(out)
}

fn enrich_child(root: &Path, id: &str, v: &mut Value, files: &BTreeMap<String, Value>) {
    let parent_id = opt_s(v, "parent");
    if let Some(pid) = parent_id.as_deref() {
        if let Some(parent) = files.get(pid) {
            if let Some(entry) = parent
                .get("slice_plan")
                .and_then(Value::as_object)
                .and_then(|p| p.get(id))
            {
                if let Some(obj) = v.as_object_mut() {
                    if let Some(eo) = entry.as_object() {
                        for (k, val) in eo {
                            if k == "__keys" {
                                continue;
                            }
                            obj.entry(k.clone()).or_insert_with(|| val.clone());
                        }
                    }
                }
            }
            if v.get("order").is_none() {
                if let Some(o) = parent.get("order") {
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("order".into(), o.clone());
                    }
                }
            }
            if opt_s(v, "status").is_none() {
                let pst = opt_s(parent, "status").unwrap_or_else(|| "idea".into());
                let child_st =
                    if matches!(pst.as_str(), "shipped" | "deferred" | "cancelled" | "idea") {
                        pst
                    } else {
                        "idea".into()
                    };
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("status".into(), Value::String(child_st));
                }
            }
        }
    }
    if opt_s(v, "status").is_none() {
        if let Some(obj) = v.as_object_mut() {
            obj.insert("status".into(), Value::String("idea".into()));
        }
    }
    if opt_s(v, "title").is_none() {
        let spec = opt_s(v, "spec").unwrap_or_default();
        let title = if spec.is_empty() {
            id.to_string()
        } else {
            title_from_spec(root, &spec, id)
        };
        if let Some(obj) = v.as_object_mut() {
            obj.insert("title".into(), Value::String(title));
        }
    }
    if opt_s(v, "summary").is_none() {
        let title = opt_s(v, "title").unwrap_or_else(|| id.to_string());
        if let Some(obj) = v.as_object_mut() {
            obj.insert("summary".into(), Value::String(title));
        }
    }
    let st = opt_s(v, "status").unwrap_or_default();
    if matches!(st.as_str(), "queued" | "ready" | "running" | "review")
        && v.get("order").and_then(Value::as_i64).is_none()
    {
        if let Some(obj) = v.as_object_mut() {
            obj.insert("order".into(), Value::Number(0.into()));
        }
    }
}

fn synthetic_child(id: &str, title: &str, parent: &str, scope: Scope, order: i64) -> Ticket {
    Ticket::Work(WorkTicket {
        id: id.into(),
        title: title.into(),
        summary: title.into(),
        status: Status::Queued { order },
        executor: Some("claude-code".into()),
        notes: None,
        spec: None,
        depends_on: vec![],
        unblocks: vec![],
        parent: Some(parent.into()),
        scope,
        user_story: None,
        acceptance: vec![],
        shipped_at: None,
        priority: None,
        owns: vec![],
        pack_last: None,
    })
}

pub fn migrate_live_tree(root: &Path) -> Result<(usize, BTreeSet<String>)> {
    let mut files = load_all_phase1_files(root)?;
    let mut needs = BTreeSet::new();
    let mut mapped: BTreeMap<String, Ticket> = BTreeMap::new();
    let ids: Vec<String> = files.keys().cloned().collect();
    for id in ids {
        let mut v = files.get(&id).cloned().unwrap();
        enrich_child(root, &id, &mut v, &files);
        files.insert(id.clone(), v.clone());
        match map_value(&id, &v) {
            MapOutcome::Mapped(t) => {
                mapped.insert(id, *t);
            }
            MapOutcome::NeedsOperator => {
                needs.insert(id.clone());
                mapped.insert(id.clone(), park_unmappable(&id, &v));
            }
        }
    }
    mapped.insert(
        "T-674.1".into(),
        synthetic_child(
            "T-674.1",
            "T-674 engine flatten emit",
            "T-674",
            Scope::Engine {
                layers: vec![EngineLayer::Core],
            },
            4310,
        ),
    );
    mapped.insert(
        "T-674.2".into(),
        synthetic_child(
            "T-674.2",
            "T-674 Enfusion reader",
            "T-674",
            Scope::Mod {
                layers: vec![ModLayer::Backend],
            },
            4310,
        ),
    );
    mapped.insert(
        "T-675.1".into(),
        synthetic_child(
            "T-675.1",
            "T-675 engine flatten emit",
            "T-675",
            Scope::Engine {
                layers: vec![EngineLayer::Core],
            },
            4320,
        ),
    );
    mapped.insert(
        "T-675.2".into(),
        synthetic_child(
            "T-675.2",
            "T-675 Enfusion reader",
            "T-675",
            Scope::Mod {
                layers: vec![ModLayer::Backend],
            },
            4320,
        ),
    );

    let dir = crate::tickets_store::tickets_dir(root);
    let mut n = 0;
    for (id, t) in &mapped {
        let path = dir.join(format!("{id}.toml"));
        let text = render_ticket_toml(t).map_err(anyhow::Error::msg)?;
        parse_ticket_toml(&text).map_err(|e| anyhow::anyhow!("{id} render does not parse: {e}"))?;
        std::fs::write(&path, text)?;
        n += 1;
    }
    Ok((n, needs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tickets_store::load_toml_tree;

    fn repo_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn needs_operator_set_equals_frozen_49() {
        let root = repo_root();
        let v = if tree_is_phase2(&root) {
            // After rewrite, NeedsOperator is by id on reconstructed values.
            crate::registry::load_registry(&root).unwrap()
        } else {
            load_toml_tree(&root).unwrap()
        };
        let mut needs = BTreeSet::new();
        for t in v.get("tickets").unwrap().as_array().unwrap() {
            let id = t.get("id").unwrap().as_str().unwrap();
            if matches!(map_value(id, t), MapOutcome::NeedsOperator) {
                needs.insert(id.to_string());
            }
        }
        // Frozen ids that became Program (have children) still match by id.
        assert_eq!(needs, frozen_unmappable());
    }

    #[test]
    fn write_live_phase2_tree() {
        if std::env::var("TBD_PHASE2_WRITE").ok().as_deref() != Some("1") {
            return;
        }
        let root = repo_root();
        if tree_is_phase2(&root) {
            return;
        }
        let (n, needs) = migrate_live_tree(&root).expect("migrate");
        assert_eq!(needs, frozen_unmappable());
        assert!(n > 800, "rewrote {n} files");
        assert!(tree_is_phase2(&root));
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

    #[test]
    fn t090_6_is_engine_scope() {
        let root = repo_root();
        if !tree_is_phase2(&root) {
            return;
        }
        match parse_file(&root, "T-090.6") {
            Ticket::Work(w) => assert!(
                matches!(w.scope, Scope::Engine { .. }),
                "T-090.6 scope {:?}",
                w.scope
            ),
            Ticket::Program(_) => panic!("T-090.6 must be Work"),
        }
    }

    /// T-912.2 regression pin: `ticket_to_value` mirrors `children`/`active` into their legacy
    /// spellings, and `value_to_ticket` must accept its own output — the alias-vs-mirror clash
    /// made `duplicate field \`children\`` out of every loaded program and broke every registry
    /// mutator (`ticket ship T-905` → `save T-067` refuse, measured at the T-912.1 tip).
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
}
