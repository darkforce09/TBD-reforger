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

/// One-line scope rendering for the detail panel — the v2 breadcrumb:
/// `domain/layer[/component][: s1+s2]`. Kept deliberately raw (T-917.2 compile-fix
/// tier); the faceted rendering lands with Ticketboard v2 B.1.
pub fn scope_compact(scope: &ScopeV2) -> String {
    let mut s = format!("{}/{}", scope.domain.as_str(), scope.layer);
    if let Some(component) = &scope.component {
        s.push('/');
        s.push_str(component);
    }
    if !scope.surface.is_empty() {
        s.push_str(": ");
        s.push_str(&scope.surface.join("+"));
    }
    s
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
    pub acceptance: &'a [String],
    pub shipped_at: Option<&'a str>,
    pub priority: Option<i64>,
    pub created_at: Option<&'a str>,
    pub completed_at: Option<&'a str>,
    pub owns: &'a [String],
    pub pack_last: Option<bool>,
    /// Compact scope line (work tickets only; programs forbid `[scope]`).
    pub scope: Option<String>,
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
            acceptance: &p.acceptance,
            shipped_at,
            priority: p.priority,
            created_at: p.created_at.as_deref(),
            completed_at: p.completed_at.as_deref(),
            owns: &p.owns,
            pack_last: p.pack_last,
            scope: None,
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
            acceptance: &w.acceptance,
            shipped_at,
            priority: w.priority,
            created_at: w.created_at.as_deref(),
            completed_at: w.completed_at.as_deref(),
            owns: &w.owns,
            pack_last: w.pack_last,
            scope: Some(scope_compact(&w.scope)),
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

    /// T-917.2: the breadcrumb renders all four v2 levels, omitting what is absent.
    #[test]
    fn scope_compact_variants() {
        let repo = work("T-1", "status = \"idea\"", "");
        match &repo {
            Ticket::Work(w) => assert_eq!(scope_compact(&w.scope), "repo/docs"),
            Ticket::Program(_) => unreachable!(),
        }

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
        match &backend {
            Ticket::Work(w) => {
                assert_eq!(scope_compact(&w.scope), "website/backend/http_api");
            }
            Ticket::Program(_) => unreachable!(),
        }

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
        match &editor {
            Ticket::Work(w) => {
                assert_eq!(
                    scope_compact(&w.scope),
                    "website/frontend/mission_creator: dock_left+map_canvas"
                );
            }
            Ticket::Program(_) => unreachable!(),
        }
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

        let workt = work(
            "T-9.1",
            "status = \"shipped\"\nshipped_at = \"abc123\"",
            "parent = \"T-9\"\n",
        );
        let v = view(&workt);
        assert_eq!(v.kind, "work");
        assert_eq!(v.parent, Some("T-9"));
        assert_eq!(v.shipped_at, Some("abc123"));
        assert!(v.children.is_empty());
        assert_eq!(v.scope.as_deref(), Some("repo/docs"));
        assert_eq!(kind_str(&workt), "work");
    }
}
