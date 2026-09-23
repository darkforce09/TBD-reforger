pub use super::classification::*;
pub use super::scope::*;
use ticket_engine::{ScopeV2, Status, StatusName, Ticket};
/// Column order is the `StatusName` declaration order. The design pins RAW
/// lowercase status names — no friendly labels (the operator lives in CLI-land).
pub const STATUS_ORDER: [StatusName; 8] = [
    StatusName::Idea,
    StatusName::Queued,
    StatusName::Ready,
    StatusName::Running,
    StatusName::Review,
    StatusName::Shipped,
    StatusName::Deferred,
    StatusName::Cancelled,
];

/// Column index for a status — total (all 8 variants), so `STATUS_ORDER` and the
/// buckets can never disagree.
pub fn column_of(status: StatusName) -> usize {
    match status {
        StatusName::Idea => 0,
        StatusName::Queued => 1,
        StatusName::Ready => 2,
        StatusName::Running => 3,
        StatusName::Review => 4,
        StatusName::Shipped => 5,
        StatusName::Deferred => 6,
        StatusName::Cancelled => 7,
    }
}

/// Columns that start as a count chip (click expands): `shipped` and `cancelled`.
pub fn collapsed_by_default(status: StatusName) -> bool {
    matches!(status, StatusName::Shipped | StatusName::Cancelled)
}

/// Registry convention: an absent `executor` means claude-code.
pub const EXECUTOR_DEFAULT: &str = "claude-code";

/// Numeric id key: `T-915.2` → `[915, 2]`, so `T-9 < T-10 < T-100` and
/// `T-915.2 < T-915.10`. Ids that do not parse sort last, tie-broken by the raw
/// string.
pub fn id_sort_key(id: &str) -> (Vec<u64>, String) {
    let segments = id
        .strip_prefix("T-")
        .and_then(|rest| {
            rest.split('.')
                .map(|seg| seg.parse::<u64>().ok())
                .collect::<Option<Vec<u64>>>()
        })
        .unwrap_or_else(|| vec![u64::MAX]);
    (segments, id.to_owned())
}

// ---- ticket field access (shared by cards and the detail panel) ----

pub fn title_of(t: &Ticket) -> &str {
    match t {
        Ticket::Program(p) => &p.title,
        Ticket::Work(w) => &w.title,
    }
}

pub fn executor_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.executor.as_deref(),
        Ticket::Work(w) => w.executor.as_deref(),
    }
}

pub fn executor_label(executor: Option<&str>) -> String {
    executor.unwrap_or(EXECUTOR_DEFAULT).to_owned()
}

pub fn kind_str(t: &Ticket) -> &'static str {
    match t {
        Ticket::Program(_) => "program",
        Ticket::Work(_) => "work",
    }
}

/// `main_goal` lives on both kinds (obligatory from `queued` upward — tier rules); legacy and idea tickets may lack it.
pub fn main_goal_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.main_goal.as_deref(),
        Ticket::Work(w) => w.main_goal.as_deref(),
    }
}

/// Card hover tooltip: the ticket's main_goal — present and
/// nonblank only, so a goal-less ticket attaches NO tooltip, never an empty
/// bubble. Precomputed per card at load; the paint path only attaches it.
pub fn card_tooltip(t: &Ticket) -> Option<String> {
    main_goal_of(t)
        .filter(|goal| !goal.trim().is_empty())
        .map(str::to_owned)
}

/// `shipped_at` lives on the work ticket directly, but inside `Status::Shipped`
/// for programs.
pub fn shipped_at_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Work(w) => w.shipped_at.as_deref(),
        Ticket::Program(p) => match &p.status {
            Status::Shipped { shipped_at, .. } => shipped_at.as_deref(),
            _ => None,
        },
    }
}

/// `"ready · #5961"` / `"shipped"` — raw status name plus order when present.
pub fn status_label(status: &Status) -> String {
    match status.order() {
        Some(order) => format!("{} · #{order}", status.name().as_str()),
        None => status.name().as_str().to_owned(),
    }
}

/// Char-safe truncation with an ellipsis (titles are precomputed per card).
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// `class` is legal on both kinds (required on work by check; rare on programs).
/// A ticket without one renders no chip — the board never invents a class.
pub fn class_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.class.as_deref(),
        Ticket::Work(w) => w.class.as_deref(),
    }
}

/// Uniform read view over both ticket kinds — the detail panel's "EVERY field"
/// surface in one place. Fields a kind does not carry come back empty/None.
pub struct TicketView<'a> {
    pub id: &'a str,
    pub kind: &'static str,
    pub title: &'a str,
    pub summary: &'a str,
    pub status: &'a Status,
    pub executor: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub spec: Option<&'a str>,
    /// per-ticket plan document path (renders it clickable —
    /// the in-app viewer's primary use case).
    pub plan: Option<&'a str>,
    pub depends_on: &'a [String],
    pub unblocks: &'a [String],
    pub parent: Option<&'a str>,
    pub children: &'a [String],
    pub active: Option<&'a str>,
    pub main_goal: Option<&'a str>,
    /// body decomposition lists (spec ) — the detail panel renders
    /// these as the ten pinned sections (, `detail::body_field_order`).
    pub context: &'a [String],
    pub requirement: &'a [String],
    pub current_state: &'a [String],
    pub approach: &'a [String],
    pub verify: &'a [String],
    pub acceptance: &'a [String],
    pub citations: &'a [String],
    /// quarantined wall prose — NOT one of the ten body fields; the
    /// detail panel renders it AFTER them as its own quarantine section.
    pub migration_legacy: &'a [String],
    pub shipped_at: Option<&'a str>,
    pub priority: Option<i64>,
    pub created_at: Option<&'a str>,
    pub completed_at: Option<&'a str>,
    pub owns: &'a [String],
    pub pack_last: Option<bool>,
    /// Raw scope (work tickets only; programs forbid `[scope]`) — render through
    /// [`breadcrumb`].
    pub scope: Option<&'a ScopeV2>,
    /// Raw class value (chips parse it through [`Class::parse`]).
    pub class: Option<&'a str>,
    /// Provenance markers ([`ticket_engine::ESTIMATED_VALUES`]) — consumes only
    /// the `"scope"` entry (the breadcrumb glyph); renders the rest.
    pub estimated: &'a [String],
    /// The method note behind `estimated[]` — rendered VERBATIM as the
    /// stamp-glyph tooltip; the git_subject / id_interpolation phrasing lives in
    /// this text and is never re-derived.
    pub estimate_note: Option<&'a str>,
}

const EMPTY_IDS: &[String] = &[];

pub fn view(t: &Ticket) -> TicketView<'_> {
    let shipped_at = shipped_at_of(t);
    let kind = kind_str(t);
    match t {
        Ticket::Program(p) => TicketView {
            id: &p.id,
            kind,
            title: &p.title,
            summary: &p.summary,
            status: &p.status,
            executor: p.executor.as_deref(),
            notes: p.notes.as_deref(),
            spec: p.spec.as_deref(),
            plan: p.plan.as_deref(),
            depends_on: &p.depends_on,
            unblocks: &p.unblocks,
            parent: None,
            children: &p.children,
            active: p.active.as_deref(),
            main_goal: p.main_goal.as_deref(),
            context: &p.context,
            requirement: &p.requirement,
            current_state: &p.current_state,
            approach: &p.approach,
            verify: &p.verify,
            acceptance: &p.acceptance,
            citations: &p.citations,
            migration_legacy: &p.migration_legacy,
            shipped_at,
            priority: p.priority,
            created_at: p.created_at.as_deref(),
            completed_at: p.completed_at.as_deref(),
            owns: &p.owns,
            pack_last: p.pack_last,
            scope: None,
            class: p.class.as_deref(),
            estimated: &p.estimated,
            estimate_note: p.estimate_note.as_deref(),
        },
        Ticket::Work(w) => TicketView {
            id: &w.id,
            kind,
            title: &w.title,
            summary: &w.summary,
            status: &w.status,
            executor: w.executor.as_deref(),
            notes: w.notes.as_deref(),
            spec: w.spec.as_deref(),
            plan: w.plan.as_deref(),
            depends_on: &w.depends_on,
            unblocks: &w.unblocks,
            parent: w.parent.as_deref(),
            children: EMPTY_IDS,
            active: None,
            main_goal: w.main_goal.as_deref(),
            context: &w.context,
            requirement: &w.requirement,
            current_state: &w.current_state,
            approach: &w.approach,
            verify: &w.verify,
            acceptance: &w.acceptance,
            citations: &w.citations,
            migration_legacy: &w.migration_legacy,
            shipped_at,
            priority: w.priority,
            created_at: w.created_at.as_deref(),
            completed_at: w.completed_at.as_deref(),
            owns: &w.owns,
            pack_last: w.pack_last,
            scope: Some(&w.scope),
            class: w.class.as_deref(),
            estimated: &w.estimated,
            estimate_note: w.estimate_note.as_deref(),
        },
    }
}
