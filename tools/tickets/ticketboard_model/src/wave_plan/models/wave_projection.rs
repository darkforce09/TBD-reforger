//! Wave lanes projected verbatim from the recorded lock.
//!
//! Missing ticket files remain visible as flagged chips. Unplanned tickets are derived
//! from corpus membership without recomputing or reordering recorded waves.

use std::collections::{HashMap, HashSet};

use ticket_model::{StatusName, Ticket, TicketId};

use crate::ticket_registry::models::corpus::Corpus;
use crate::ticket_registry::models::projection as board;
use crate::wave_plan::services::lock_file::WaveLock;

/// Tooltip for a lock id with no ticket file — visibly flagged, display-only, no
/// judgment (lock-vs-tickets drift is the trust banner's job, ).
pub const NO_FILE_TOOLTIP: &str = "no ticket file — rendered as recorded in the lock";

/// One ticket chip, precomputed at load. Status color and tooltip resolve through
/// the corpus when the id exists; `corpus_index == None` marks a lock id with no
/// ticket file (struck through in the UI).
pub struct WaveChip {
    /// The ticket id as recorded.
    pub id: TicketId,
    /// The ticket's corpus index; `None` when no ticket file has the id.
    pub corpus_index: Option<usize>,
    /// The ticket's status, when its file exists.
    pub status: Option<StatusName>,
    /// The chip tooltip.
    pub tooltip: String,
}

/// One open wave.
pub struct Lane {
    /// The wave number.
    pub n: u32,
    /// `"wave 133 · 8"` — precomputed.
    pub label: String,
    /// The wave's tickets, in lock order.
    pub chips: Vec<WaveChip>,
    /// `"n<TAB>id\n"` per ticket — the copy-lane acceptance surface.
    pub tsv: String,
}

/// Wave 0 — ALWAYS collapsed to the count chip; the id list only renders on demand.
pub struct Wave0 {
    /// `"1090 parked"` — the count chip text.
    pub label: String,
    /// The parked tickets, in lock order.
    pub chips: Vec<WaveChip>,
    /// The `n<TAB>id` lines of the copy action.
    pub tsv: String,
}

/// The Waves tab: the header, the pack-last tickets, the open lanes and wave 0, as recorded.
pub struct WavesModel {
    /// `"wave_base 132 · max_concurrent 8"` — straight off the lock.
    pub header: String,
    /// The `pack_last` tickets.
    pub pack_last: Vec<WaveChip>,
    /// Open lanes (`n > 0`), lock order, tickets verbatim.
    pub lanes: Vec<Lane>,
    /// Wave 0, when the lock records one.
    pub wave0: Option<Wave0>,
    /// Derived-from-files side bucket, sorted by numeric id — NOT lock data.
    pub unplanned: Vec<WaveChip>,
}

/// Mirror of xtask `TicketView::dispatchable`: kind work AND live status
/// (queued/ready/running/review) AND executor (default claude-code) == claude-code.
pub fn dispatchable(t: &Ticket) -> bool {
    matches!(t, Ticket::Work(_))
        && t.status().name().is_live()
        && board::executor_of(t).unwrap_or(board::EXECUTOR_DEFAULT) == board::EXECUTOR_DEFAULT
}

/// Dispatchable corpus ids absent from EVERY lock wave (wave 0 included) — pure set
/// arithmetic; lane membership is never re-derived from ticket status.
pub fn unplanned_ids(corpus: &Corpus, lock: &WaveLock) -> Vec<String> {
    let locked: HashSet<&str> = lock
        .waves
        .iter()
        .flat_map(|w| w.tickets.iter().map(String::as_str))
        .collect();
    let mut ids: Vec<String> = corpus
        .tickets
        .iter()
        .filter(|t| dispatchable(&t.ticket) && !locked.contains(t.ticket.id().as_str()))
        .map(|t| t.ticket.id().to_string())
        .collect();
    ids.sort_by_cached_key(|id| board::id_sort_key(&TicketId::new(id.as_str())));
    ids
}

/// `"n<TAB>id"` lines, one per ticket in lock order, each newline-terminated — the
/// paste surface checked against the corresponding `wave.lock` block.
pub fn lane_tsv(n: u32, tickets: &[String]) -> String {
    tickets.iter().map(|id| format!("{n}\t{id}\n")).collect()
}

fn chip(id: &str, corpus: &Corpus, ids: &HashMap<String, usize>) -> WaveChip {
    match ids.get(id) {
        Some(&index) => {
            let t = &corpus.tickets[index].ticket;
            WaveChip {
                id: TicketId::new(id),
                corpus_index: Some(index),
                status: Some(t.status().name()),
                tooltip: board::title_of(t).to_owned(),
            }
        }
        None => WaveChip {
            id: TicketId::new(id),
            corpus_index: None,
            status: None,
            tooltip: NO_FILE_TOOLTIP.to_owned(),
        },
    }
}

impl WavesModel {
    /// Projects `lock` over `corpus`, resolving ids through `ids` (ticket id to corpus index).
    pub fn build(corpus: &Corpus, ids: &HashMap<String, usize>, lock: &WaveLock) -> Self {
        let chips_of = |list: &[String]| -> Vec<WaveChip> {
            list.iter().map(|id| chip(id, corpus, ids)).collect()
        };
        let lanes = lock
            .waves
            .iter()
            .filter(|w| w.n > 0)
            .map(|w| Lane {
                n: w.n,
                label: format!("wave {} · {}", w.n, w.tickets.len()),
                chips: chips_of(&w.tickets),
                tsv: lane_tsv(w.n, &w.tickets),
            })
            .collect();
        let wave0 = lock.waves.iter().find(|w| w.n == 0).map(|w| Wave0 {
            label: format!("{} parked", w.tickets.len()),
            chips: chips_of(&w.tickets),
            tsv: lane_tsv(0, &w.tickets),
        });
        let unplanned = unplanned_ids(corpus, lock)
            .iter()
            .map(|id| chip(id, corpus, ids))
            .collect();
        Self {
            header: format!(
                "wave_base {} · max_concurrent {}",
                lock.wave_base, lock.max_concurrent
            ),
            pack_last: chips_of(&lock.pack_last),
            lanes,
            wave0,
            unplanned,
        }
    }
}
