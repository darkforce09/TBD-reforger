//! Encoding C: flat `status = "queued"` plus sibling `order`, custom mapping onto [`Status`].

use crate::{
    FrontendScope, ProgramTicket, Scope, Status, StatusName, Ticket, WebsiteScope, WorkTicket,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<WebsiteScopeFile>,
    #[serde(rename = "mod", default, skip_serializing_if = "Option::is_none")]
    pub r#mod: Option<ModScopeFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<LayersFile<crate::SchemaLayer>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<LayersFile<crate::EngineLayer>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<LayersFile<crate::RepoLayer>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebsiteScopeFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<crate::FrontendEditor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<crate::FrontendPage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<crate::FrontendShell>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<LayersFile<crate::WebsiteBackendLayer>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tests: Option<LayersFile<crate::WebsiteTestLayer>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModScopeFile {
    pub layers: Vec<crate::ModLayer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LayersFile<T> {
    pub layers: Vec<T>,
}

impl WebsiteScopeFile {
    fn from_website(w: &WebsiteScope) -> Self {
        let mut f = WebsiteScopeFile::default();
        match w {
            WebsiteScope::Frontend(FrontendScope::Editor(e)) => f.editor = Some(e.clone()),
            WebsiteScope::Frontend(FrontendScope::Page(p)) => f.page = Some(p.clone()),
            WebsiteScope::Frontend(FrontendScope::Shell(s)) => f.shell = Some(s.clone()),
            WebsiteScope::Backend { layers } => {
                f.backend = Some(LayersFile {
                    layers: layers.clone(),
                })
            }
            WebsiteScope::Tests { layers } => {
                f.tests = Some(LayersFile {
                    layers: layers.clone(),
                })
            }
        }
        f
    }

    fn into_website(self) -> Result<WebsiteScope, String> {
        let n = [
            self.editor.is_some(),
            self.page.is_some(),
            self.shell.is_some(),
            self.backend.is_some(),
            self.tests.is_some(),
        ]
        .into_iter()
        .filter(|x| *x)
        .count();
        if n != 1 {
            return Err(format!(
                "expected exactly one [scope.website.*] table, got {n}"
            ));
        }
        Ok(if let Some(e) = self.editor {
            WebsiteScope::Frontend(FrontendScope::Editor(e))
        } else if let Some(p) = self.page {
            WebsiteScope::Frontend(FrontendScope::Page(p))
        } else if let Some(s) = self.shell {
            WebsiteScope::Frontend(FrontendScope::Shell(s))
        } else if let Some(b) = self.backend {
            WebsiteScope::Backend { layers: b.layers }
        } else {
            WebsiteScope::Tests {
                layers: self.tests.unwrap().layers,
            }
        })
    }
}

impl ScopeFile {
    pub fn from_scope(scope: &Scope) -> Self {
        let mut s = ScopeFile::default();
        match scope {
            Scope::Website(w) => s.website = Some(WebsiteScopeFile::from_website(w)),
            Scope::Mod { layers } => {
                s.r#mod = Some(ModScopeFile {
                    layers: layers.clone(),
                })
            }
            Scope::Schema { layers } => {
                s.schema = Some(LayersFile {
                    layers: layers.clone(),
                })
            }
            Scope::Engine { layers } => {
                s.engine = Some(LayersFile {
                    layers: layers.clone(),
                })
            }
            Scope::Repo { layers } => {
                s.repo = Some(LayersFile {
                    layers: layers.clone(),
                })
            }
        }
        s
    }

    pub fn into_scope(self) -> Result<Scope, String> {
        let n = [
            self.website.is_some(),
            self.r#mod.is_some(),
            self.schema.is_some(),
            self.engine.is_some(),
            self.repo.is_some(),
        ]
        .into_iter()
        .filter(|x| *x)
        .count();
        if n != 1 {
            return Err(format!("expected exactly one [scope.*] table, got {n}"));
        }
        Ok(if let Some(w) = self.website {
            Scope::Website(w.into_website()?)
        } else if let Some(m) = self.r#mod {
            Scope::Mod { layers: m.layers }
        } else if let Some(s) = self.schema {
            Scope::Schema { layers: s.layers }
        } else if let Some(e) = self.engine {
            Scope::Engine { layers: e.layers }
        } else {
            Scope::Repo {
                layers: self.repo.unwrap().layers,
            }
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TicketFile {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_story: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipped_at: Option<String>,
    /// T-913.1 lifecycle stamps, canonical slot: after `shipped_at` (still a bare commit
    /// SHA — untouched semantics), before `owns`. RFC 3339 UTC only; validated in
    /// [`TicketFile::into_ticket`], so a malformed value refuses the tree instead of
    /// being coerced to now. Widening the on-disk key set like this also requires
    /// `ALLOWED_NEW` (xtask tickets_store) and `.ai/tickets/schema.json` in the same
    /// deliberate commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owns: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_last: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<ScopeFile>,
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
                f.user_story.clone().unwrap_or_default(),
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

impl TicketFile {
    pub fn into_ticket(self) -> Result<Ticket, String> {
        validate_timestamps(&self)?;
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
                    status,
                    executor: self.executor,
                    notes: self.notes,
                    spec: self.spec,
                    depends_on: self.depends_on,
                    unblocks: self.unblocks,
                    children: self.children,
                    active: self.active,
                    user_story: self.user_story,
                    acceptance: self.acceptance,
                    priority: self.priority,
                    created_at: self.created_at,
                    completed_at: self.completed_at,
                    owns: self.owns,
                    pack_last: self.pack_last,
                }))
            }
            "work" => {
                let scope = self.scope.ok_or("work requires [scope]")?.into_scope()?;
                if !self.children.is_empty() {
                    return Err("work forbids children".into());
                }
                Ok(Ticket::Work(WorkTicket {
                    id: self.id,
                    title: self.title,
                    summary: self.summary,
                    status,
                    executor: self.executor,
                    notes: self.notes,
                    spec: self.spec,
                    depends_on: self.depends_on,
                    unblocks: self.unblocks,
                    parent: self.parent,
                    scope,
                    user_story: self.user_story,
                    acceptance: self.acceptance,
                    shipped_at: self.shipped_at,
                    priority: self.priority,
                    created_at: self.created_at,
                    completed_at: self.completed_at,
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
                status: p.status.name().as_str().into(),
                order: p.status.order(),
                spec: p.spec.clone(),
                executor: p.executor.clone(),
                notes: p.notes.clone(),
                priority: p.priority,
                depends_on: p.depends_on.clone(),
                unblocks: p.unblocks.clone(),
                parent: None,
                children: p.children.clone(),
                active: p.active.clone(),
                user_story: p.user_story.clone(),
                acceptance: p.acceptance.clone(),
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
                owns: p.owns.clone(),
                pack_last: p.pack_last,
                scope: None,
            },
            Ticket::Work(w) => TicketFile {
                id: w.id.clone(),
                kind: "work".into(),
                title: w.title.clone(),
                summary: w.summary.clone(),
                status: w.status.name().as_str().into(),
                order: w.status.order(),
                spec: w.spec.clone(),
                executor: w.executor.clone(),
                notes: w.notes.clone(),
                priority: w.priority,
                depends_on: w.depends_on.clone(),
                unblocks: w.unblocks.clone(),
                parent: w.parent.clone(),
                children: vec![],
                active: None,
                user_story: w.user_story.clone(),
                acceptance: w.acceptance.clone(),
                shipped_at: w.shipped_at.clone(),
                created_at: w.created_at.clone(),
                completed_at: w.completed_at.clone(),
                owns: w.owns.clone(),
                pack_last: w.pack_last,
                scope: Some(ScopeFile::from_scope(&w.scope)),
            },
        }
    }
}

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
    use crate::{RepoLayer, Scope};

    #[test]
    fn parse_render_work_queued() {
        let t = Ticket::Work(WorkTicket {
            id: "T-905".into(),
            title: "x".into(),
            summary: "hello \" \\\\ world".into(),
            status: Status::Queued { order: 5850 },
            executor: Some("claude-code".into()),
            notes: None,
            spec: None,
            depends_on: vec![],
            unblocks: vec![],
            parent: None,
            scope: Scope::Repo {
                layers: vec![RepoLayer::Ci],
            },
            user_story: None,
            acceptance: vec![],
            shipped_at: None,
            priority: None,
            created_at: None,
            completed_at: None,
            owns: vec![],
            pack_last: None,
        });
        let s = render_ticket_toml(&t).unwrap();
        assert!(s.contains("status = \"queued\""));
        assert!(s.contains("order = 5850"));
        assert!(s.contains("[scope.repo]"));
        let back = parse_ticket_toml(&s).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn website_editor_table_path() {
        let t = Ticket::Work(WorkTicket {
            id: "T-816".into(),
            title: "esc".into(),
            summary: "esc".into(),
            status: Status::Queued { order: 4980 },
            executor: Some("claude-code".into()),
            notes: None,
            spec: None,
            depends_on: vec![],
            unblocks: vec![],
            parent: None,
            scope: Scope::Website(WebsiteScope::Frontend(FrontendScope::Editor(
                crate::FrontendEditor {
                    chrome: vec![],
                    capability: None,
                },
            ))),
            user_story: None,
            acceptance: vec![],
            shipped_at: None,
            priority: None,
            created_at: None,
            completed_at: None,
            owns: vec![],
            pack_last: None,
        });
        let s = render_ticket_toml(&t).unwrap();
        assert!(s.contains("[scope.website.editor]"), "{s}");
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
            status: Status::Shipped {
                shipped_at: Some("abc123def".into()),
                order: Some(6000),
            },
            executor: Some("claude-code".into()),
            notes: None,
            spec: None,
            depends_on: vec![],
            unblocks: vec![],
            parent: None,
            scope: Scope::Repo {
                layers: vec![RepoLayer::Tickets],
            },
            user_story: None,
            acceptance: vec![],
            shipped_at: Some("abc123def".into()),
            priority: None,
            created_at: Some("2026-08-14T10:00:00Z".into()),
            completed_at: Some("2026-08-14T11:30:00+00:00".into()),
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

    /// T-913.1: program arm carries the stamps too.
    #[test]
    fn program_timestamps_roundtrip() {
        let toml_in = r#"
id = "T-913"
kind = "program"
title = "metrics"
summary = "metrics"
status = "queued"
order = 5920
children = ["T-913.1"]
created_at = "2026-08-10T09:00:00Z"
completed_at = "2026-08-14T12:00:00Z"
"#;
        let t = parse_ticket_toml(toml_in).unwrap();
        match &t {
            Ticket::Program(p) => {
                assert_eq!(p.created_at.as_deref(), Some("2026-08-10T09:00:00Z"));
                assert_eq!(p.completed_at.as_deref(), Some("2026-08-14T12:00:00Z"));
            }
            Ticket::Work(_) => panic!("T-913 must parse as Program"),
        }
        let rendered = render_ticket_toml(&t).unwrap();
        assert_eq!(t, parse_ticket_toml(&rendered).unwrap());
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
[scope.repo]
layers = ["docs"]
"#
            ))
            .unwrap_err();
            assert!(err.contains("T-901"), "must name the ticket: {err}");
            assert!(err.contains("created_at"), "must name the field: {err}");
        }
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
[scope.repo]
layers = ["docs"]
"#,
        )
        .unwrap_err();
        assert!(err.contains("idea must not carry order"));
    }
}
