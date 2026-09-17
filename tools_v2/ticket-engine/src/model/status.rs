//! Status.

use super::*;

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
        main_goal: String,
        acceptance: Vec<String>,
    },
    Running {
        order: i64,
        spec: String,
        main_goal: String,
        acceptance: Vec<String>,
    },
    Review {
        order: i64,
        spec: String,
        main_goal: String,
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

    /// Ready/running/review require spec + main_goal + nonempty acceptance.
    /// (T-920.1: `main_goal` is the renamed `user_story` — same slot, same rule.)
    pub fn live_ready(
        name: StatusName,
        order: i64,
        spec: String,
        main_goal: String,
        acceptance: Vec<String>,
    ) -> Result<Self, String> {
        if spec.trim().is_empty() {
            return Err("spec required".into());
        }
        if main_goal.trim().is_empty() {
            return Err("main_goal required".into());
        }
        if acceptance.iter().all(|s| s.trim().is_empty()) {
            return Err("acceptance required".into());
        }
        Ok(match name {
            StatusName::Ready => Status::Ready {
                order,
                spec,
                main_goal,
                acceptance,
            },
            StatusName::Running => Status::Running {
                order,
                spec,
                main_goal,
                acceptance,
            },
            StatusName::Review => Status::Review {
                order,
                spec,
                main_goal,
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
