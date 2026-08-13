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

impl TicketFile {
    pub fn into_ticket(self) -> Result<Ticket, String> {
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
            owns: vec![],
            pack_last: None,
        });
        let s = render_ticket_toml(&t).unwrap();
        assert!(s.contains("[scope.website.editor]"), "{s}");
        assert_eq!(t, parse_ticket_toml(&s).unwrap());
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
