//! Waves view model (T-915.2 §UI shape) — pure, unit-tested, no egui types.
//!
//! Projects the parsed `WaveLock` VERBATIM: one lane per open wave (`n > 0`) in lock
//! order, tickets in the lock's array order — never sorted, never repacked. Wave 0 is
//! always a count chip ("N parked"). The only derived surface is the "Unplanned"
//! bucket: dispatchable corpus ids absent from every lock wave — pure set arithmetic
//! between the ticket files and the lock, labeled as file-derived in the UI.

use std::collections::{HashMap, HashSet};

use tbd_tickets::{StatusName, Ticket};

use crate::board;
use crate::corpus::Corpus;
use crate::wavelock::WaveLock;

/// Tooltip for a lock id with no ticket file — visibly flagged, display-only, no
/// judgment (lock-vs-tickets drift is the trust banner's job, T-915.3).
pub const NO_FILE_TOOLTIP: &str = "no ticket file — rendered as recorded in the lock";

/// One ticket chip, precomputed at load. Status color and tooltip resolve through
/// the corpus when the id exists; `corpus_index == None` marks a lock id with no
/// ticket file (struck through in the UI).
pub struct WaveChip {
    pub id: String,
    pub corpus_index: Option<usize>,
    pub status: Option<StatusName>,
    pub tooltip: String,
}

pub struct Lane {
    pub n: u32,
    /// `"wave 133 · 8"` — precomputed.
    pub label: String,
    pub chips: Vec<WaveChip>,
    /// `"n<TAB>id\n"` per ticket — the copy-lane acceptance surface.
    pub tsv: String,
}

/// Wave 0 — ALWAYS collapsed to the count chip; the id list only renders on demand.
pub struct Wave0 {
    /// `"1090 parked"` — the count chip text.
    pub label: String,
    pub chips: Vec<WaveChip>,
    pub tsv: String,
}

pub struct WavesModel {
    /// `"wave_base 132 · max_concurrent 8"` — straight off the lock.
    pub header: String,
    pub pack_last: Vec<WaveChip>,
    /// Open lanes (`n > 0`), lock order, tickets verbatim.
    pub lanes: Vec<Lane>,
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
        .filter(|t| dispatchable(&t.ticket) && !locked.contains(t.ticket.id()))
        .map(|t| t.ticket.id().to_owned())
        .collect();
    ids.sort_by_key(|id| board::id_sort_key(id));
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
                id: id.to_owned(),
                corpus_index: Some(index),
                status: Some(t.status().name()),
                tooltip: board::title_of(t).to_owned(),
            }
        }
        None => WaveChip {
            id: id.to_owned(),
            corpus_index: None,
            status: None,
            tooltip: NO_FILE_TOOLTIP.to_owned(),
        },
    }
}

impl WavesModel {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{corpus_of, index_of, program, work};
    use crate::wavelock::LockWave;
    use std::collections::BTreeMap;

    fn lock(waves: Vec<(u32, Vec<&str>)>) -> WaveLock {
        WaveLock {
            version: 1,
            max_concurrent: 8,
            wave_base: 132,
            pack_last: vec!["T-290".to_string()],
            waves: waves
                .into_iter()
                .map(|(n, tickets)| LockWave {
                    n,
                    tickets: tickets.into_iter().map(str::to_owned).collect(),
                })
                .collect(),
            owns: BTreeMap::new(),
            depends_on: BTreeMap::new(),
        }
    }

    #[test]
    fn dispatchable_mirrors_the_xtask_rule() {
        // Work + live status + default executor ⇒ dispatchable.
        assert!(dispatchable(&work(
            "T-1",
            "status = \"queued\"\norder = 5",
            ""
        )));
        assert!(dispatchable(&work(
            "T-2",
            "status = \"ready\"\norder = 5\nspec = \"docs/x.md\"\nuser_story = \"u\"\nacceptance = [\"a\"]",
            "executor = \"claude-code\"\n",
        )));
        // Dead statuses are not dispatchable.
        assert!(!dispatchable(&work("T-3", "status = \"idea\"", "")));
        assert!(!dispatchable(&work("T-4", "status = \"shipped\"", "")));
        assert!(!dispatchable(&work("T-5", "status = \"deferred\"", "")));
        // Non-claude-code executor is operator work.
        assert!(!dispatchable(&work(
            "T-6",
            "status = \"queued\"\norder = 5",
            "executor = \"cursor-docs\"\n",
        )));
        // Programs are never dispatchable.
        assert!(!dispatchable(&program(
            "T-7",
            "status = \"queued\"\norder = 5",
            &["T-7.1"],
        )));
    }

    #[test]
    fn unplanned_is_pure_set_arithmetic() {
        let corpus = corpus_of(vec![
            // In an open wave — planned.
            work("T-1", "status = \"queued\"\norder = 10", ""),
            // Dispatchable and absent from every lock wave — THE unplanned id.
            work("T-2", "status = \"queued\"\norder = 20", ""),
            // Parked (shipped) and absent — not unplanned.
            work("T-3", "status = \"shipped\"", ""),
            // Non-claude-code executor and absent — not unplanned.
            work(
                "T-4",
                "status = \"queued\"\norder = 30",
                "executor = \"cursor-docs\"\n",
            ),
            // Program and absent — not unplanned.
            program("T-5", "status = \"queued\"\norder = 40", &["T-5.1"]),
            // Dispatchable but sitting in wave 0 — in a lock wave, so not unplanned.
            work("T-6", "status = \"queued\"\norder = 50", ""),
            // Two more unplanned ids to pin numeric ordering.
            work("T-10", "status = \"queued\"\norder = 60", ""),
            work("T-9", "status = \"queued\"\norder = 70", ""),
        ]);
        let lock = lock(vec![(0, vec!["T-6"]), (133, vec!["T-1"])]);
        assert_eq!(unplanned_ids(&corpus, &lock), vec!["T-2", "T-9", "T-10"]);
    }

    #[test]
    fn lanes_render_the_lock_verbatim_never_sorted() {
        let corpus = corpus_of(vec![
            work("T-2", "status = \"queued\"\norder = 10", ""),
            work(
                "T-9",
                "status = \"running\"\norder = 20\nspec = \"docs/x.md\"\nuser_story = \"u\"\nacceptance = [\"a\"]",
                "",
            ),
        ]);
        let ids = index_of(&corpus);
        // Deliberately unsorted lane: the model must preserve the lock's order.
        let lock = lock(vec![(0, vec!["T-2"]), (134, vec!["T-9", "T-2"])]);
        let model = WavesModel::build(&corpus, &ids, &lock);
        assert_eq!(model.lanes.len(), 1, "wave 0 is never a lane");
        assert_eq!(model.lanes[0].n, 134);
        assert_eq!(model.lanes[0].label, "wave 134 · 2");
        let chip_ids: Vec<&str> = model.lanes[0].chips.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(chip_ids, vec!["T-9", "T-2"]);
        let w0 = model.wave0.expect("wave 0 present in lock");
        assert_eq!(w0.chips.len(), 1);
        assert_eq!(w0.label, "1 parked");
        assert_eq!(w0.tsv, "0\tT-2\n");
        assert_eq!(model.header, "wave_base 132 · max_concurrent 8");
        assert_eq!(model.pack_last.len(), 1);
        assert_eq!(model.pack_last[0].id, "T-290");
    }

    #[test]
    fn lock_id_without_ticket_file_is_flagged() {
        let corpus = corpus_of(vec![work("T-1", "status = \"queued\"\norder = 10", "")]);
        let ids = index_of(&corpus);
        let lock = lock(vec![(133, vec!["T-1", "T-404"])]);
        let model = WavesModel::build(&corpus, &ids, &lock);
        let chips = &model.lanes[0].chips;
        assert_eq!(chips[0].corpus_index, Some(0));
        assert_eq!(chips[0].status, Some(StatusName::Queued));
        assert_eq!(chips[0].tooltip, "title of T-1");
        // The missing id stays in the lane (verbatim), flagged for the paint path.
        assert_eq!(chips[1].id, "T-404");
        assert_eq!(chips[1].corpus_index, None);
        assert_eq!(chips[1].status, None);
        assert_eq!(chips[1].tooltip, NO_FILE_TOOLTIP);
    }

    #[test]
    fn lane_tsv_format_is_n_tab_id_lines() {
        let tickets = vec!["T-146".to_string(), "T-090.4".to_string()];
        assert_eq!(lane_tsv(133, &tickets), "133\tT-146\n133\tT-090.4\n");
        assert_eq!(lane_tsv(0, &["T-001".to_string()]), "0\tT-001\n");
        assert_eq!(lane_tsv(7, &[]), "");
    }
}
