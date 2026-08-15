//! Board projection (T-915.1 §UI shape) — pure, unit-tested, no egui types.
//!
//! Buckets the corpus into the 8 raw `StatusName` columns, sorts cards by
//! `(order, id)`, and precomputes every card label at load time so the paint path
//! never formats strings.

use std::collections::HashMap;

use tbd_tickets::{ScopeV2, Status, StatusName, Ticket};

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

/// The closed class set, mirrored from [`tbd_tickets::CLASS_VALUES`] (parity is
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
    pub depends_on: &'a [String],
    pub unblocks: &'a [String],
    pub parent: Option<&'a str>,
    pub children: &'a [String],
    pub active: Option<&'a str>,
    pub user_story: Option<&'a str>,
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
    /// Provenance markers ([`tbd_tickets::ESTIMATED_VALUES`]) — B.1 consumes only
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
            depends_on: &p.depends_on,
            unblocks: &p.unblocks,
            parent: None,
            children: &p.children,
            active: p.active.as_deref(),
            user_story: p.user_story.as_deref(),
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
            depends_on: &w.depends_on,
            unblocks: &w.unblocks,
            parent: w.parent.as_deref(),
            children: EMPTY_IDS,
            active: None,
            user_story: w.user_story.as_deref(),
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
mod tests {
    use super::*;
    use crate::corpus::{Corpus, Counts, LoadedTicket, is_child_id};
    use std::path::PathBuf;

    fn parse(toml: &str) -> Ticket {
        tbd_tickets::parse_ticket_toml(toml).unwrap()
    }

    fn work(id: &str, status_lines: &str, extra: &str) -> Ticket {
        parse(&format!(
            r#"id = "{id}"
kind = "work"
title = "title of {id}"
{status_lines}
{extra}
[scope]
domain = "repo"
layer = "docs"
"#
        ))
    }

    fn corpus_of(tickets: Vec<Ticket>) -> Corpus {
        let tickets: Vec<LoadedTicket> = tickets
            .into_iter()
            .map(|ticket| {
                let path = PathBuf::from(format!("{}.toml", ticket.id()));
                LoadedTicket { ticket, path }
            })
            .collect();
        let parents = tickets
            .iter()
            .filter(|t| !is_child_id(t.ticket.id()))
            .count();
        let children = tickets.len() - parents;
        Corpus {
            counts: Counts {
                total: tickets.len(),
                parents,
                children,
            },
            tickets,
        }
    }

    fn column_ids(board: &BoardModel, status: StatusName) -> Vec<&str> {
        board.columns[column_of(status)]
            .cards
            .iter()
            .map(|c| c.id.as_str())
            .collect()
    }

    #[test]
    fn status_order_matches_column_of() {
        for (i, status) in STATUS_ORDER.iter().enumerate() {
            assert_eq!(column_of(*status), i);
        }
        assert_eq!(STATUS_ORDER[0].as_str(), "idea");
        assert_eq!(STATUS_ORDER[7].as_str(), "cancelled");
    }

    #[test]
    fn bucketing_headers_and_chips() {
        let corpus = corpus_of(vec![
            work("T-1", "status = \"idea\"", ""),
            work("T-2", "status = \"queued\"\norder = 10", ""),
            work("T-3", "status = \"queued\"\norder = 20", ""),
            work("T-4", "status = \"shipped\"", ""),
        ]);
        let board = BoardModel::build(&corpus);
        assert_eq!(column_ids(&board, StatusName::Idea), vec!["T-1"]);
        assert_eq!(column_ids(&board, StatusName::Queued), vec!["T-2", "T-3"]);
        assert_eq!(column_ids(&board, StatusName::Shipped), vec!["T-4"]);
        assert!(column_ids(&board, StatusName::Ready).is_empty());
        let queued = &board.columns[column_of(StatusName::Queued)];
        assert_eq!(queued.header, "queued · 2");
        assert_eq!(queued.chip, "queued\n2");
    }

    #[test]
    fn cards_sort_by_order_then_numeric_id() {
        let corpus = corpus_of(vec![
            work("T-2", "status = \"queued\"\norder = 30", ""),
            work("T-3", "status = \"queued\"\norder = 10", ""),
            work("T-1", "status = \"queued\"\norder = 10", ""),
        ]);
        let board = BoardModel::build(&corpus);
        // order 10 first (T-1 before T-3 on id), order 30 last.
        assert_eq!(
            column_ids(&board, StatusName::Queued),
            vec!["T-1", "T-3", "T-2"]
        );
    }

    #[test]
    fn ideas_sort_by_numeric_id_not_string() {
        let corpus = corpus_of(vec![
            work("T-100", "status = \"idea\"", ""),
            work("T-9", "status = \"idea\"", ""),
            work("T-10", "status = \"idea\"", ""),
        ]);
        let board = BoardModel::build(&corpus);
        assert_eq!(
            column_ids(&board, StatusName::Idea),
            vec!["T-9", "T-10", "T-100"]
        );
    }

    #[test]
    fn child_ids_sort_numerically_within_ties() {
        let corpus = corpus_of(vec![
            work("T-915.10", "status = \"queued\"\norder = 5", ""),
            work("T-915.2", "status = \"queued\"\norder = 5", ""),
        ]);
        let board = BoardModel::build(&corpus);
        assert_eq!(
            column_ids(&board, StatusName::Queued),
            vec!["T-915.2", "T-915.10"]
        );
    }

    #[test]
    fn unparsable_ids_sort_last() {
        let (segs, _) = id_sort_key("T-abc");
        assert_eq!(segs, vec![u64::MAX]);
        let (segs, _) = id_sort_key("T-915.1");
        assert_eq!(segs, vec![915, 1]);
    }

    #[test]
    fn executor_chip_defaults_to_claude_code() {
        let corpus = corpus_of(vec![
            work("T-1", "status = \"idea\"", ""),
            work("T-2", "status = \"idea\"", "executor = \"cursor-docs\"\n"),
        ]);
        let board = BoardModel::build(&corpus);
        let idea = &board.columns[column_of(StatusName::Idea)];
        assert_eq!(idea.cards[0].executor, "claude-code");
        assert_eq!(idea.cards[1].executor, "cursor-docs");
    }

    #[test]
    fn order_label_precomputed() {
        let corpus = corpus_of(vec![
            work("T-1", "status = \"queued\"\norder = 5961", ""),
            work("T-2", "status = \"idea\"", ""),
        ]);
        let board = BoardModel::build(&corpus);
        assert_eq!(
            board.columns[column_of(StatusName::Queued)].cards[0].order_label,
            "#5961"
        );
        assert_eq!(
            board.columns[column_of(StatusName::Idea)].cards[0].order_label,
            ""
        );
    }

    #[test]
    fn id_to_index_resolves_every_ticket() {
        let corpus = corpus_of(vec![
            work("T-1", "status = \"idea\"", ""),
            work("T-2.1", "status = \"idea\"", ""),
        ]);
        let board = BoardModel::build(&corpus);
        assert_eq!(board.id_to_index.len(), 2);
        let i = board.id_to_index["T-2.1"];
        assert_eq!(corpus.tickets[i].ticket.id(), "T-2.1");
    }

    #[test]
    fn truncate_chars_is_char_safe() {
        assert_eq!(truncate_chars("short", 10), "short");
        let long = "ünïcödé".repeat(20);
        let cut = truncate_chars(&long, 12);
        assert_eq!(cut.chars().count(), 12);
        assert!(cut.ends_with('…'));
    }

    fn work_scope(scope: &ScopeV2, estimated: &[String]) -> Breadcrumb {
        breadcrumb(scope, estimated)
    }

    fn scope_of(t: &Ticket) -> &ScopeV2 {
        match t {
            Ticket::Work(w) => &w.scope,
            Ticket::Program(_) => unreachable!("work builder"),
        }
    }

    /// T-918.1: breadcrumb string assembly over all four v2 levels — empty
    /// component is SKIPPED (`domain › layer`, no marker: component-free layers
    /// are complete); empty surface UNDER a component sets `no_surface` (the
    /// detail-panel `(no surface)` marker; cards omit it).
    #[test]
    fn breadcrumb_assembly_variants() {
        // No component (⇒ no surface tier): two segments, no marker.
        let repo = work("T-1", "status = \"idea\"", "");
        let bc = work_scope(scope_of(&repo), &[]);
        assert_eq!(bc.label(), "repo › docs");
        assert_eq!(
            bc.segs.iter().map(|s| s.level).collect::<Vec<_>>(),
            vec![ScopeLevel::Domain, ScopeLevel::Layer]
        );
        assert!(!bc.no_surface);
        assert!(!bc.estimated);

        // Component with an EMPTY surface list: marker flag on.
        let backend = parse(
            r#"id = "T-2"
kind = "work"
title = "b"
status = "idea"

[scope]
domain = "website"
layer = "backend"
component = "http_api"
"#,
        );
        let bc = work_scope(scope_of(&backend), &[]);
        assert_eq!(bc.label(), "website › backend › http_api");
        assert!(bc.no_surface, "component present + no surface ⇒ marker");

        // Full four-level path: surfaces join into ONE segment.
        let editor = parse(
            r#"id = "T-3"
kind = "work"
title = "e"
status = "idea"

[scope]
domain = "website"
layer = "frontend"
component = "mission_creator"
surface = ["dock_left", "map_canvas"]
"#,
        );
        let bc = work_scope(scope_of(&editor), &[]);
        assert_eq!(
            bc.label(),
            "website › frontend › mission_creator › dock_left+map_canvas"
        );
        assert_eq!(bc.segs.len(), 4);
        assert_eq!(bc.segs[3].level, ScopeLevel::Surface);
        assert!(!bc.no_surface);
    }

    /// The estimated-scope marker predicate: exactly the `"scope"` entry, nothing
    /// else, flips the breadcrumb glyph.
    #[test]
    fn breadcrumb_estimated_scope_marker() {
        assert!(scope_estimated(&["scope".to_owned()]));
        assert!(scope_estimated(&["tokens".to_owned(), "scope".to_owned()]));
        assert!(!scope_estimated(&["tokens".to_owned()]));
        assert!(!scope_estimated(&[]));

        let inferred = work(
            "T-1",
            "status = \"idea\"",
            "estimated = [\"scope\"]\nestimate_note = \"owns-inferred\"\n",
        );
        assert!(work_scope(scope_of(&inferred), estimated_of(&inferred)).estimated);
        let carried = work("T-2", "status = \"idea\"", "estimated = [\"tokens\"]\n");
        assert!(!work_scope(scope_of(&carried), estimated_of(&carried)).estimated);
    }

    fn estimated_of(t: &Ticket) -> &[String] {
        match t {
            Ticket::Work(w) => &w.estimated,
            Ticket::Program(p) => &p.estimated,
        }
    }

    /// T-918.1: class enum stays in lockstep with the registry's closed set, and
    /// the chip accents are total + pairwise distinct (the deny-wildcard matches
    /// in `as_str`/`accent_rgb` make a 6th enum variant a compile error).
    #[test]
    fn class_parity_and_total_distinct_accents() {
        assert_eq!(
            Class::ALL.map(Class::as_str).to_vec(),
            tbd_tickets::CLASS_VALUES.to_vec(),
            "Class::ALL must mirror tbd_tickets::CLASS_VALUES exactly"
        );
        for class in Class::ALL {
            assert_eq!(Class::parse(class.as_str()), Some(class));
        }
        assert_eq!(Class::parse("epic"), None);
        assert_eq!(Class::parse("Bug"), None, "raw lowercase values only");
        let mut accents: Vec<_> = Class::ALL.iter().map(|c| c.accent_rgb()).collect();
        accents.sort_unstable();
        accents.dedup();
        assert_eq!(accents.len(), Class::ALL.len(), "chip accents must differ");
    }

    /// Cards precompute breadcrumb + class chip; programs carry neither.
    #[test]
    fn cards_precompute_breadcrumb_and_class() {
        let corpus = corpus_of(vec![
            work("T-1", "status = \"idea\"", "class = \"bug\"\n"),
            parse(
                r#"id = "T-9"
kind = "program"
title = "prog"
summary = "s"
status = "idea"
children = ["T-9.1"]
"#,
            ),
        ]);
        let board = BoardModel::build(&corpus);
        let idea = &board.columns[column_of(StatusName::Idea)];
        let work_card = idea.cards.iter().find(|c| c.id == "T-1").unwrap();
        assert_eq!(work_card.class, Some(Class::Bug));
        assert_eq!(
            work_card.breadcrumb.as_ref().unwrap().label(),
            "repo › docs"
        );
        let program_card = idea.cards.iter().find(|c| c.id == "T-9").unwrap();
        assert_eq!(program_card.class, None, "no class ⇒ no chip");
        assert!(program_card.breadcrumb.is_none(), "programs have no scope");
    }

    #[test]
    fn status_label_forms() {
        let idea = work("T-1", "status = \"idea\"", "");
        assert_eq!(status_label(idea.status()), "idea");
        let queued = work("T-2", "status = \"queued\"\norder = 5", "");
        assert_eq!(status_label(queued.status()), "queued · #5");
        let shipped = work("T-3", "status = \"shipped\"", "");
        assert_eq!(status_label(shipped.status()), "shipped");
    }

    #[test]
    fn view_maps_both_kinds() {
        let program = parse(
            r#"id = "T-9"
kind = "program"
title = "prog"
summary = "sum"
status = "queued"
order = 40
children = ["T-9.1", "T-9.2"]
active = "T-9.1"
"#,
        );
        let v = view(&program);
        assert_eq!(v.kind, "program");
        assert_eq!(v.children, &["T-9.1".to_string(), "T-9.2".to_string()][..]);
        assert_eq!(v.active, Some("T-9.1"));
        assert_eq!(v.parent, None);
        assert!(v.scope.is_none());
        assert_eq!(v.class, None);
        assert!(v.estimated.is_empty());

        let workt = work(
            "T-9.1",
            "status = \"shipped\"\nshipped_at = \"abc123\"",
            "parent = \"T-9\"\nclass = \"chore\"\nestimated = [\"scope\"]\n\
             context = [\"why now\"]\nrequirement = [\"the ask\"]\n\
             current_state = [\"what exists\"]\napproach = [\"step 1\"]\n\
             verify = [\"cargo test\"]\ncitations = [\"docs/x.md\"]\n\
             migration_legacy = [\"wall line\"]\n",
        );
        let v = view(&workt);
        assert_eq!(v.kind, "work");
        assert_eq!(v.parent, Some("T-9"));
        assert_eq!(v.shipped_at, Some("abc123"));
        assert!(v.children.is_empty());
        // T-918.3: the body lists + quarantine reach the view verbatim.
        assert_eq!(v.context, &["why now".to_string()][..]);
        assert_eq!(v.requirement, &["the ask".to_string()][..]);
        assert_eq!(v.current_state, &["what exists".to_string()][..]);
        assert_eq!(v.approach, &["step 1".to_string()][..]);
        assert_eq!(v.verify, &["cargo test".to_string()][..]);
        assert_eq!(v.citations, &["docs/x.md".to_string()][..]);
        assert_eq!(v.migration_legacy, &["wall line".to_string()][..]);
        let bc = breadcrumb(v.scope.unwrap(), v.estimated);
        assert_eq!(bc.label(), "repo › docs");
        assert!(bc.estimated);
        assert_eq!(v.class, Some("chore"));
        assert_eq!(kind_str(&workt), "work");
    }
}
