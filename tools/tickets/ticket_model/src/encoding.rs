//! The canonical TOML encoding of one ticket file.
//!
//! **Role:** [`TicketFile`], the on-disk shape of `.ai/tickets/<id>.toml`, its mapping onto
//! [`Ticket`] and back, and [`parse_ticket_toml`] and [`render_ticket_toml`] over it.
//! **Position:** between the model and the files; the store reads and writes every ticket
//! through it, and the ticketboard and `ticket_registry` parse single files with it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** the status is the flat `status` word with a sibling `order`, mapped onto
//! [`Status`]; `[scope]` is the flat table of [`crate::ScopeV2`]; a parse refuses a malformed
//! timestamp, an unknown `class` or `estimated` value, a surface without a component, a bad
//! `kind` or status, and a kind-field mismatch; a render emits the keys in the field order below,
//! so every file renders back byte for byte:
//!
//! - `class` after `summary`, `plan` after `spec`;
//! - `context`, `requirement`, `current_state`, `approach`, `verify` after `main_goal`, before
//!   `acceptance`, and `citations` after `acceptance`;
//! - `estimated` and `estimate_note` after `completed_at`;
//! - `migration_legacy` immediately before `owns`;
//! - `[scope]` last.
//!
//! The set of keys legal on disk is `ticket_registry`'s key contract (`ENCODING_C_KEYS` and
//! `ALLOWED_NEW`) together with `.ai/tickets/schema.json`; a new key changes both and this struct
//! in one commit.

use crate::{ProgramTicket, ScopeV2, Status, StatusName, Ticket, TicketId, WorkTicket};
use serde::{Deserialize, Serialize};

/// One ticket file as TOML holds it: every key of either kind, with the status flattened into
/// `status` and its sibling keys. Field order is the canonical key order; an empty list or a
/// `None` is a key the render omits.
#[derive(Debug, Serialize, Deserialize)]
pub struct TicketFile {
    /// `id`: the ticket id, equal to the file stem.
    pub id: TicketId,
    /// `kind`: `program` or `work`.
    pub kind: String,
    /// `title`: required; changed tickets meet [`crate::TITLE_WORD_CAP`].
    pub title: String,
    /// `summary`; empty when the key is absent.
    #[serde(default)]
    pub summary: String,
    /// `class`: one of [`crate::CLASS_VALUES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// `status`: one of the eight [`StatusName`] words.
    pub status: String,
    /// `order`: the dispatch position; required by the live statuses, refused on `idea`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i64>,
    /// `spec`: the specification document's path; required by the ready-class statuses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
    /// `plan`: the ticket's plan document path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    /// `executor`: who may take the ticket.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor: Option<String>,
    /// `notes`: free-form notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// `priority`: the optional numeric priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    /// `depends_on`: ids that land first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// `unblocks`: ids waiting on this ticket.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unblocks: Vec<String>,
    /// `parent`: a work ticket's parent program; a program renders without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// `children`, also read from `slices`: a program's child ids; refused on a work ticket.
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "slices")]
    pub children: Vec<String>,
    /// `active`, also read from `active_slice`: a program's active child id.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "active_slice"
    )]
    pub active: Option<String>,
    /// `main_goal`, also read from `user_story`, which every older revision spells, in the same
    /// slot; a render always writes `main_goal`. Required by the ready-class statuses.
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "user_story")]
    pub main_goal: Option<String>,
    /// `context`: body lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<String>,
    /// `requirement`: body lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirement: Vec<String>,
    /// `current_state`: body lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub current_state: Vec<String>,
    /// `approach`: body lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approach: Vec<String>,
    /// `verify`: body lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verify: Vec<String>,
    /// `acceptance`: the criteria; a ready-class status needs a non-blank line.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance: Vec<String>,
    /// `citations`: references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<String>,
    /// `shipped_at`: the landing commit's SHA of a shipped ticket.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipped_at: Option<String>,
    /// `created_at`: RFC 3339 UTC, checked in [`TicketFile::into_ticket`], so a malformed value
    /// refuses the parse rather than becoming the current time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// `completed_at`: RFC 3339 UTC, checked like `created_at`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    /// `estimated`: entries of [`crate::ESTIMATED_VALUES`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub estimated: Vec<String>,
    /// `estimate_note`: why an estimate could not be mined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate_note: Option<String>,
    /// `migration_legacy`: parked prose awaiting decomposition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migration_legacy: Vec<String>,
    /// `owns`: the paths and globs the work touches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owns: Vec<String>,
    /// `pack_last`: `true` places the ticket in the last wave.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_last: Option<bool>,
    /// `[scope]`: required on a work ticket, refused on a program.
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

/// Refuses a malformed `created_at` or `completed_at`, naming the ticket; nothing substitutes
/// the current time.
fn validate_timestamps(f: &TicketFile) -> Result<(), String> {
    for (field, value) in [
        ("created_at", f.created_at.as_deref()),
        ("completed_at", f.completed_at.as_deref()),
    ] {
        if let Some(s) = value {
            time_source::validate_rfc3339_utc(field, s).map_err(|e| format!("{}: {e}", f.id))?;
        }
    }
    Ok(())
}

/// Refuses a `class` or `estimated` value outside its set and a `surface` without a
/// `component`. These keys appear only in files that carry today's schema, so unlike the word
/// caps the parse can enforce them without making a historical revision unreadable.
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
    /// The typed ticket this file describes. Refuses, with a one-line reason, what the module
    /// invariants list: a bad timestamp, value, status or kind, a status missing its fields, a
    /// program with a scope or without children, and a work ticket without a scope or with
    /// children.
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

    /// The file shape of `t`, ready to render: the status spread into `status`, `order` and
    /// `shipped_at`, and the keys the kind does not carry left empty.
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

/// Parses one ticket file's text. The parse checks shape only (kinds, status fields, timestamp
/// format, the `class` and `estimated` value sets, surface-requires-component), not scope
/// legality against `.ai/tickets/scope-vocab.toml`, which a lone file cannot know;
/// [`crate::Corpus::load`] and `ticket check` resolve the vocabulary and refuse naming the
/// ticket and the offending word. Errors are one line: the TOML error or the refusal.
pub fn parse_ticket_toml(text: &str) -> Result<Ticket, String> {
    let file: TicketFile = toml::from_str(text).map_err(|e| e.to_string())?;
    file.into_ticket()
}

/// Renders a ticket in the canonical form: [`TicketFile::from_ticket`] through
/// `toml::to_string_pretty`, keys in field order. Errors only when TOML cannot represent a value.
pub fn render_ticket_toml(t: &Ticket) -> Result<String, String> {
    let file = TicketFile::from_ticket(t);
    toml::to_string_pretty(&file).map_err(|e| e.to_string())
}

#[cfg(test)]
#[path = "tests/encoding/mod.rs"]
mod tests;
