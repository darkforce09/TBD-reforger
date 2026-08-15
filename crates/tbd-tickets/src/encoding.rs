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
mod tests {
    use super::*;
    use crate::Domain;

    #[test]
    fn parse_render_work_queued() {
        let t = Ticket::Work(WorkTicket {
            id: "T-905".into(),
            title: "x".into(),
            summary: "hello \" \\\\ world".into(),
            class: Some("chore".into()),
            status: Status::Queued { order: 5850 },
            executor: Some("claude-code".into()),
            notes: None,
            spec: None,
            plan: None,
            depends_on: vec![],
            unblocks: vec![],
            parent: None,
            scope: ScopeV2 {
                domain: Domain::Repo,
                layer: "ci".into(),
                component: None,
                surface: vec![],
            },
            main_goal: None,
            context: vec![],
            requirement: vec![],
            current_state: vec![],
            approach: vec![],
            verify: vec![],
            acceptance: vec![],
            citations: vec![],
            shipped_at: None,
            priority: None,
            created_at: None,
            completed_at: None,
            estimated: vec![],
            estimate_note: None,
            migration_legacy: vec![],
            owns: vec![],
            pack_last: None,
        });
        let s = render_ticket_toml(&t).unwrap();
        assert!(s.contains("status = \"queued\""));
        assert!(s.contains("order = 5850"));
        assert!(s.contains("[scope]"), "{s}");
        assert!(s.contains("domain = \"repo\""), "{s}");
        assert!(s.contains("layer = \"ci\""), "{s}");
        assert!(!s.contains("component"), "None component omitted:\n{s}");
        assert!(!s.contains("surface"), "empty surface omitted:\n{s}");
        let back = parse_ticket_toml(&s).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn flat_scope_full_depth_roundtrip() {
        let t = parse_ticket_toml(
            r#"
id = "T-816"
kind = "work"
title = "esc"
summary = "esc"
class = "feature"
status = "queued"
order = 4980
executor = "claude-code"

[scope]
domain = "website"
layer = "frontend"
component = "mission_creator"
surface = ["attr_panel", "toolbelt"]
"#,
        )
        .unwrap();
        match &t {
            Ticket::Work(w) => {
                assert_eq!(w.scope.domain, Domain::Website);
                assert_eq!(w.scope.layer, "frontend");
                assert_eq!(w.scope.component.as_deref(), Some("mission_creator"));
                assert_eq!(w.scope.surface, vec!["attr_panel", "toolbelt"]);
            }
            Ticket::Program(_) => panic!("work"),
        }
        let s = render_ticket_toml(&t).unwrap();
        assert!(s.contains("component = \"mission_creator\""), "{s}");
        assert_eq!(t, parse_ticket_toml(&s).unwrap());
    }

    /// The v1 nested scope tree must REFUSE under v2 types — the migrator's whole
    /// reason to work Value→Value.
    #[test]
    fn v1_nested_scope_refuses() {
        let err = parse_ticket_toml(
            r#"
id = "T-001"
kind = "work"
title = "x"
status = "idea"

[scope.repo]
layers = ["docs"]
"#,
        )
        .unwrap_err();
        assert!(
            err.contains("domain") || err.contains("unknown field") || err.contains("repo"),
            "v1 scope must not parse as v2: {err}"
        );
    }

    #[test]
    fn class_and_estimated_values_are_validated() {
        let base = |class: &str, estimated: &str| {
            format!(
                r#"
id = "T-901"
kind = "work"
title = "x"
status = "idea"
{class}
{estimated}

[scope]
domain = "repo"
layer = "docs"
"#
            )
        };
        let err = parse_ticket_toml(&base("class = \"epic\"", "")).unwrap_err();
        assert!(
            err.contains("T-901") && err.contains("epic") && err.contains("bug|feature"),
            "{err}"
        );
        let err = parse_ticket_toml(&base("", "estimated = [\"vibes\"]")).unwrap_err();
        assert!(
            err.contains("T-901") && err.contains("vibes") && err.contains("tokens"),
            "{err}"
        );
        for c in crate::CLASS_VALUES {
            parse_ticket_toml(&base(&format!("class = \"{c}\""), ""))
                .unwrap_or_else(|e| panic!("class {c} legal: {e}"));
        }
        for e in crate::ESTIMATED_VALUES {
            parse_ticket_toml(&base("", &format!("estimated = [\"{e}\"]")))
                .unwrap_or_else(|err| panic!("estimated {e} legal: {err}"));
        }
    }

    #[test]
    fn surface_requires_component() {
        let err = parse_ticket_toml(
            r#"
id = "T-902"
kind = "work"
title = "x"
status = "idea"

[scope]
domain = "website"
layer = "frontend"
surface = ["map_canvas"]
"#,
        )
        .unwrap_err();
        assert!(
            err.contains("T-902") && err.contains("surface requires scope.component"),
            "{err}"
        );
    }

    /// T-917.2: every new key lands in its canonical slot — class after summary, plan
    /// after spec, the body lists between main_goal and acceptance (in order),
    /// citations after acceptance, estimated/estimate_note after completed_at,
    /// migration_legacy immediately before owns, [scope] trailing. Extends the
    /// T-913.1 `timestamps_roundtrip_in_canonical_slot` pattern.
    #[test]
    fn v2_keys_land_in_canonical_slots() {
        let t = Ticket::Work(WorkTicket {
            id: "T-917".into(),
            title: "slotted".into(),
            summary: "slotted".into(),
            class: Some("feature".into()),
            status: Status::Shipped {
                shipped_at: Some("abc123def".into()),
                order: Some(6000),
            },
            executor: Some("claude-code".into()),
            notes: Some("n".into()),
            spec: Some("docs/spec.md".into()),
            plan: Some("docs/plans/T-917_plan.md".into()),
            depends_on: vec!["T-1".into()],
            unblocks: vec!["T-2".into()],
            parent: None,
            scope: ScopeV2 {
                domain: Domain::Website,
                layer: "frontend".into(),
                component: Some("mission_creator".into()),
                surface: vec!["attr_panel".into()],
            },
            main_goal: Some("story".into()),
            context: vec!["why".into()],
            requirement: vec!["ask".into()],
            current_state: vec!["today".into()],
            approach: vec!["steps".into()],
            verify: vec!["cargo test".into()],
            acceptance: vec!["done".into()],
            citations: vec!["docs/x.md".into()],
            shipped_at: Some("abc123def".into()),
            priority: Some(1),
            created_at: Some("2026-08-14T10:00:00Z".into()),
            completed_at: Some("2026-08-14T11:30:00Z".into()),
            estimated: vec!["tokens".into()],
            estimate_note: Some("no receipts era".into()),
            migration_legacy: vec!["old wall".into()],
            owns: vec!["xtask/src/cmds.rs".into()],
            pack_last: None,
        });
        let s = render_ticket_toml(&t).unwrap();
        let pos = |needle: &str| {
            s.find(needle)
                .unwrap_or_else(|| panic!("{needle} in:\n{s}"))
        };
        let order = [
            "summary = ",
            "class = ",
            "status = ",
            "spec = ",
            "plan = ",
            "executor = ",
            "main_goal = ",
            "context = ",
            "requirement = ",
            "current_state = ",
            "approach = ",
            "verify = ",
            "acceptance = ",
            "citations = ",
            "shipped_at = ",
            "created_at = ",
            "completed_at = ",
            "estimated = ",
            "estimate_note = ",
            "migration_legacy = ",
            "owns = ",
            "[scope]",
        ];
        for pair in order.windows(2) {
            assert!(
                pos(pair[0]) < pos(pair[1]),
                "canonical slot violated: {} must precede {}:\n{s}",
                pair[0],
                pair[1]
            );
        }
        assert_eq!(t, parse_ticket_toml(&s).unwrap());
    }

    /// T-913.1: stamps round-trip and land in the canonical slot — after `shipped_at`
    /// (still a bare SHA), before `owns`.
    #[test]
    fn timestamps_roundtrip_in_canonical_slot() {
        let t = Ticket::Work(WorkTicket {
            id: "T-914".into(),
            title: "stamped".into(),
            summary: "stamped".into(),
            class: Some("chore".into()),
            status: Status::Shipped {
                shipped_at: Some("abc123def".into()),
                order: Some(6000),
            },
            executor: Some("claude-code".into()),
            notes: None,
            spec: None,
            plan: None,
            depends_on: vec![],
            unblocks: vec![],
            parent: None,
            scope: ScopeV2 {
                domain: Domain::Repo,
                layer: "tickets".into(),
                component: None,
                surface: vec![],
            },
            main_goal: None,
            context: vec![],
            requirement: vec![],
            current_state: vec![],
            approach: vec![],
            verify: vec![],
            acceptance: vec![],
            citations: vec![],
            shipped_at: Some("abc123def".into()),
            priority: None,
            created_at: Some("2026-08-14T10:00:00Z".into()),
            completed_at: Some("2026-08-14T11:30:00+00:00".into()),
            estimated: vec![],
            estimate_note: None,
            migration_legacy: vec![],
            owns: vec!["xtask/src/cmds.rs".into()],
            pack_last: None,
        });
        let s = render_ticket_toml(&t).unwrap();
        let pos = |needle: &str| {
            s.find(needle)
                .unwrap_or_else(|| panic!("{needle} in:\n{s}"))
        };
        assert!(
            pos("shipped_at = ") < pos("created_at = ")
                && pos("created_at = ") < pos("completed_at = ")
                && pos("completed_at = ") < pos("owns = "),
            "canonical slot violated:\n{s}"
        );
        assert_eq!(t, parse_ticket_toml(&s).unwrap());
    }

    /// T-913.1: program arm carries the stamps too — and (T-917.2) the class/body keys
    /// are LEGAL on programs while scope stays forbidden.
    #[test]
    fn program_timestamps_and_v2_fields_roundtrip() {
        let toml_in = r#"
id = "T-913"
kind = "program"
title = "metrics"
summary = "metrics"
class = "chore"
status = "queued"
order = 5920
children = ["T-913.1"]
context = ["why now"]
verify = ["cargo xtask ticket check"]
created_at = "2026-08-10T09:00:00Z"
completed_at = "2026-08-14T12:00:00Z"
"#;
        let t = parse_ticket_toml(toml_in).unwrap();
        match &t {
            Ticket::Program(p) => {
                assert_eq!(p.created_at.as_deref(), Some("2026-08-10T09:00:00Z"));
                assert_eq!(p.completed_at.as_deref(), Some("2026-08-14T12:00:00Z"));
                assert_eq!(p.class.as_deref(), Some("chore"));
                assert_eq!(p.context, vec!["why now"]);
                assert_eq!(p.verify, vec!["cargo xtask ticket check"]);
            }
            Ticket::Work(_) => panic!("T-913 must parse as Program"),
        }
        let rendered = render_ticket_toml(&t).unwrap();
        assert_eq!(t, parse_ticket_toml(&rendered).unwrap());

        let err = parse_ticket_toml(&format!(
            "{toml_in}\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        ))
        .unwrap_err();
        assert!(err.contains("program forbids [scope]"), "{err}");
    }

    /// T-913.1: malformed stamps are load errors that NAME the ticket — never now.
    #[test]
    fn malformed_timestamp_is_parse_error_naming_ticket() {
        for bad in [
            "2026-13-99T25:61:00Z",
            "2026-08-14 10:00",
            "2026-08-14T10:00:00+05:00",
        ] {
            let err = parse_ticket_toml(&format!(
                r#"
id = "T-901"
kind = "work"
title = "x"
summary = "x"
status = "queued"
order = 1
created_at = "{bad}"
owns = ["docs/x.md"]

[scope]
domain = "repo"
layer = "docs"
"#
            ))
            .unwrap_err();
            assert!(err.contains("T-901"), "must name the ticket: {err}");
            assert!(err.contains("created_at"), "must name the field: {err}");
        }
    }

    /// T-920.1 — the rename roundtrip: a pre-rename blob carrying `user_story`
    /// parses via the serde alias into `main_goal`, and the render emits ONLY
    /// `main_goal`, in the same canonical slot (after `active`-tier keys, before
    /// `context`). A load + write_back of a carrier IS the migration.
    #[test]
    fn user_story_alias_parses_and_emits_main_goal() {
        let legacy = r#"
id = "T-919"
kind = "work"
title = "Wall triage drain"
summary = "s"
class = "chore"
status = "queued"
order = 5990
spec = "docs/spec.md"
user_story = "the goal, pre-rename spelling"
context = ["why"]
acceptance = ["gate"]

[scope]
domain = "repo"
layer = "docs"
"#;
        let t = parse_ticket_toml(legacy).expect("user_story alias parses");
        match &t {
            Ticket::Work(w) => assert_eq!(
                w.main_goal.as_deref(),
                Some("the goal, pre-rename spelling")
            ),
            Ticket::Program(_) => panic!("work"),
        }
        let rendered = render_ticket_toml(&t).expect("render");
        assert!(
            rendered.contains("main_goal = \"the goal, pre-rename spelling\""),
            "{rendered}"
        );
        for line in rendered.lines() {
            assert!(
                !line.starts_with("user_story = "),
                "render must never emit the dead spelling:\n{rendered}"
            );
        }
        // Same canonical slot: spec < main_goal < context < acceptance.
        let pos = |needle: &str| {
            rendered
                .find(needle)
                .unwrap_or_else(|| panic!("{needle} in:\n{rendered}"))
        };
        assert!(
            pos("spec = ") < pos("main_goal = ")
                && pos("main_goal = ") < pos("context = ")
                && pos("context = ") < pos("acceptance = "),
            "canonical slot violated:\n{rendered}"
        );
        assert_eq!(t, parse_ticket_toml(&rendered).unwrap());
        // Carrying BOTH spellings is a serde duplicate-field refusal, not a silent
        // pick — the alias-class discipline the 4a2f3426 pin established.
        let both = legacy.replace(
            "user_story = \"the goal, pre-rename spelling\"",
            "user_story = \"old\"\nmain_goal = \"new\"",
        );
        let err = parse_ticket_toml(&both).expect_err("both spellings must refuse");
        assert!(err.contains("duplicate"), "{err}");
    }

    #[test]
    fn idea_rejects_order() {
        let err = parse_ticket_toml(
            r#"
id = "T-001"
kind = "work"
title = "x"
status = "idea"
order = 1

[scope]
domain = "repo"
layer = "docs"
"#,
        )
        .unwrap_err();
        assert!(err.contains("idea must not carry order"));
    }
}
