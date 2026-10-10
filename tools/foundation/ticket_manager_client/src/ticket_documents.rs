//! The ticket-level JSON documents `ttm --json` prints.
//!
//! **Role:** typed forms of the `ttm.version/1`, `ttm.ticket/1`, `ttm.resolve/1`, `ttm.list/1`,
//! `ttm.next/1`, `ttm.check/1`, `ttm.receipt/1`, `ttm.land/1`, `ttm.unland/1` and
//! `ttm.metrics/1` documents, each with the format tag [`crate::parse_document`] checks.
//! **Position:** produced by [`crate::TicketManager`]'s calls; the slice runner, the wave drivers
//! and the preflight read them.
//! **Signals & state:** none; plain data.
//! **Invariants:** each type declares only the fields its callers read, and unknown fields are
//! ignored, because the contract adds fields freely and never renames one without a new format
//! version; an optional field the contract may omit defaults to empty rather than failing.

use serde::Deserialize;

use crate::ticket_references::{LegacyTicketNumber, ReceiptName, TicketSlug};

/// A JSON document of the ticket manager's contract, recognised by its `format` tag.
pub trait TicketManagerDocument: serde::de::DeserializeOwned {
    /// The `format` tag the document carries, such as `ttm.ticket/1`.
    const FORMAT: &'static str;
}

/// Declares the format tag of each document type.
macro_rules! document_formats {
    ($($document:ty => $format:literal),* $(,)?) => {
        $(impl TicketManagerDocument for $document {
            const FORMAT: &'static str = $format;
        })*
    };
}
pub(crate) use document_formats;

document_formats! {
    VersionDocument => "ttm.version/1",
    TicketDocument => "ttm.ticket/1",
    ResolvedReference => "ttm.resolve/1",
    TicketListDocument => "ttm.list/1",
    NextTicketsDocument => "ttm.next/1",
    CheckDocument => "ttm.check/1",
    ReceiptDocument => "ttm.receipt/1",
    LandDocument => "ttm.land/1",
    UnlandDocument => "ttm.unland/1",
    MetricsDocument => "ttm.metrics/1",
}

/// `ttm version`: the binary's version and the versions of its contracts.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VersionDocument {
    /// The binary's own version.
    pub version: String,
    /// The JSON contract version (the `/1` of every format tag).
    pub json_version: u32,
    /// The schema version of the ticket database.
    pub db_schema: i64,
    /// The ticket database the binary opened.
    pub db_path: String,
}

/// A child of a ticket, as `ttm show` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ChildTicket {
    /// The child's slug.
    pub slug: TicketSlug,
    /// The child's legacy number, when it was imported with one.
    #[serde(default)]
    pub legacy_id: Option<LegacyTicketNumber>,
    /// The child's title.
    pub title: String,
    /// The child's status (`queued`, `ready`, `shipped`, `cancelled`, …).
    pub status: String,
}

/// A commit linked to a ticket.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LinkedCommit {
    /// The commit's sha.
    pub sha: String,
    /// Why it is linked: `landing`, `subject`, `estimate`, `cited` or `receipt`.
    pub role: String,
    /// The commit's subject, when recorded.
    #[serde(default)]
    pub subject: Option<String>,
}

/// `ttm show <ref>`: one ticket.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TicketDocument {
    /// The project the ticket belongs to.
    pub project: String,
    /// The ticket's slug.
    pub slug: TicketSlug,
    /// The ticket's legacy number, when it was imported with one.
    #[serde(default)]
    pub legacy_id: Option<LegacyTicketNumber>,
    /// `work` or `program`.
    pub kind: String,
    /// The ticket's title.
    pub title: String,
    /// The ticket's status.
    pub status: String,
    /// Who may take the ticket: `claude-code`, `documentation`, `workbench`, `human` or `ci`.
    pub executor: String,
    /// The parent's slug, for a child slice.
    #[serde(default)]
    pub parent: Option<TicketSlug>,
    /// Every child, in order.
    #[serde(default)]
    pub children: Vec<ChildTicket>,
    /// The slice of a programme that runs next, when the ticket names one.
    #[serde(default)]
    pub active_slice: Option<TicketSlug>,
    /// The repository paths the ticket owns.
    #[serde(default)]
    pub owns: Vec<String>,
    /// Whether the ticket has a specification.
    pub has_spec: bool,
    /// Whether the ticket has a plan.
    pub has_plan: bool,
    /// How many run receipts the ticket has.
    pub receipt_count: usize,
    /// The outcome of the newest receipt (`ran`, `landed` or `failed`).
    #[serde(default)]
    pub latest_receipt_outcome: Option<String>,
    /// The commit the ticket landed in.
    #[serde(default)]
    pub landing_sha: Option<String>,
    /// Every commit linked to the ticket.
    #[serde(default)]
    pub commits: Vec<LinkedCommit>,
    /// The wave label the ticket sits in (`0` = parked), when the project has a wave lock.
    #[serde(default)]
    pub wave: Option<u32>,
}

impl TicketDocument {
    /// The ticket's legacy number when it has one, else its slug: the spelling the factory's
    /// branches and worktrees used before the ticket manager.
    pub fn display_reference(&self) -> &str {
        self.legacy_id
            .as_ref()
            .map(LegacyTicketNumber::as_str)
            .unwrap_or(self.slug.as_str())
    }
}

/// `ttm resolve <ref>`: the ticket a reference names.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ResolvedReference {
    /// The project the ticket belongs to.
    pub project: String,
    /// The ticket's slug.
    pub slug: TicketSlug,
    /// The ticket's legacy number, when it was imported with one.
    #[serde(default)]
    pub legacy_id: Option<LegacyTicketNumber>,
}

/// One ticket row of `ttm list` and `ttm next`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TicketSummary {
    /// The ticket's slug.
    pub slug: TicketSlug,
    /// The ticket's legacy number, when it was imported with one.
    #[serde(default)]
    pub legacy_id: Option<LegacyTicketNumber>,
    /// The ticket's title.
    pub title: String,
    /// The ticket's status.
    pub status: String,
    /// Who may take the ticket.
    pub executor: String,
    /// The parent's slug, for a child slice.
    #[serde(default)]
    pub parent: Option<TicketSlug>,
    /// The commit the ticket landed in.
    #[serde(default)]
    pub landing_sha: Option<String>,
}

/// `ttm list`: the tickets of the project.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TicketListDocument {
    /// The listed tickets.
    pub tickets: Vec<TicketSummary>,
}

/// `ttm next`: the active tickets and the ones to take next.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NextTicketsDocument {
    /// The project.
    pub project: String,
    /// The tickets in progress.
    pub active: Vec<TicketSummary>,
    /// The tickets to take next, in order.
    pub next: Vec<TicketSummary>,
}

/// One finding of `ttm check`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CheckFinding {
    /// The ticket the finding is about, when it is about one.
    #[serde(default)]
    pub slug: Option<String>,
    /// The rule that found it.
    pub rule: String,
    /// What is wrong.
    pub message: String,
    /// `error` or `warning`.
    pub severity: String,
}

/// `ttm check`: the project's ticket validation.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CheckDocument {
    /// The project.
    pub project: String,
    /// Whether the check passed.
    pub ok: bool,
    /// How many errors it found.
    pub errors: usize,
    /// How many warnings it found.
    pub warnings: usize,
    /// Every finding.
    #[serde(default)]
    pub findings: Vec<CheckFinding>,
}

/// `ttm record-run`: the receipt a run was recorded as.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ReceiptDocument {
    /// The receipt's name, `run-<row>`.
    pub receipt_id: ReceiptName,
    /// The ticket the run was recorded against.
    pub slug: TicketSlug,
}

/// `ttm land`: the landing commit recorded on a ticket.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LandDocument {
    /// The ticket.
    pub slug: TicketSlug,
    /// The ticket's legacy number, when it was imported with one.
    #[serde(default)]
    pub legacy_id: Option<LegacyTicketNumber>,
    /// The landing commit now recorded.
    pub landing_sha: String,
    /// The database row of the receipt stamped `landed`, when the ticket had one.
    #[serde(default)]
    pub stamped_receipt: Option<i64>,
}

/// `ttm unland`: the ticket whose landing commit was cleared.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UnlandDocument {
    /// The ticket.
    pub slug: TicketSlug,
}

/// The token totals of one agent in `ttm metrics`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AgentTokenTotals {
    /// The agent.
    pub agent: String,
    /// How many runs it recorded.
    pub runs: usize,
    /// The sum of its tokens.
    pub total: u64,
}

/// `ttm metrics`: run receipts and token totals.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MetricsDocument {
    /// The project.
    pub project: String,
    /// The ticket the totals are about, or none for the whole project.
    #[serde(default)]
    pub slug: Option<TicketSlug>,
    /// How many runs are recorded.
    pub runs: usize,
    /// The sum of every run's tokens.
    pub total_tokens: u64,
    /// The totals per agent.
    #[serde(default)]
    pub by_agent: Vec<AgentTokenTotals>,
}
