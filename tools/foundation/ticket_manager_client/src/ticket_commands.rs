//! The ticket-level calls of [`TicketManager`].
//!
//! **Role:** one method per `ttm` ticket command the tools use: `version`, `show`, `resolve`,
//! `list`, `next`, `brief`, `check`, `record-run`, `land`, `unland`, `ship`, `set-status` and
//! `metrics`; [`RunRecord`] and [`TokenCounts`] are what `record-run` records.
//! **Position:** the slice runner records receipts and reads briefs here; the wave drivers land
//! and read tickets; the preflight checks the project.
//! **Signals & state:** none held; each method spawns `ttm` once.
//! **Invariants:** a reference is passed to `ttm` as given (a slug, a legacy number or a commit
//! prefix), and its answer names the ticket by slug; token counts are recorded as the agent
//! reported them, never invented.

use crate::error::Result;
use crate::ticket_documents::{
    CheckDocument, LandDocument, MetricsDocument, NextTicketsDocument, ReceiptDocument,
    ResolvedReference, TicketDocument, TicketListDocument, UnlandDocument, VersionDocument,
};
use crate::ticket_manager::{FailingVerdict, TicketManager};

/// The token counts of one agent run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenCounts {
    /// Uncached prompt tokens.
    pub input: u64,
    /// Generated tokens.
    pub output: u64,
    /// Prompt tokens served from the provider's cache.
    pub cache_read: u64,
    /// Prompt tokens written into the provider's cache.
    pub cache_write: u64,
    /// Reasoning tokens, when the agent reports them.
    pub reasoning: Option<u64>,
}

/// One agent run, as `ttm record-run` records it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRecord {
    /// The agent that ran (the basename of the invoked program).
    pub agent: String,
    /// What the run consumed.
    pub tokens: TokenCounts,
    /// When the run started (RFC 3339 UTC); with the ticket and the agent, it keys the receipt.
    pub started: String,
    /// When the run finished (RFC 3339 UTC).
    pub finished: Option<String>,
    /// `ran`, `landed` or `failed`.
    pub outcome: Option<String>,
    /// The commit the run ended at.
    pub sha: Option<String>,
}

impl TicketManager {
    /// `ttm version`.
    pub fn version(&self) -> Result<VersionDocument> {
        self.document(&strings(&["version"]), FailingVerdict::Refused)
    }

    /// `ttm show <reference>`.
    pub fn show(&self, reference: &str) -> Result<TicketDocument> {
        self.document(&strings(&["show", reference]), FailingVerdict::Refused)
    }

    /// `ttm resolve <reference>`.
    pub fn resolve(&self, reference: &str) -> Result<ResolvedReference> {
        self.document(&strings(&["resolve", reference]), FailingVerdict::Refused)
    }

    /// `ttm list --status <status>`, or `ttm list --all` (every status) when `status` is `None`.
    pub fn list(&self, status: Option<&str>) -> Result<TicketListDocument> {
        let mut args = strings(&["list"]);
        match status {
            Some(status) => args.extend(strings(&["--status", status])),
            // `ttm list` hides shipped and cancelled tickets unless asked.
            None => args.push("--all".to_string()),
        }
        self.document(&args, FailingVerdict::Refused)
    }

    /// `ttm next -n <count>`.
    pub fn next(&self, count: usize) -> Result<NextTicketsDocument> {
        let count = count.to_string();
        self.document(&strings(&["next", "-n", &count]), FailingVerdict::Refused)
    }

    /// `ttm brief <reference>`: the agent execution brief, as text.
    pub fn brief(&self, reference: &str) -> Result<String> {
        self.text(&strings(&["brief", reference]))
    }

    /// `ttm check`: the project's ticket validation. A failing check is a document with `ok`
    /// false, not an error.
    pub fn check(&self) -> Result<CheckDocument> {
        self.document(&strings(&["check"]), FailingVerdict::CarriesDocument)
    }

    /// `ttm record-run <reference> …`: records one run receipt. Idempotent on the ticket, the
    /// agent and the start stamp.
    pub fn record_run(&self, reference: &str, run: &RunRecord) -> Result<ReceiptDocument> {
        let tokens = run.tokens;
        let mut args = strings(&["record-run", reference, "--agent", &run.agent]);
        for (flag, count) in [
            ("--input", tokens.input),
            ("--output", tokens.output),
            ("--cache-read", tokens.cache_read),
            ("--cache-write", tokens.cache_write),
        ] {
            args.extend([flag.to_string(), count.to_string()]);
        }
        if let Some(reasoning) = tokens.reasoning {
            args.extend(["--reasoning".to_string(), reasoning.to_string()]);
        }
        args.extend(strings(&["--started", &run.started]));
        for (flag, value) in [
            ("--finished", &run.finished),
            ("--outcome", &run.outcome),
            ("--sha", &run.sha),
        ] {
            if let Some(value) = value {
                args.extend([flag.to_string(), value.clone()]);
            }
        }
        self.document(&args, FailingVerdict::Refused)
    }

    /// `ttm land <reference> --sha <sha> [--require-receipt]`: records the landing commit and
    /// stamps the newest receipt `landed`; it does not ship the ticket.
    pub fn land(&self, reference: &str, sha: &str, require_receipt: bool) -> Result<LandDocument> {
        let mut args = strings(&["land", reference, "--sha", sha]);
        if require_receipt {
            args.push("--require-receipt".to_string());
        }
        self.document(&args, FailingVerdict::Refused)
    }

    /// `ttm unland <reference>`: clears the landing commit.
    pub fn unland(&self, reference: &str) -> Result<UnlandDocument> {
        self.document(&strings(&["unland", reference]), FailingVerdict::Refused)
    }

    /// `ttm ship <reference> [--sha <sha>]`.
    pub fn ship(&self, reference: &str, sha: Option<&str>) -> Result<TicketDocument> {
        let mut args = strings(&["ship", reference]);
        if let Some(sha) = sha {
            args.extend(strings(&["--sha", sha]));
        }
        self.document(&args, FailingVerdict::Refused)
    }

    /// `ttm set-status <reference> <status>`.
    pub fn set_status(&self, reference: &str, status: &str) -> Result<TicketDocument> {
        self.document(
            &strings(&["set-status", reference, status]),
            FailingVerdict::Refused,
        )
    }

    /// `ttm metrics [<reference>]`.
    pub fn metrics(&self, reference: Option<&str>) -> Result<MetricsDocument> {
        let mut args = strings(&["metrics"]);
        args.extend(reference.map(str::to_string));
        self.document(&args, FailingVerdict::Refused)
    }
}

/// Owned copies of `args`.
pub(crate) fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}
