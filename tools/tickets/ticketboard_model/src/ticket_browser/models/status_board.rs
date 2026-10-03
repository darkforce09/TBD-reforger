//! The status columns and their precomputed cards.
//!
//! **Role:** `BoardModel`, `Column` and `Card`: every ticket in the column of its status, sorted by
//! order then numeric id, with the card text precomputed.
//! **Position:** built by `WorkspaceState::new` from the corpus; painted by the desktop application's
//! status board.
//! **Signals & state:** none; built once per load.
//! **Invariants:** cards sort by order then numeric id, tickets without an order last; ids that do
//! not parse sort last.

use crate::ticket_registry::{models::corpus::Corpus, models::projection::*};
use std::collections::HashMap;
use ticket_model::{StatusName, Ticket, TicketId};
/// Card title truncation bound, in chars (precomputed; card rows never wrap).
const TITLE_MAX_CHARS: usize = 48;

/// Everything the card paint path needs, precomputed at load.
pub struct Card {
    /// Index into `Corpus::tickets`.
    pub index: usize,
    /// The ticket id.
    pub id: TicketId,
    /// The title, truncated for the card.
    pub title: String,
    /// The executor label.
    pub executor: String,
    /// `"#5961"` when the ticket carries an order, else empty.
    pub order_label: String,
    /// Scope breadcrumb (work tickets; programs carry no scope). Cards render the
    /// compact form — no [`NO_SURFACE_MARKER`] (detail-panel only).
    pub breadcrumb: Option<Breadcrumb>,
    /// Class chip accent (absent class — programs, pre-triage work — no chip).
    pub class: Option<Class>,
    /// Hover tooltip: the ticket's main_goal via [`card_tooltip`] —
    /// the goal surfaces on hover without opening the detail panel. Absent or
    /// blank ⇒ `None` (no empty tooltip bubble).
    pub tooltip: Option<String>,
}

/// One status column.
pub struct Column {
    /// The status the column holds.
    pub status: StatusName,
    /// `"queued · 173"` — precomputed header.
    pub header: String,
    /// `"shipped\n743"` — precomputed count-chip label for collapsed columns.
    pub chip: String,
    /// The cards, in order key then id order.
    pub cards: Vec<Card>,
}

/// The status board: one column per status, in display order.
pub struct BoardModel {
    /// The columns.
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

impl BoardModel {
    /// Builds the board over `corpus`.
    pub fn build(corpus: &Corpus) -> Self {
        let mut buckets: [Vec<(SortKey, Card)>; 8] = Default::default();
        let mut id_to_index = HashMap::with_capacity(corpus.tickets.len());
        for (index, loaded) in corpus.tickets.iter().enumerate() {
            let t = &loaded.ticket;
            id_to_index.insert(t.id().to_string(), index);
            let card = Card {
                index,
                id: t.id().clone(),
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

#[cfg(test)]
#[path = "tests/status_board.rs"]
mod tests;
