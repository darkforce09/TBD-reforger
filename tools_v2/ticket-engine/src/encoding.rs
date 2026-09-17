//! Encoding C: flat `status = "queued"` plus sibling `order`, custom mapping onto [`Status`].
//!
//! T-917.2 (schema v2): `[scope]` is a FLAT table (`domain`/`layer`/`component`/`surface`
//! — the nested `[scope.website.editor]` tree and its `ScopeFile` plumbing died at the
//! cutover), and the ticket body decomposed into typed fields. Canonical top-level key
//! slots, in emit order (pinned by `v2_keys_land_in_canonical_slots`):
//!
//! - `class` after `summary`;
//! - `plan` after `spec`;
//! - `context`, `requirement`, `current_state`, `approach`, `verify` after
//!   `main_goal` (the T-920.1 rename of `user_story` — same slot), before
//!   `acceptance`;
//! - `citations` after `acceptance`;
//! - `estimated` + `estimate_note` after `completed_at`;
//! - `migration_legacy` immediately before `owns`;
//! - `[scope]` stays the trailing table.
//!
//! Widening the on-disk key set requires `ALLOWED_NEW` (xtask tickets_store) and
//! `.ai/tickets/schema.json` in the same deliberate commit.

use crate::{ProgramTicket, ScopeV2, Status, StatusName, Ticket, WorkTicket};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TicketFile {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unblocks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "slices")]
    pub children: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "active_slice"
    )]
    pub active: Option<String>,
    /// T-920.1 rename (t920 spec Decisions log #1): the on-disk key is `main_goal`;
    /// `user_story` is a parse-time serde alias so every pre-rename git revision
    /// stays readable — render always emits `main_goal`, in the SAME canonical slot
    /// the old key held. `user_story` itself stays listed in the frozen
    /// `ENCODING_C_KEYS` as history (on-disk keys must be a SUBSET of the union — a
    /// vanished key is legal).
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "user_story")]
    pub main_goal: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirement: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub current_state: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approach: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verify: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipped_at: Option<String>,
    /// T-913.1 lifecycle stamps, canonical slot: after `shipped_at` (still a bare commit
    /// SHA — untouched semantics), before the provenance keys. RFC 3339 UTC only;
    /// validated in [`TicketFile::into_ticket`], so a malformed value refuses the tree
    /// instead of being coerced to now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub estimated: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate_note: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migration_legacy: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_last: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<ScopeV2>,
}

fn status_from_file(f: &TicketFile) -> Result<Status, String> {
    let name = StatusName::parse(&f.status).ok_or_else(|| format!("bad status {}", f.status))?;
    match name {
        StatusName::Idea => {
            if f.order.is_some() {
                return Err("idea must not carry order".into());
            }
            Ok(Status::Idea)
        }
        StatusName::Queued => {
            let order = f.order.ok_or("queued requires order")?;
            Ok(Status::Queued { order })
        }
        StatusName::Ready | StatusName::Running | StatusName::Review => {
            let order = f.order.ok_or("ready-class requires order")?;
            Status::live_ready(
                name,
                order,
                f.spec.clone().unwrap_or_default(),
                f.main_goal.clone().unwrap_or_default(),
                f.acceptance.clone(),
            )
        }
        StatusName::Shipped => Ok(Status::Shipped {
            shipped_at: f.shipped_at.clone(),
            order: f.order,
        }),
        StatusName::Deferred => Ok(Status::Deferred { order: f.order }),
        StatusName::Cancelled => Ok(Status::Cancelled { order: f.order }),
    }
}

/// T-913.1: malformed lifecycle stamps are parse errors that NAME the ticket — the load
/// refuses; nothing ever substitutes now.
fn validate_timestamps(f: &TicketFile) -> Result<(), String> {
    for (field, value) in [
        ("created_at", f.created_at.as_deref()),
        ("completed_at", f.completed_at.as_deref()),
    ] {
        if let Some(s) = value {
            crate::timestamp::validate_rfc3339_utc(field, s)
                .map_err(|e| format!("{}: {e}", f.id))?;
        }
    }
    Ok(())
}

/// T-917.2 value validation for the new keys. Safe to parse-enforce (unlike the body
/// caps): the keys did not exist before v2, so no historical revision can carry them.
fn validate_v2_fields(f: &TicketFile) -> Result<(), String> {
    if let Some(class) = &f.class
        && !crate::CLASS_VALUES.contains(&class.as_str())
    {
        return Err(format!(
            "{}: class \"{class}\" is not one of {}",
            f.id,
            crate::CLASS_VALUES.join("|")
        ));
    }
    for e in &f.estimated {
        if !crate::ESTIMATED_VALUES.contains(&e.as_str()) {
            return Err(format!(
                "{}: estimated[] entry \"{e}\" is not one of {}",
                f.id,
                crate::ESTIMATED_VALUES.join("|")
            ));
        }
    }
    if let Some(scope) = &f.scope
        && !scope.surface.is_empty()
        && scope.component.is_none()
    {
        return Err(format!(
            "{}: scope.surface requires scope.component (the vocabulary has no layer-level surfaces)",
            f.id
        ));
    }
    Ok(())
}

impl TicketFile {
    pub fn into_ticket(self) -> Result<Ticket, String> {
        validate_timestamps(&self)?;
        validate_v2_fields(&self)?;
        let status = status_from_file(&self)?;
        match self.kind.as_str() {
            "program" => {
                if self.scope.is_some() {
                    return Err("program forbids [scope]".into());
                }
                if self.children.is_empty() {
                    return Err("program requires children".into());
                }
                Ok(Ticket::Program(ProgramTicket {
                    id: self.id,
                    title: self.title,
                    summary: self.summary,
                    class: self.class,
                    status,
                    executor: self.executor,
                    notes: self.notes,
                    spec: self.spec,
                    plan: self.plan,
                    depends_on: self.depends_on,
                    unblocks: self.unblocks,
                    children: self.children,
                    active: self.active,
                    main_goal: self.main_goal,
                    context: self.context,
                    requirement: self.requirement,
                    current_state: self.current_state,
                    approach: self.approach,
                    verify: self.verify,
                    acceptance: self.acceptance,
                    citations: self.citations,
                    priority: self.priority,
                    created_at: self.created_at,
                    completed_at: self.completed_at,
                    estimated: self.estimated,
                    estimate_note: self.estimate_note,
                    migration_legacy: self.migration_legacy,
                    owns: self.owns,
                    pack_last: self.pack_last,
                }))
            }
            "work" => {
                let scope = self.scope.ok_or("work requires [scope]")?;
                if !self.children.is_empty() {
                    return Err("work forbids children".into());
                }
                Ok(Ticket::Work(WorkTicket {
                    id: self.id,
                    title: self.title,
                    summary: self.summary,
                    class: self.class,
                    status,
                    executor: self.executor,
                    notes: self.notes,
                    spec: self.spec,
                    plan: self.plan,
                    depends_on: self.depends_on,
                    unblocks: self.unblocks,
                    parent: self.parent,
                    scope,
                    main_goal: self.main_goal,
                    context: self.context,
                    requirement: self.requirement,
                    current_state: self.current_state,
                    approach: self.approach,
                    verify: self.verify,
                    acceptance: self.acceptance,
                    citations: self.citations,
                    shipped_at: self.shipped_at,
                    priority: self.priority,
                    created_at: self.created_at,
                    completed_at: self.completed_at,
                    estimated: self.estimated,
                    estimate_note: self.estimate_note,
                    migration_legacy: self.migration_legacy,
                    owns: self.owns,
                    pack_last: self.pack_last,
                }))
            }
            other => Err(format!("bad kind {other}")),
        }
    }

    pub fn from_ticket(t: &Ticket) -> Self {
        match t {
            Ticket::Program(p) => TicketFile {
                id: p.id.clone(),
                kind: "program".into(),
                title: p.title.clone(),
                summary: p.summary.clone(),
                class: p.class.clone(),
                status: p.status.name().as_str().into(),
                order: p.status.order(),
                spec: p.spec.clone(),
                plan: p.plan.clone(),
                executor: p.executor.clone(),
                notes: p.notes.clone(),
                priority: p.priority,
                depends_on: p.depends_on.clone(),
                unblocks: p.unblocks.clone(),
                parent: None,
                children: p.children.clone(),
                active: p.active.clone(),
                main_goal: p.main_goal.clone(),
                context: p.context.clone(),
                requirement: p.requirement.clone(),
                current_state: p.current_state.clone(),
                approach: p.approach.clone(),
                verify: p.verify.clone(),
                acceptance: p.acceptance.clone(),
                citations: p.citations.clone(),
                shipped_at: match &p.status {
                    Status::Shipped { shipped_at, .. } => shipped_at.clone(),
                    Status::Idea
                    | Status::Queued { .. }
                    | Status::Ready { .. }
                    | Status::Running { .. }
                    | Status::Review { .. }
                    | Status::Deferred { .. }
                    | Status::Cancelled { .. } => None,
                },
                created_at: p.created_at.clone(),
                completed_at: p.completed_at.clone(),
                estimated: p.estimated.clone(),
                estimate_note: p.estimate_note.clone(),
                migration_legacy: p.migration_legacy.clone(),
                owns: p.owns.clone(),
                pack_last: p.pack_last,
                scope: None,
            },
            Ticket::Work(w) => TicketFile {
                id: w.id.clone(),
                kind: "work".into(),
                title: w.title.clone(),
                summary: w.summary.clone(),
                class: w.class.clone(),
                status: w.status.name().as_str().into(),
                order: w.status.order(),
                spec: w.spec.clone(),
                plan: w.plan.clone(),
                executor: w.executor.clone(),
                notes: w.notes.clone(),
                priority: w.priority,
                depends_on: w.depends_on.clone(),
                unblocks: w.unblocks.clone(),
                parent: w.parent.clone(),
                children: vec![],
                active: None,
                main_goal: w.main_goal.clone(),
                context: w.context.clone(),
                requirement: w.requirement.clone(),
                current_state: w.current_state.clone(),
                approach: w.approach.clone(),
                verify: w.verify.clone(),
                acceptance: w.acceptance.clone(),
                citations: w.citations.clone(),
                shipped_at: w.shipped_at.clone(),
                created_at: w.created_at.clone(),
                completed_at: w.completed_at.clone(),
                estimated: w.estimated.clone(),
                estimate_note: w.estimate_note.clone(),
                migration_legacy: w.migration_legacy.clone(),
                owns: w.owns.clone(),
                pack_last: w.pack_last,
                scope: Some(w.scope.clone()),
            },
        }
    }
}

/// Parse one ticket TOML. **Documented weakening (T-917.2, spec §Scope v2):** a bare
/// parse is SHAPE-STRICT ONLY — it validates structure (kinds, status data, timestamp
/// format, class/estimated value sets, surface-requires-component) but NOT scope
/// legality against `.ai/tickets/scope-vocab.toml`, because a lone parse cannot know
/// per-parent vocabulary legality. Every real path goes through [`crate::Corpus::load`]
/// or `ticket check`, which resolve the vocabulary and refuse naming ticket +
/// offending pair.
pub fn parse_ticket_toml(text: &str) -> Result<Ticket, String> {
    let file: TicketFile = toml::from_str(text).map_err(|e| e.to_string())?;
    file.into_ticket()
}

pub fn render_ticket_toml(t: &Ticket) -> Result<String, String> {
    let file = TicketFile::from_ticket(t);
    toml::to_string_pretty(&file).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "tests/encoding/mod.rs"]
mod tests;
