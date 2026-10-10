//! The wave-level JSON documents `ttm --json` prints.
//!
//! **Role:** typed forms of the `ttm.wave/1`, `ttm.wave-repack/1`, `ttm.wave-check/1`,
//! `ttm.wave-close/1`, `ttm.wave-history/1` and `ttm.collisions/1` documents, and the questions
//! the wave drivers ask of a wave plan ([`WavePlan::open_wave`], [`WavePlan::row`],
//! [`WavePlan::is_complete`]).
//! **Position:** produced by [`crate::TicketManager`]'s wave calls; the platform and mod wave
//! drivers and the preflight read them.
//! **Signals & state:** none; plain data.
//! **Invariants:** a wave plan lists its open waves (labels above `0`) in ascending order and its
//! pending-close waves oldest first; a ticket counts as complete only when its status is
//! `shipped` or `cancelled`, so an unknown ticket or a missing status is never complete.

use serde::Deserialize;

use crate::ticket_documents::{TicketManagerDocument, document_formats};
use crate::ticket_references::{LegacyTicketNumber, TicketSlug};

document_formats! {
    WavePlan => "ttm.wave/1",
    RepackDocument => "ttm.wave-repack/1",
    WaveCheckDocument => "ttm.wave-check/1",
    WaveCloseDocument => "ttm.wave-close/1",
    WaveHistoryDocument => "ttm.wave-history/1",
    CollisionsDocument => "ttm.collisions/1",
}

/// The statuses that count a ticket as complete.
pub const COMPLETE_STATUSES: [&str; 2] = ["shipped", "cancelled"];

/// Whether `status` counts a ticket as complete (`shipped` or `cancelled`).
pub fn is_complete_status(status: &str) -> bool {
    COMPLETE_STATUSES.contains(&status)
}

/// One ticket of a wave, as `ttm wave show` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WaveRow {
    /// The ticket's slug, as the wave lock names it.
    pub slug: TicketSlug,
    /// False when the lock names something that is no longer a ticket.
    pub known: bool,
    /// The ticket's legacy number, when it was imported with one.
    #[serde(default)]
    pub legacy_id: Option<LegacyTicketNumber>,
    /// The ticket's title; empty for an unknown ticket.
    #[serde(default)]
    pub title: String,
    /// The ticket's status; none for an unknown ticket.
    #[serde(default)]
    pub status: Option<String>,
    /// The parent's slug, for a child slice.
    #[serde(default)]
    pub parent: Option<TicketSlug>,
    /// Who may take the ticket.
    #[serde(default)]
    pub executor: Option<String>,
    /// The repository paths the ticket owns.
    #[serde(default)]
    pub owns: Vec<String>,
    /// How many run receipts the ticket has.
    #[serde(default)]
    pub receipt_count: usize,
    /// The outcome of the newest receipt.
    #[serde(default)]
    pub latest_receipt_outcome: Option<String>,
    /// The commit the ticket landed in.
    #[serde(default)]
    pub landing_sha: Option<String>,
}

impl WaveRow {
    /// Whether the ticket is shipped or cancelled.
    pub fn is_complete(&self) -> bool {
        self.status.as_deref().is_some_and(is_complete_status)
    }

    /// Whether `reference` names this ticket: its slug, or its legacy number in any case.
    pub fn answers_to(&self, reference: &str) -> bool {
        self.slug.as_str() == reference
            || self
                .legacy_id
                .as_ref()
                .is_some_and(|number| number.as_str().eq_ignore_ascii_case(reference))
    }
}

/// One wave of a wave plan: its label and its tickets.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WaveEntry {
    /// The wave's label.
    pub n: u32,
    /// The wave's tickets, in plan order.
    pub tickets: Vec<WaveRow>,
}

/// `ttm wave show`: the project's wave plan.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WavePlan {
    /// The project.
    pub project: String,
    /// The label of the newest closed wave.
    pub wave_base: u32,
    /// The newest close the plan's numbering is seated on.
    pub ledger_floor: u32,
    /// How many tickets a wave holds at most.
    pub max_concurrent: usize,
    /// The open waves (labels above `0`), ascending.
    pub waves: Vec<WaveEntry>,
    /// The waves whose every ticket completed, pending their close, oldest first.
    #[serde(default)]
    pub emptied: Vec<WaveEntry>,
    /// The plan's one-line summary.
    #[serde(default)]
    pub summary: String,
}

impl WavePlan {
    /// The first open wave holding a ticket that is not complete; `None` when every open wave is
    /// complete.
    pub fn open_wave(&self) -> Option<&WaveEntry> {
        self.waves
            .iter()
            .filter(|wave| wave.n > 0)
            .find(|wave| wave.tickets.iter().any(|row| !row.is_complete()))
    }

    /// The open wave labelled `n`.
    pub fn wave(&self, n: u32) -> Option<&WaveEntry> {
        self.waves.iter().find(|wave| wave.n == n)
    }

    /// Every row of the open and pending-close waves, open waves first.
    pub fn rows(&self) -> impl Iterator<Item = &WaveRow> {
        self.waves
            .iter()
            .chain(self.emptied.iter())
            .flat_map(|wave| wave.tickets.iter())
    }

    /// The row `reference` names (slug or legacy number) in an open or pending-close wave.
    pub fn row(&self, reference: &str) -> Option<&WaveRow> {
        self.rows().find(|row| row.answers_to(reference))
    }

    /// Whether the ticket `reference` names is complete; `false` when the plan does not list it.
    pub fn is_complete(&self, reference: &str) -> bool {
        self.row(reference).is_some_and(WaveRow::is_complete)
    }

    /// How many tickets the open waves hold, and how many open waves there are.
    pub fn open_counts(&self) -> (usize, usize) {
        let open: Vec<&WaveEntry> = self.waves.iter().filter(|wave| wave.n > 0).collect();
        (open.iter().map(|wave| wave.tickets.len()).sum(), open.len())
    }
}

/// `ttm wave repack`: what the repack did.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RepackDocument {
    /// The new plan's one-line summary.
    pub summary: String,
    /// What the repack warned about.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Whether nothing was written.
    #[serde(default)]
    pub dry_run: bool,
}

/// `ttm wave check`: whether the stored wave plan agrees with the tickets and the close ledger.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WaveCheckDocument {
    /// Whether the check found nothing.
    pub ok: bool,
    /// Every finding.
    #[serde(default)]
    pub findings: Vec<String>,
}

/// `ttm wave close`: the close the ticket manager recorded.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WaveCloseDocument {
    /// The closed wave's label.
    pub n: u32,
    /// The marker commit recorded for the close.
    #[serde(default)]
    pub sha: Option<String>,
    /// When the close was recorded.
    pub closed_at: String,
    /// The tickets the close covers.
    pub members: Vec<TicketSlug>,
}

/// One member of a wave in `ttm wave history`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WaveMember {
    /// The ticket's slug.
    pub slug: TicketSlug,
    /// The ticket's legacy number, when it was imported with one.
    #[serde(default)]
    pub legacy_id: Option<LegacyTicketNumber>,
    /// The ticket's status now; none when the ticket no longer exists.
    #[serde(default)]
    pub status_now: Option<String>,
    /// The ticket's status at the `--as-of` instant, from its event history; none when no
    /// instant was asked for or the history does not reach back that far.
    #[serde(default)]
    pub status_as_of: Option<String>,
    /// The commit the ticket landed in.
    #[serde(default)]
    pub landing_sha: Option<String>,
}

/// `ttm wave history <n>`: one wave's membership and its members' statuses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WaveHistoryDocument {
    /// The project.
    #[serde(default)]
    pub project: String,
    /// The wave's label.
    pub n: u32,
    /// `open`, `emptied`, `closed` or `unknown`.
    pub state: String,
    /// When the wave was closed.
    #[serde(default)]
    pub closed_at: Option<String>,
    /// The marker commit the close recorded.
    #[serde(default)]
    pub closed_sha: Option<String>,
    /// The wave's members.
    #[serde(default)]
    pub members: Vec<WaveMember>,
}

/// One ticket of the collision analysis.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CollisionRow {
    /// The ticket's key.
    pub key: String,
    /// The ticket's title.
    #[serde(default)]
    pub title: String,
    /// The ticket's wave label.
    #[serde(default)]
    pub wave: Option<u32>,
    /// The repository paths the ticket owns.
    #[serde(default)]
    pub owns: Vec<String>,
}

/// `ttm wave collisions`: the next disjoint dispatch set.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CollisionsDocument {
    /// How many tickets may run at once.
    pub cap: usize,
    /// The wave the picked tickets come from.
    #[serde(default)]
    pub next_wave: Option<u32>,
    /// The tickets running now.
    #[serde(default)]
    pub running: Vec<CollisionRow>,
    /// The tickets picked to run next, disjoint from the running ones.
    #[serde(default)]
    pub picked: Vec<CollisionRow>,
}
