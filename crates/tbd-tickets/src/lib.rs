//! Typed ticket model. Inner `deny(clippy::wildcard_enum_match_arm)` is the sole
//! exhaustive-match authority (T-911.2).
#![deny(clippy::wildcard_enum_match_arm)]

use serde::{Deserialize, Serialize};

mod encoding;
#[cfg(test)]
mod proptest_roundtrip;
pub use encoding::{TicketFile, parse_ticket_toml, render_ticket_toml};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorChrome {
    Left,
    Right,
    Map,
    Top,
    Bottom,
    Attr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Selection,
    Transform,
    Place,
    Persistence,
    Undo,
    Layers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebsiteBackendLayer {
    Api,
    Db,
    Auth,
    Realtime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebsiteTestLayer {
    Unit,
    Integration,
    Gate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModLayer {
    Ui,
    Gamemode,
    Backend,
    Feature,
    Prefab,
    Data,
    Workbench,
    Worlds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaLayer {
    Mission,
    Registry,
    Contract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineLayer {
    Core,
    Render,
    World,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoLayer {
    Ci,
    Docs,
    Xtask,
    Tickets,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FrontendEditor {
    #[serde(default)]
    pub chrome: Vec<EditorChrome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FrontendPage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FrontendShell {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability: Option<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontendScope {
    Editor(FrontendEditor),
    Page(FrontendPage),
    Shell(FrontendShell),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebsiteScope {
    Frontend(FrontendScope),
    Backend { layers: Vec<WebsiteBackendLayer> },
    Tests { layers: Vec<WebsiteTestLayer> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Website(WebsiteScope),
    Mod { layers: Vec<ModLayer> },
    Schema { layers: Vec<SchemaLayer> },
    Engine { layers: Vec<EngineLayer> },
    Repo { layers: Vec<RepoLayer> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusName {
    Idea,
    Queued,
    Ready,
    Running,
    Review,
    Shipped,
    Deferred,
    Cancelled,
}

impl StatusName {
    pub fn as_str(self) -> &'static str {
        match self {
            StatusName::Idea => "idea",
            StatusName::Queued => "queued",
            StatusName::Ready => "ready",
            StatusName::Running => "running",
            StatusName::Review => "review",
            StatusName::Shipped => "shipped",
            StatusName::Deferred => "deferred",
            StatusName::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "idea" => StatusName::Idea,
            "queued" => StatusName::Queued,
            "ready" => StatusName::Ready,
            "running" => StatusName::Running,
            "review" => StatusName::Review,
            "shipped" => StatusName::Shipped,
            "deferred" => StatusName::Deferred,
            "cancelled" => StatusName::Cancelled,
            _ => return None,
        })
    }

    pub fn is_live(self) -> bool {
        matches!(
            self,
            StatusName::Queued | StatusName::Ready | StatusName::Running | StatusName::Review
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Idea,
    Queued {
        order: i64,
    },
    Ready {
        order: i64,
        spec: String,
        user_story: String,
        acceptance: Vec<String>,
    },
    Running {
        order: i64,
        spec: String,
        user_story: String,
        acceptance: Vec<String>,
    },
    Review {
        order: i64,
        spec: String,
        user_story: String,
        acceptance: Vec<String>,
    },
    Shipped {
        shipped_at: Option<String>,
        order: Option<i64>,
    },
    Deferred {
        order: Option<i64>,
    },
    Cancelled {
        order: Option<i64>,
    },
}

impl Status {
    pub fn name(&self) -> StatusName {
        match self {
            Status::Idea => StatusName::Idea,
            Status::Queued { .. } => StatusName::Queued,
            Status::Ready { .. } => StatusName::Ready,
            Status::Running { .. } => StatusName::Running,
            Status::Review { .. } => StatusName::Review,
            Status::Shipped { .. } => StatusName::Shipped,
            Status::Deferred { .. } => StatusName::Deferred,
            Status::Cancelled { .. } => StatusName::Cancelled,
        }
    }

    pub fn order(&self) -> Option<i64> {
        match self {
            Status::Idea => None,
            Status::Queued { order } => Some(*order),
            Status::Ready { order, .. }
            | Status::Running { order, .. }
            | Status::Review { order, .. } => Some(*order),
            Status::Shipped { order, .. }
            | Status::Deferred { order, .. }
            | Status::Cancelled { order, .. } => *order,
        }
    }

    /// Ready/running/review require spec + user_story + nonempty acceptance.
    pub fn live_ready(
        name: StatusName,
        order: i64,
        spec: String,
        user_story: String,
        acceptance: Vec<String>,
    ) -> Result<Self, String> {
        if spec.trim().is_empty() {
            return Err("spec required".into());
        }
        if user_story.trim().is_empty() {
            return Err("user_story required".into());
        }
        if acceptance.iter().all(|s| s.trim().is_empty()) {
            return Err("acceptance required".into());
        }
        Ok(match name {
            StatusName::Ready => Status::Ready {
                order,
                spec,
                user_story,
                acceptance,
            },
            StatusName::Running => Status::Running {
                order,
                spec,
                user_story,
                acceptance,
            },
            StatusName::Review => Status::Review {
                order,
                spec,
                user_story,
                acceptance,
            },
            StatusName::Idea
            | StatusName::Queued
            | StatusName::Shipped
            | StatusName::Deferred
            | StatusName::Cancelled => return Err("not a ready-class status".into()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramTicket {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub status: Status,
    pub executor: Option<String>,
    pub notes: Option<String>,
    pub spec: Option<String>,
    pub depends_on: Vec<String>,
    pub unblocks: Vec<String>,
    pub children: Vec<String>,
    pub active: Option<String>,
    pub user_story: Option<String>,
    pub acceptance: Vec<String>,
    pub priority: Option<i64>,
    pub owns: Vec<String>,
    pub pack_last: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkTicket {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub status: Status,
    pub executor: Option<String>,
    pub notes: Option<String>,
    pub spec: Option<String>,
    pub depends_on: Vec<String>,
    pub unblocks: Vec<String>,
    pub parent: Option<String>,
    pub scope: Scope,
    pub user_story: Option<String>,
    pub acceptance: Vec<String>,
    pub shipped_at: Option<String>,
    pub priority: Option<i64>,
    pub owns: Vec<String>,
    pub pack_last: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ticket {
    Program(ProgramTicket),
    Work(WorkTicket),
}

impl Ticket {
    pub fn id(&self) -> &str {
        match self {
            Ticket::Program(p) => &p.id,
            Ticket::Work(w) => &w.id,
        }
    }

    pub fn status(&self) -> &Status {
        match self {
            Ticket::Program(p) => &p.status,
            Ticket::Work(w) => &w.status,
        }
    }
}

pub const FROZEN_UNMAPPABLE: &[&str] = &[
    "T-067", "T-071", "T-110", "T-111", "T-113", "T-130", "T-134", "T-144", "T-145", "T-146",
    "T-147", "T-148", "T-149", "T-151", "T-160", "T-161", "T-162", "T-163", "T-164", "T-165",
    "T-183", "T-241", "T-242", "T-251", "T-252", "T-253", "T-259", "T-275", "T-280", "T-290",
    "T-291", "T-311", "T-415", "T-419", "T-439", "T-460", "T-462", "T-541", "T-543", "T-545",
    "T-604", "T-605", "T-606", "T-607", "T-608", "T-609", "T-612", "T-617", "T-619",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_unmappable_is_49() {
        assert_eq!(FROZEN_UNMAPPABLE.len(), 49);
        let mut s: Vec<_> = FROZEN_UNMAPPABLE.to_vec();
        s.sort();
        s.dedup();
        assert_eq!(s.len(), 49);
    }

    #[test]
    fn ready_constructor_rejects_empty_story() {
        let err = Status::live_ready(
            StatusName::Ready,
            1,
            "spec.md".into(),
            "   ".into(),
            vec!["a".into()],
        )
        .unwrap_err();
        assert!(err.contains("user_story"));
    }

    #[test]
    fn status_name_roundtrip() {
        for s in [
            "idea",
            "queued",
            "ready",
            "running",
            "review",
            "shipped",
            "deferred",
            "cancelled",
        ] {
            assert_eq!(StatusName::parse(s).unwrap().as_str(), s);
        }
        assert!(StatusName::parse("nope").is_none());
    }
}
