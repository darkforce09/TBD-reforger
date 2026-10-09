//! The ticket scope, the closed field value sets, the word caps and the shared predicates.
//!
//! **Role:** [`Domain`] and [`ScopeV2`] (the `[scope]` table), the `class` and `estimated` value
//! sets, the word caps, the two debt pins and the predicates that judge SHA shape, title debt,
//! `main_goal` debt, missing ready-tier fields and a work ticket's class.
//! **Position:** inside `ticket_model::model`; the encoding validates values against these sets,
//! and `ticket_registry`'s checks and operations, `ticket_metrics` and the ticketboard call the
//! predicates through the crate root, so each rule has one definition.
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** `Domain` is the only compiled scope level; every word cap counts with
//! `split_whitespace().count()` over the TOML-parsed string; both debt pins are 0.

use super::*;

/// The top level of a work ticket's scope, the one level that is a closed Rust enum. The levels
/// below it (`layer`, `component`, `surface`) are words from `.ai/tickets/scope-vocab.toml`,
/// resolved by [`crate::Corpus::load`] and `ticket check`, so adding one needs no code change.
/// On disk the domain is its snake_case name (`domain = "website"`); `frontend` is a layer,
/// never a domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    /// `website`: the web platform: the API, the single-page app and what they share.
    Website,
    /// `mod`: the Enfusion mod, its assets, worlds and Workbench tooling.
    Mod,
    /// `schema`: the boundary contracts, including the mission and registry schemas.
    Schema,
    /// `engine`: the map and graphics engines.
    Engine,
    /// `repo`: repository-wide work: CI, documentation, tooling and the ticket registry.
    Repo,
}

impl Domain {
    /// The on-disk spelling, the same word the vocabulary's top-level tables use.
    pub fn as_str(self) -> &'static str {
        match self {
            Domain::Website => "website",
            Domain::Mod => "mod",
            Domain::Schema => "schema",
            Domain::Engine => "engine",
            Domain::Repo => "repo",
        }
    }
}

/// A work ticket's place in the vocabulary: exactly one `domain` and `layer`, an optional
/// `component` (the vocabulary has component-free layers), and a `surface` list (one slice may
/// touch several surfaces of its component; a ticket spanning components is mis-sliced).
/// Serialised as the flat `[scope]` table, which refuses unknown keys:
///
/// ```toml
/// [scope]
/// domain = "website"
/// layer = "frontend"
/// component = "mission_creator"   # omitted when None
/// surface = ["attr_panel"]        # omitted when empty
/// ```
///
/// [`crate::TicketFile::into_ticket`] holds the shape rule (a surface requires a component,
/// because the vocabulary has no layer-level surfaces); legality against
/// `.ai/tickets/scope-vocab.toml` is the vocabulary's question ([`crate::ScopeVocab`]), not the
/// parser's (see [`crate::parse_ticket_toml`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeV2 {
    /// `domain`: the closed top level.
    pub domain: Domain,
    /// `layer`: a layer table under the domain in the vocabulary (`frontend`, `docs`, ...).
    pub layer: String,
    /// `component`: a component key under the layer; `None` (key omitted) for a
    /// component-free scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    /// `surface`: entries of the component's surface array; empty (key omitted) when the work
    /// is component-wide. Non-empty only with a `component`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surface: Vec<String>,
}

/// The closed `class` value set. `ticket check` requires a class on every work ticket; the parse
/// refuses any other value wherever the key is present.
pub const CLASS_VALUES: &[&str] = &["bug", "feature", "chore", "audit", "docs"];

/// The fields an `estimated` list may name: the stamps and facts whose value is an estimate
/// rather than a record. `scope` marks a scope inferred from `owns` rather than written by an
/// author.
pub const ESTIMATED_VALUES: &[&str] = &[
    "created_at",
    "completed_at",
    "shipped_at",
    "tokens",
    "scope",
];

/// Whether `v` is 7 to 40 lowercase hex characters, the shape of `shipped_at` and of an estimate
/// commit. The one SHA-shape rule: the estimates check, the ship gate and `ticket_registry`'s
/// `ops::stamp_sha` all judge through it.
pub fn is_sha_shaped(v: &str) -> bool {
    (7..=40).contains(&v.len())
        && v.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// The word cap on a work ticket's `summary`. Like every word cap here, `ticket check` and the
/// operations' post-image gate enforce it and the parse never does, so every historical
/// revision stays readable; every enforcer counts `split_whitespace().count()` over the
/// TOML-parsed string.
///
/// A non-empty `migration_legacy` exempts exactly this cap (a quarantined ticket's summary is its
/// title, which is never truncated). Program summaries are uncapped.
pub const SUMMARY_WORD_CAP: usize = 40;

/// The word cap on each line of `context`, `requirement`, `current_state`, `approach` and
/// `verify`. `acceptance`, `notes` and `main_goal` are uncapped, by field and never by ticket
/// class.
pub const BODY_LINE_WORD_CAP: usize = 30;

/// The word cap on each `citations` entry, which holds a reference and nothing else.
pub const CITATION_WORD_CAP: usize = 8;

/// The word cap on a title. A real title is non-empty, is not the ticket id and stays within
/// this many words; the operations' post-image gate enforces it on every ticket an operation
/// changes, and [`TITLE_DEBT_PIN`] meters the whole corpus.
pub const TITLE_WORD_CAP: usize = 10;

/// Whether a title is debt: it equals the id, or its TOML-parsed text holds more than
/// [`TITLE_WORD_CAP`] words by `split_whitespace().count()`. Both ticket kinds count. The
/// check-side counter, the operations' post-image gate and the store's ratchet test share it.
pub fn title_is_debt(id: &crate::TicketId, title: &str) -> bool {
    id.as_str() == title || title.split_whitespace().count() > TITLE_WORD_CAP
}

/// The number of tickets, of either kind, whose title is debt ([`title_is_debt`]). A count above
/// the pin is red in `ticket check` and means a title bypassed the gate; a repair lowers the pin
/// in the same commit. The pin only shrinks.
pub const TITLE_DEBT_PIN: usize = 0;

/// The number of live work tickets without a `main_goal` ([`main_goal_is_debt`]), held by
/// `ticket check`. The operations' post-image gate refuses a
/// changed, non-quarantined live work ticket without one, so the count cannot grow. Quarantined
/// tickets (non-empty `migration_legacy`) count: decomposing the parked text fills `main_goal`
/// and lowers the pin in the same commit.
pub const MAIN_GOAL_DEBT_PIN: usize = 0;

/// Whether a work ticket is `main_goal` debt: its status is live (queued, ready, running or
/// review) and its `main_goal` is absent or blank. The check-side counter and the store's
/// ratchet test share it.
pub fn main_goal_is_debt(w: &WorkTicket) -> bool {
    w.status.name().is_live() && w.main_goal.as_deref().unwrap_or("").trim().is_empty()
}

/// The body fields a ready, running, review or shipped work ticket lacks, by name, in the order
/// `context`, `requirement`, `current_state`, `approach`, `verify`, `acceptance`; a field whose
/// lines are all blank counts as missing. `ticket_registry`'s `mark-ready` and `ship` refuse
/// naming each, and `ticket check` reports the same list.
///
/// `main_goal` and `spec` are absent because a ready-class status cannot be built without them
/// ([`Status::live_ready`]); `ship` checks `main_goal` itself. `acceptance` is present because
/// `ship` from `queued` never passed that constructor. Callers apply the quarantine exemption (a
/// non-empty `migration_legacy`) themselves.
pub fn empty_ready_tier_fields(w: &WorkTicket) -> Vec<&'static str> {
    let mut missing = Vec::new();
    for (name, lines) in [
        ("context", &w.context),
        ("requirement", &w.requirement),
        ("current_state", &w.current_state),
        ("approach", &w.approach),
        ("verify", &w.verify),
        ("acceptance", &w.acceptance),
    ] {
        if lines.iter().all(|s| s.trim().is_empty()) {
            missing.push(name);
        }
    }
    missing
}

/// A class for a work ticket from its title and summary words: the same text always gives the
/// same class, which is triage metadata rather than an estimate (no `estimated` entry names it).
/// It matches whole alphanumeric words, so "prefix" and "fixture" are not bugs. Precedence:
/// `bug`, `audit`, `docs`, `chore`, then `feature`.
pub fn classify_work(text: &str) -> &'static str {
    let lower = text.to_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    let has = |names: &[&str]| tokens.iter().any(|t| names.contains(t));
    if has(&[
        "fix",
        "fixes",
        "fixed",
        "bug",
        "bugs",
        "regression",
        "regressions",
    ]) {
        "bug"
    } else if has(&["audit", "audits"]) {
        "audit"
    } else if has(&["docs", "doc", "readme", "documentation"]) {
        "docs"
    } else if has(&[
        "refactor",
        "refactors",
        "cleanup",
        "delete",
        "deletes",
        "port",
        "ports",
        "rename",
        "renames",
        "migrate",
        "migrates",
        "migration",
        "gate",
        "gates",
        "ci",
    ]) {
        "chore"
    } else {
        "feature"
    }
}
