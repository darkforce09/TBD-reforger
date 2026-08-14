//! Test-only helpers: the scratch-dir guard (discovery + corpus + wavelock tests) and
//! tiny corpus builders shared by the waves / tree / filters model tests.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use tbd_tickets::Ticket;

use crate::corpus::{Corpus, Counts, LoadedTicket, is_child_id};

/// A unique scratch directory, removed on drop. Tests never chdir and never touch
/// the live repo — everything happens under the OS temp dir.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(tag: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ticketboard-test-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ---- corpus builders (waves / tree / filters model tests) ----

pub fn parse_ticket(toml: &str) -> Ticket {
    tbd_tickets::parse_ticket_toml(toml).unwrap()
}

/// Minimal work ticket. `status_lines` supplies `status = …` (plus `order` when the
/// status demands one); `extra` is any additional top-level lines (executor, parent,
/// owns, summary, …).
pub fn work(id: &str, status_lines: &str, extra: &str) -> Ticket {
    parse_ticket(&format!(
        r#"id = "{id}"
kind = "work"
title = "title of {id}"
{status_lines}
{extra}
[scope.repo]
layers = ["docs"]
"#
    ))
}

/// Minimal program ticket. The encoding refuses program-without-children, so callers
/// always name at least one child id (the child file need not exist — nothing checks
/// parent↔child at parse time).
pub fn program(id: &str, status_lines: &str, children: &[&str]) -> Ticket {
    let kids = children
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    parse_ticket(&format!(
        r#"id = "{id}"
kind = "program"
title = "program {id}"
summary = "summary of {id}"
{status_lines}
children = [{kids}]
"#
    ))
}

/// Wrap parsed tickets into a `Corpus` (paths synthesized from ids).
pub fn corpus_of(tickets: Vec<Ticket>) -> Corpus {
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

/// id → corpus index — the `BoardModel::id_to_index` map without building a board.
pub fn index_of(corpus: &Corpus) -> HashMap<String, usize> {
    corpus
        .tickets
        .iter()
        .enumerate()
        .map(|(i, t)| (t.ticket.id().to_owned(), i))
        .collect()
}
