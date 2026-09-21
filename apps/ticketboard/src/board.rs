//! Board projection (T-915.1 §UI shape) — pure, unit-tested, no egui types.
//!
//! Buckets the corpus into the 8 raw `StatusName` columns, sorts cards by
//! `(order, id)`, and precomputes every card label at load time so the paint path
//! never formats strings.

use std::collections::HashMap;

use ticket_engine::{ScopeV2, Status, StatusName, Ticket};

use crate::corpus::Corpus;

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

/// Card title truncation bound, in chars (precomputed; card rows never wrap).
const TITLE_MAX_CHARS: usize = 48;

/// Everything the card paint path needs, precomputed at load.
pub struct Card {
    /// Index into `Corpus::tickets`.
    pub index: usize,
    pub id: String,
    pub title: String,
    pub executor: String,
    /// `"#5961"` when the ticket carries an order, else empty.
    pub order_label: String,
    /// Scope breadcrumb (work tickets; programs carry no scope). Cards render the
    /// compact form — no [`NO_SURFACE_MARKER`] (detail-panel only).
    pub breadcrumb: Option<Breadcrumb>,
    /// Class chip accent (absent class — programs, pre-triage work — no chip).
    pub class: Option<Class>,
    /// Hover tooltip (T-920.2): the ticket's main_goal via [`card_tooltip`] —
    /// the goal surfaces on hover without opening the detail panel. Absent or
    /// blank ⇒ `None` (no empty tooltip bubble).
    pub tooltip: Option<String>,
}

pub struct Column {
    pub status: StatusName,
    /// `"queued · 173"` — precomputed header.
    pub header: String,
    /// `"shipped\n743"` — precomputed count-chip label for collapsed columns.
    pub chip: String,
    pub cards: Vec<Card>,
}

pub struct BoardModel {
    pub columns: [Column; 8],
    /// Ticket id → index into `Corpus::tickets` (clickable id refs in the detail
    /// panel resolve through this).
    pub id_to_index: HashMap<String, usize>,
}

/// `(order, id)` — absent orders sort last, so `idea` columns fall back to pure
/// numeric-id order.
type SortKey = (i64, Vec<u64>, String);

fn sort_key(ticket: &Ticket) -> SortKey {
    let (segments, raw) = id_sort_key(ticket.id());
    (ticket.status().order().unwrap_or(i64::MAX), segments, raw)
}

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

impl BoardModel {
    pub fn build(corpus: &Corpus) -> Self {
        let mut buckets: [Vec<(SortKey, Card)>; 8] = Default::default();
        let mut id_to_index = HashMap::with_capacity(corpus.tickets.len());
        for (index, loaded) in corpus.tickets.iter().enumerate() {
            let t = &loaded.ticket;
            id_to_index.insert(t.id().to_owned(), index);
            let card = Card {
                index,
                id: t.id().to_owned(),
                title: truncate_chars(title_of(t), TITLE_MAX_CHARS),
                executor: executor_label(executor_of(t)),
                order_label: t
                    .status()
                    .order()
                    .map(|o| format!("#{o}"))
                    .unwrap_or_default(),
                breadcrumb: match t {
                    Ticket::Work(w) => Some(breadcrumb(&w.scope, &w.estimated)),
                    Ticket::Program(_) => None,
                },
                class: class_of(t).and_then(Class::parse),
                tooltip: card_tooltip(t),
            };
            buckets[column_of(t.status().name())].push((sort_key(t), card));
        }
        let mut buckets = buckets.into_iter();
        let columns = STATUS_ORDER.map(|status| {
            let mut bucket = buckets.next().expect("8 buckets for 8 statuses");
            bucket.sort_by(|a, b| a.0.cmp(&b.0));
            let cards: Vec<Card> = bucket.into_iter().map(|(_, card)| card).collect();
            Column {
                header: format!("{} · {}", status.as_str(), cards.len()),
                chip: format!("{}\n{}", status.as_str(), cards.len()),
                status,
                cards,
            }
        });
        Self {
            columns,
            id_to_index,
        }
    }
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

/// `main_goal` lives on both kinds (obligatory from `queued` upward — T-920
/// tier rules); legacy and idea tickets may lack it.
pub fn main_goal_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.main_goal.as_deref(),
        Ticket::Work(w) => w.main_goal.as_deref(),
    }
}

/// Card hover tooltip (T-920.2): the ticket's main_goal — present and
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

// ---- scope breadcrumb (T-918.1 / B.1) ----

/// Breadcrumb level — each level gets its own muted accent (app.rs maps to color).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeLevel {
    Domain,
    Layer,
    Component,
    Surface,
}

/// One breadcrumb segment: level + display text (the surface list joins into a
/// single `s1+s2` segment — one level, one segment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbSeg {
    pub level: ScopeLevel,
    pub text: String,
}

/// Display model for the v2 scope path (supersedes the T-917.2 raw
/// `scope_compact`): `domain › layer [› component] [› s1+s2]`.
///
/// - An absent component is SKIPPED visibly — the path is just `domain › layer`
///   (component-free layers exist per the vocabulary; nothing is missing there).
/// - `no_surface` marks a component WITH an empty surface list — a surface could
///   exist and does not (surface is required on live tickets from S.2 on). The
///   DETAIL panel renders the explicit [`NO_SURFACE_MARKER`]; cards stay compact
///   and omit it. Component-free scope never sets it (no surface tier to miss).
/// - `estimated` marks `"scope" ∈ estimated[]` — the migrator owns-inferred this
///   scope; rendered as the [`SCOPE_ESTIMATED_GLYPH`] with its tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breadcrumb {
    pub segs: Vec<BreadcrumbSeg>,
    pub no_surface: bool,
    pub estimated: bool,
}

/// Separator glyph between breadcrumb segments.
pub const SCOPE_SEP: &str = "›";
/// Detail-panel marker for a component whose surface list is empty.
pub const NO_SURFACE_MARKER: &str = "(no surface)";
/// Glyph prefixed to owns-inferred (estimated) scope breadcrumbs.
pub const SCOPE_ESTIMATED_GLYPH: &str = "~";
/// Tooltip on the estimated-scope glyph.
pub const SCOPE_ESTIMATED_TIP: &str = "scope owns-inferred at migration";

/// The estimated[] marker predicate: was this ticket's scope owns-inferred by the
/// v2 migrator (rather than carried by v1 data)?
pub fn scope_estimated(estimated: &[String]) -> bool {
    estimated.iter().any(|e| e == "scope")
}

pub fn breadcrumb(scope: &ScopeV2, estimated: &[String]) -> Breadcrumb {
    let mut segs = vec![
        BreadcrumbSeg {
            level: ScopeLevel::Domain,
            text: scope.domain.as_str().to_owned(),
        },
        BreadcrumbSeg {
            level: ScopeLevel::Layer,
            text: scope.layer.clone(),
        },
    ];
    if let Some(component) = &scope.component {
        segs.push(BreadcrumbSeg {
            level: ScopeLevel::Component,
            text: component.clone(),
        });
    }
    if !scope.surface.is_empty() {
        segs.push(BreadcrumbSeg {
            level: ScopeLevel::Surface,
            text: scope.surface.join("+"),
        });
    }
    Breadcrumb {
        no_surface: scope.component.is_some() && scope.surface.is_empty(),
        estimated: scope_estimated(estimated),
        segs,
    }
}

impl Breadcrumb {
    /// Plain-text path — the four scope fields joined (`repo › docs`,
    /// `website › frontend › mission_creator › a+b`). The chip path paints `segs`
    /// individually; this is the canonical string form (tests + hover copy).
    pub fn label(&self) -> String {
        self.segs
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(&format!(" {SCOPE_SEP} "))
    }
}

// ---- work-ticket class (T-918.1 / B.1) ----

/// The closed class set, mirrored from [`ticket_engine::CLASS_VALUES`] (parity is
/// test-pinned). An enum so the chip accent match below is TOTAL — a 6th class
/// fails compile here before it can ever render unstyled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Bug,
    Feature,
    Chore,
    Audit,
    Docs,
}

impl Class {
    pub const ALL: [Class; 5] = [
        Class::Bug,
        Class::Feature,
        Class::Chore,
        Class::Audit,
        Class::Docs,
    ];

    pub fn parse(s: &str) -> Option<Class> {
        Some(match s {
            "bug" => Class::Bug,
            "feature" => Class::Feature,
            "chore" => Class::Chore,
            "audit" => Class::Audit,
            "docs" => Class::Docs,
            _ => return None,
        })
    }

    #[deny(clippy::wildcard_enum_match_arm)]
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Bug => "bug",
            Class::Feature => "feature",
            Class::Chore => "chore",
            Class::Audit => "audit",
            Class::Docs => "docs",
        }
    }

    /// Chip accent rgb — the app.rs status palette hues, reused so class chips
    /// read in the board's existing color language: bug = cancelled red,
    /// feature = queued blue, chore = idea gray, audit = review purple,
    /// docs = shipped green. Total match (deny above the fn): adding a class
    /// variant without an accent does not compile.
    #[deny(clippy::wildcard_enum_match_arm)]
    pub fn accent_rgb(self) -> (u8, u8, u8) {
        match self {
            Class::Bug => (215, 115, 105),
            Class::Feature => (120, 165, 225),
            Class::Chore => (150, 150, 150),
            Class::Audit => (195, 150, 235),
            Class::Docs => (105, 150, 115),
        }
    }
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
    /// T-917.2 per-ticket plan document path (T-918.4 renders it clickable —
    /// the in-app viewer's primary use case).
    pub plan: Option<&'a str>,
    pub depends_on: &'a [String],
    pub unblocks: &'a [String],
    pub parent: Option<&'a str>,
    pub children: &'a [String],
    pub active: Option<&'a str>,
    pub main_goal: Option<&'a str>,
    /// T-917.2 body decomposition lists (spec §Body) — the detail panel renders
    /// these as the ten pinned sections (T-918.3, `detail::body_field_order`).
    pub context: &'a [String],
    pub requirement: &'a [String],
    pub current_state: &'a [String],
    pub approach: &'a [String],
    pub verify: &'a [String],
    pub acceptance: &'a [String],
    pub citations: &'a [String],
    /// T-917.3 quarantined wall prose — NOT one of the ten body fields; the
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
    /// Provenance markers ([`ticket_engine::ESTIMATED_VALUES`]) — B.1 consumes only
    /// the `"scope"` entry (the breadcrumb glyph); B.2 renders the rest.
    pub estimated: &'a [String],
    /// The method note behind `estimated[]` (T-918.2) — rendered VERBATIM as the
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

#[cfg(test)]
#[path = "tests/board_tests.rs"]
mod tests;
