//! The ticket status and the fields each status requires.
//!
//! **Role:** [`StatusName`], the eight on-disk status words, and [`Status`], which carries the
//! fields a status requires, so an illegal combination cannot be built.
//! **Position:** inside `ticket_model::model`; the encoding maps the flat `status` and `order`
//! keys onto [`Status`], and every crate above reads a ticket's lifecycle through it.
//! **Signals & state:** none; plain values.
//! **Invariants:** `queued` carries an order; `ready`, `running` and `review` carry an order, a
//! non-blank `spec` and `main_goal` and an `acceptance` with a non-blank line, and only
//! [`Status::live_ready`] builds them from loose fields; `idea` carries nothing.

use super::*;

/// The eight status words a ticket file's `status` key may hold, without the fields each
/// status carries; serialised as the snake_case word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusName {
    /// `idea`: filed, with no order.
    Idea,
    /// `queued`: in the backlog, with an order.
    Queued,
    /// `ready`: spec and plan written; the run pipeline may take it.
    Ready,
    /// `running`: an agent is working on it.
    Running,
    /// `review`: the work waits for a person to verify it.
    Review,
    /// `shipped`: landed; `shipped_at` names the landing commit once stamped.
    Shipped,
    /// `deferred`: put off by the operator; still open work.
    Deferred,
    /// `cancelled`: dropped; the file stays.
    Cancelled,
}

impl StatusName {
    /// The on-disk word, as the `status` key spells it.
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

    /// The status a `status` word names; `None` for any word outside the eight.
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

    /// Whether the status is live work in the queue: `queued`, `ready`, `running` or `review`.
    pub fn is_live(self) -> bool {
        matches!(
            self,
            StatusName::Queued | StatusName::Ready | StatusName::Running | StatusName::Review
        )
    }
}

/// A ticket's status with the fields that status requires. On disk it is the flat `status` word
/// plus sibling keys (`order`, `spec`, `main_goal`, `acceptance`, `shipped_at`); the encoding maps
/// between the two. `order` is the dispatch position among live tickets, lower first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// Filed with no order; the file carries no `order` key.
    Idea,
    /// In the backlog.
    Queued {
        /// The dispatch position.
        order: i64,
    },
    /// Ready for the run pipeline.
    Ready {
        /// The dispatch position.
        order: i64,
        /// The specification document's repository path; never blank.
        spec: String,
        /// What the ticket achieves, in one statement; never blank.
        main_goal: String,
        /// The acceptance criteria, at least one line non-blank.
        acceptance: Vec<String>,
    },
    /// Taken by an agent; the same fields as [`Status::Ready`].
    Running {
        /// The dispatch position.
        order: i64,
        /// The specification document's repository path; never blank.
        spec: String,
        /// What the ticket achieves, in one statement; never blank.
        main_goal: String,
        /// The acceptance criteria, at least one line non-blank.
        acceptance: Vec<String>,
    },
    /// Waiting for a person to verify the work; the same fields as [`Status::Ready`].
    Review {
        /// The dispatch position.
        order: i64,
        /// The specification document's repository path; never blank.
        spec: String,
        /// What the ticket achieves, in one statement; never blank.
        main_goal: String,
        /// The acceptance criteria, at least one line non-blank.
        acceptance: Vec<String>,
    },
    /// Landed.
    Shipped {
        /// The landing commit's SHA; `None` between `ship` and `stamp-sha`.
        shipped_at: Option<String>,
        /// The dispatch position the ticket held, when the file keeps one.
        order: Option<i64>,
    },
    /// Put off by the operator.
    Deferred {
        /// The dispatch position the ticket held, when the file keeps one.
        order: Option<i64>,
    },
    /// Dropped.
    Cancelled {
        /// The dispatch position the ticket held, when the file keeps one.
        order: Option<i64>,
    },
}

impl Status {
    /// The status word without its fields.
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

    /// The dispatch position: always present on a live status, `None` on `idea`, and whatever
    /// the file keeps on `shipped`, `deferred` and `cancelled`.
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

    /// Builds a `ready`, `running` or `review` status from loose fields. Refuses, with a one-line
    /// reason, a blank `spec` or `main_goal`, an `acceptance` whose lines are all blank, or a
    /// `name` that is not one of the three.
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
