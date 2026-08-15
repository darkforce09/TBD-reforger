//! Typed ticket model. Inner `deny(clippy::wildcard_enum_match_arm)` is the sole
//! exhaustive-match authority (T-911.2).
#![deny(clippy::wildcard_enum_match_arm)]

use serde::{Deserialize, Serialize};

mod encoding;
pub mod ops;
#[cfg(test)]
mod proptest_roundtrip;
pub mod store;
mod timestamp;
pub mod vocab;
pub use encoding::{TicketFile, parse_ticket_toml, render_ticket_toml};
pub use ops::OpOutcome;
pub use store::Corpus;
pub use timestamp::{now_utc_rfc3339, validate_rfc3339_utc};
pub use vocab::ScopeVocab;

/// Scope v2 domain — the ONE level that stays a closed Rust enum (T-917 spec §Scope v2:
/// "changes ~never"). Everything below it (`layer`/`component`/`surface`) is validated
/// data from `.ai/tickets/scope-vocab.toml`, resolved at [`Corpus::load`] and in
/// `ticket check` — never compiled (compiled-enum friction is what produced the
/// 199-ticket docs landfill). The crate-level `deny(clippy::wildcard_enum_match_arm)`
/// keeps every `Domain` match exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Website,
    Mod,
    Schema,
    Engine,
    Repo,
}

impl Domain {
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

/// Scope v2 (T-917.2): the flat 4-level breadcrumb — exactly one `domain`/`layer`,
/// optional `component` (component-free layers exist per the vocabulary), and a
/// `surface` array (a coherent slice may touch several surfaces; a ticket spanning
/// components is mis-sliced). Serialized as the flat `[scope]` table:
///
/// ```toml
/// [scope]
/// domain = "website"
/// layer = "frontend"
/// component = "mission_creator"   # omitted when None
/// surface = ["attr_panel"]        # omitted when empty
/// ```
///
/// Shape rules live in `TicketFile::into_ticket` (surface requires component — the
/// vocabulary tree has no layer-level surfaces to name); per-value LEGALITY against
/// `.ai/tickets/scope-vocab.toml` is deliberately NOT here — see the documented
/// weakening on [`parse_ticket_toml`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeV2 {
    pub domain: Domain,
    pub layer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surface: Vec<String>,
}

/// The closed `class` value set (T-917 Decisions log #4) — required on work tickets
/// (check-enforced; value-validated at parse when present).
pub const CLASS_VALUES: &[&str] = &["bug", "feature", "chore", "audit", "docs"];

/// Legal `estimated[]` entries — the provenance machinery's field list (T-917 spec
/// §Provenance; `scope` is the non-numeric reuse recorded by the v2 migrator).
pub const ESTIMATED_VALUES: &[&str] = &[
    "created_at",
    "completed_at",
    "shipped_at",
    "tokens",
    "scope",
];

/// 7–40 lowercase hex — the repo's `shipped_at` / estimate SHA shape. THE single
/// authority (T-917.6): the T-917.4 miner, the T-917.5 estimates check, the S.6 ship
/// gate and [`ops::stamp_sha`] all judge SHA-shapedness through this one predicate
/// (xtask re-exports it), so the shape rule cannot fork.
pub fn is_sha_shaped(v: &str) -> bool {
    (7..=40).contains(&v.len())
        && v.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// T-917.3 word caps (spec §Body, Decisions log #6) — CHECK-enforced (`ticket check`)
/// plus an `ops::validate_post_image` refusal on tickets an op rewrites; NEVER
/// parse-enforced, so old git revisions stay readable. The counting instrument is
/// `split_whitespace().count()` over the TOML-PARSED string everywhere (quarantine
/// verb, check rules, ops gate, ratchet pin) — one instrument, no raw-regex-vs-parse
/// method disagreement (the spec's measured-facts table documents that trap).
///
/// `summary` cap on WORK tickets; a nonempty `migration_legacy` exempts exactly the
/// summary cap (quarantined tickets got `summary := title`, which may itself exceed
/// the cap — truncation is forbidden). Program summaries are uncapped this pass
/// (work-only per spec §Wall quarantine; over-cap program summaries are reported by
/// the verb as a future note, never moved).
pub const SUMMARY_WORD_CAP: usize = 40;
/// Per-line cap on `context[]`/`requirement[]`/`current_state[]`/`approach[]`/
/// `verify[]` (spec §Body). `acceptance`/`notes`/`main_goal` are uncapped —
/// grandfathering by *field choice*, never by ticket class.
pub const BODY_LINE_WORD_CAP: usize = 30;
/// Per-entry cap on `citations[]` (reference-only strings).
pub const CITATION_WORD_CAP: usize = 8;

/// T-920.1 title gate (t920 spec Decisions log #4): a REAL title is nonempty, is not
/// the ticket id, and stays within this many words. Enforced on CHANGED tickets by the
/// ops post-image gate; history debt is metered by [`TITLE_DEBT_PIN`] and drained by
/// the T-919/T-921 streams — never check-redded wholesale.
pub const TITLE_WORD_CAP: usize = 10;

/// The one title-debt instrument (t920 spec §Schema changes): `title == id` OR the
/// TOML-parsed title exceeds [`TITLE_WORD_CAP`] by `split_whitespace().count()` —
/// the same counting instrument as every other word cap in this crate. Both ticket
/// kinds count. Shared by the check-side counter, the ops post-image gate and the
/// store ratchet test, so the instrument cannot fork.
pub fn title_is_debt(id: &str, title: &str) -> bool {
    title == id || title.split_whitespace().count() > TITLE_WORD_CAP
}

/// T-920.1 shrink-only debt pin: work+program tickets where [`title_is_debt`]
/// (measured on the live tree at land time — 99 id-as-title + 341 over-cap, zero
/// overlap possible: an id is one token). Drift is red BOTH ways in `ticket check`
/// and in the store ratchet test: growth means a title gate bypass (ops refuse debt
/// titles on changed tickets), shrinkage means a repair landed and the pin must
/// shrink in the same commit (the T-919/T-921 batch contract).
pub const TITLE_DEBT_PIN: usize = 333;

/// T-920.1 shrink-only debt pin: queued/ready/running/review WORK tickets with empty
/// `main_goal` (instrument: [`main_goal_is_debt`]) — measured on the live tree at
/// land time. The queued-tier main_goal obligation (t920 spec Decisions log #1) binds
/// as this metered ratchet instead of an instant corpus-wide red because the debt is
/// history-wide; NEW offenders are impossible: the ops post-image gate refuses a
/// changed non-quarantined queued+ work ticket without main_goal. Quarantined
/// carriers (nonempty `migration_legacy`) ARE counted — the wall holds the content
/// unprocessed, and the T-919 drain fills main_goal when it decomposes the wall,
/// shrinking this pin in the same commit.
pub const MAIN_GOAL_DEBT_PIN: usize = 52;

/// The one main_goal-debt instrument (t920 spec §Schema changes): a live
/// (queued/ready/running/review) work ticket whose `main_goal` is empty/absent.
/// Shared by the check-side counter and the store ratchet test.
pub fn main_goal_is_debt(w: &WorkTicket) -> bool {
    w.status.name().is_live() && w.main_goal.as_deref().unwrap_or("").trim().is_empty()
}

/// T-920.1 ready-tier body obligation (t920 spec Decisions log #2): the six fields a
/// ready/running/review (and future-shipped) WORK ticket must carry nonempty, in the
/// spec table's order. Returns the empty ones by name — `ops::mark_ready` and
/// `ops::ship` refuse naming each, and the corpus-wide check rule reds the same list.
/// `main_goal` and `spec` are NOT here: ready-class already parse-enforces them
/// ([`Status::live_ready`]), so listing them would double-report (`ship` checks
/// main_goal separately — a queued→shipped jump never passes the ready-class parse).
/// `acceptance` IS here although ready-class parse-enforces it too: `ship` from
/// queued is the reachable case; on ready-class tickets the entry is belt-and-braces
/// that cannot fire (an empty-acceptance ready ticket refuses the corpus load).
/// Callers apply the quarantine exemption (nonempty `migration_legacy`) themselves —
/// content exists, unprocessed (the T-919 drain fills the fields with the wall).
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

/// Conservative-deterministic class triage from title/summary prose (same input →
/// same class; metadata triage, not provenance — T-917.2 migrator header documents
/// why this carries no `estimated[]` marker). Token-boundary matching on purpose:
/// substring matching would classify "prefix"/"fixture" as bugs. Precedence:
/// bug > audit > docs > chore > feature.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusName {
    Idea,
    Queued,
    Ready,
    Running,
    Review,
    Shipped,
    Deferred,
    Cancelled,
}

impl StatusName {
    pub fn as_str(self) -> &'static str {
        match self {
            StatusName::Idea => "idea",
            StatusName::Queued => "queued",
            StatusName::Ready => "ready",
            StatusName::Running => "running",
            StatusName::Review => "review",
            StatusName::Shipped => "shipped",
            StatusName::Deferred => "deferred",
            StatusName::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "idea" => StatusName::Idea,
            "queued" => StatusName::Queued,
            "ready" => StatusName::Ready,
            "running" => StatusName::Running,
            "review" => StatusName::Review,
            "shipped" => StatusName::Shipped,
            "deferred" => StatusName::Deferred,
            "cancelled" => StatusName::Cancelled,
            _ => return None,
        })
    }

    pub fn is_live(self) -> bool {
        matches!(
            self,
            StatusName::Queued | StatusName::Ready | StatusName::Running | StatusName::Review
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Idea,
    Queued {
        order: i64,
    },
    Ready {
        order: i64,
        spec: String,
        main_goal: String,
        acceptance: Vec<String>,
    },
    Running {
        order: i64,
        spec: String,
        main_goal: String,
        acceptance: Vec<String>,
    },
    Review {
        order: i64,
        spec: String,
        main_goal: String,
        acceptance: Vec<String>,
    },
    Shipped {
        shipped_at: Option<String>,
        order: Option<i64>,
    },
    Deferred {
        order: Option<i64>,
    },
    Cancelled {
        order: Option<i64>,
    },
}

impl Status {
    pub fn name(&self) -> StatusName {
        match self {
            Status::Idea => StatusName::Idea,
            Status::Queued { .. } => StatusName::Queued,
            Status::Ready { .. } => StatusName::Ready,
            Status::Running { .. } => StatusName::Running,
            Status::Review { .. } => StatusName::Review,
            Status::Shipped { .. } => StatusName::Shipped,
            Status::Deferred { .. } => StatusName::Deferred,
            Status::Cancelled { .. } => StatusName::Cancelled,
        }
    }

    pub fn order(&self) -> Option<i64> {
        match self {
            Status::Idea => None,
            Status::Queued { order } => Some(*order),
            Status::Ready { order, .. }
            | Status::Running { order, .. }
            | Status::Review { order, .. } => Some(*order),
            Status::Shipped { order, .. }
            | Status::Deferred { order, .. }
            | Status::Cancelled { order, .. } => *order,
        }
    }

    /// Ready/running/review require spec + main_goal + nonempty acceptance.
    /// (T-920.1: `main_goal` is the renamed `user_story` — same slot, same rule.)
    pub fn live_ready(
        name: StatusName,
        order: i64,
        spec: String,
        main_goal: String,
        acceptance: Vec<String>,
    ) -> Result<Self, String> {
        if spec.trim().is_empty() {
            return Err("spec required".into());
        }
        if main_goal.trim().is_empty() {
            return Err("main_goal required".into());
        }
        if acceptance.iter().all(|s| s.trim().is_empty()) {
            return Err("acceptance required".into());
        }
        Ok(match name {
            StatusName::Ready => Status::Ready {
                order,
                spec,
                main_goal,
                acceptance,
            },
            StatusName::Running => Status::Running {
                order,
                spec,
                main_goal,
                acceptance,
            },
            StatusName::Review => Status::Review {
                order,
                spec,
                main_goal,
                acceptance,
            },
            StatusName::Idea
            | StatusName::Queued
            | StatusName::Shipped
            | StatusName::Deferred
            | StatusName::Cancelled => return Err("not a ready-class status".into()),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramTicket {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// T-917.2: legal on programs (the KEY, value-validated); REQUIRED on work only.
    pub class: Option<String>,
    pub status: Status,
    pub executor: Option<String>,
    pub notes: Option<String>,
    pub spec: Option<String>,
    /// T-917.2: per-ticket plan document path (distinct from the shared program
    /// `spec`); becomes a ready-gate at S.6.
    pub plan: Option<String>,
    pub depends_on: Vec<String>,
    pub unblocks: Vec<String>,
    pub children: Vec<String>,
    pub active: Option<String>,
    /// T-920.1: renamed from `user_story` (t920 spec Decisions log #1 — the content
    /// was goal-shaped, not persona prose). Same canonical slot; old revisions parse
    /// via the serde alias on `TicketFile`.
    pub main_goal: Option<String>,
    /// T-917.2 body decomposition (spec §Body): typed line lists, caps check-enforced
    /// later (S.3) — never parse-enforced, so old git revisions stay readable.
    pub context: Vec<String>,
    pub requirement: Vec<String>,
    pub current_state: Vec<String>,
    pub approach: Vec<String>,
    pub verify: Vec<String>,
    pub acceptance: Vec<String>,
    pub citations: Vec<String>,
    pub priority: Option<i64>,
    /// T-913.1 lifecycle stamps — RFC 3339 UTC strings validated on parse (see
    /// [`validate_rfc3339_utc`]); malformed values refuse the load, never become now.
    /// `ProgramTicket` carries no `shipped_at` field (that lives inside
    /// [`Status::Shipped`]), so the canonical slot is immediately before `owns` — the
    /// same position all three ticket types use.
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
    /// T-917.2 provenance: which stamps/facts are estimates, values from
    /// [`ESTIMATED_VALUES`]; `estimate_note` names the gap when method 2 could not
    /// mine a value.
    pub estimated: Vec<String>,
    pub estimate_note: Option<String>,
    /// T-917.3 wall quarantine target — byte-reversible parked prose. Minting this
    /// field on a NEW ticket goes red at check level (shrink-only ratchet).
    pub migration_legacy: Vec<String>,
    pub owns: Vec<String>,
    pub pack_last: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkTicket {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// T-917.2: bug | feature | chore | audit | docs — REQUIRED on work tickets
    /// (check-enforced; the parse validates the value whenever present).
    pub class: Option<String>,
    pub status: Status,
    pub executor: Option<String>,
    pub notes: Option<String>,
    pub spec: Option<String>,
    /// T-917.2: per-ticket plan document path (S.6 ready-gate).
    pub plan: Option<String>,
    pub depends_on: Vec<String>,
    pub unblocks: Vec<String>,
    pub parent: Option<String>,
    pub scope: ScopeV2,
    /// T-920.1: renamed from `user_story` — see [`ProgramTicket::main_goal`].
    pub main_goal: Option<String>,
    /// T-917.2 body decomposition (spec §Body) — see [`ProgramTicket`] notes.
    pub context: Vec<String>,
    pub requirement: Vec<String>,
    pub current_state: Vec<String>,
    pub approach: Vec<String>,
    pub verify: Vec<String>,
    pub acceptance: Vec<String>,
    pub citations: Vec<String>,
    pub shipped_at: Option<String>,
    pub priority: Option<i64>,
    /// T-913.1 lifecycle stamps — after `shipped_at` (which stays a bare commit SHA),
    /// immediately before `owns`; RFC 3339 UTC, validated on parse, never backfilled.
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
    /// T-917.2 provenance — see [`ProgramTicket::estimated`]. `"scope"` here is the
    /// migrator's honest escape: this ticket's scope was owns-inferred, not carried
    /// by v1 data.
    pub estimated: Vec<String>,
    pub estimate_note: Option<String>,
    /// T-917.3 wall quarantine target — see [`ProgramTicket::migration_legacy`].
    pub migration_legacy: Vec<String>,
    pub owns: Vec<String>,
    pub pack_last: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ticket {
    Program(ProgramTicket),
    Work(WorkTicket),
}

impl Ticket {
    pub fn id(&self) -> &str {
        match self {
            Ticket::Program(p) => &p.id,
            Ticket::Work(w) => &w.id,
        }
    }

    pub fn status(&self) -> &Status {
        match self {
            Ticket::Program(p) => &p.status,
            Ticket::Work(w) => &w.status,
        }
    }
}

pub const FROZEN_UNMAPPABLE: &[&str] = &[
    "T-067", "T-071", "T-110", "T-111", "T-113", "T-130", "T-134", "T-144", "T-145", "T-146",
    "T-147", "T-148", "T-149", "T-151", "T-160", "T-161", "T-162", "T-163", "T-164", "T-165",
    "T-183", "T-241", "T-242", "T-251", "T-252", "T-253", "T-259", "T-275", "T-280", "T-290",
    "T-291", "T-311", "T-415", "T-419", "T-439", "T-460", "T-462", "T-541", "T-543", "T-545",
    "T-604", "T-605", "T-606", "T-607", "T-608", "T-609", "T-612", "T-617", "T-619",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_unmappable_is_49() {
        assert_eq!(FROZEN_UNMAPPABLE.len(), 49);
        let mut s: Vec<_> = FROZEN_UNMAPPABLE.to_vec();
        s.sort();
        s.dedup();
        assert_eq!(s.len(), 49);
    }

    #[test]
    fn ready_constructor_rejects_empty_goal() {
        let err = Status::live_ready(
            StatusName::Ready,
            1,
            "spec.md".into(),
            "   ".into(),
            vec!["a".into()],
        )
        .unwrap_err();
        assert!(err.contains("main_goal"));
    }

    /// T-920.1 — the shared title-debt instrument: id-as-title and >10-word titles
    /// are debt; a real title within the cap is not; the id can never trip the
    /// word-count arm (one token), so the two debt classes cannot overlap.
    #[test]
    fn title_debt_instrument() {
        assert!(title_is_debt("T-915.4", "T-915.4"), "id-as-title is debt");
        assert!(
            title_is_debt(
                "T-1",
                "one two three four five six seven eight nine ten eleven"
            ),
            "11 words is debt"
        );
        assert!(
            !title_is_debt("T-1", "one two three four five six seven eight nine ten"),
            "exactly 10 words is legal"
        );
        assert!(!title_is_debt("T-1", "Fix the marker save regression"));
        // Emptiness is deliberately NOT this instrument's business: the corpus-wide
        // idea-tier check rule reds an empty title (measured zero offenders), and the
        // ops post-image gate refuses writing one — the pin meters only the two
        // measured history-debt classes.
        assert!(!title_is_debt("T-1", ""));
    }

    /// T-917.2: the class triage is token-boundary conservative — "prefix"/"fixture"
    /// must never classify as bug — and deterministic in its precedence order.
    #[test]
    fn classify_work_is_token_boundary_and_ordered() {
        assert_eq!(classify_work("Fix editor crash"), "bug");
        assert_eq!(classify_work("marker regression on save"), "bug");
        assert_eq!(classify_work("prefix and fixture handling"), "feature");
        assert_eq!(classify_work("Audit gate coverage"), "audit");
        assert_eq!(classify_work("README doc-only pass"), "docs");
        assert_eq!(classify_work("delete the Makefile"), "chore");
        assert_eq!(classify_work("port wave.sh to xtask"), "chore");
        assert_eq!(classify_work("Marker style widening"), "feature");
        // Precedence: a fix that mentions docs is a bug, not docs.
        assert_eq!(classify_work("fix README typo"), "bug");
        for c in [
            classify_work("a"),
            classify_work("fix"),
            classify_work("audit"),
            classify_work("docs"),
            classify_work("ci"),
        ] {
            assert!(CLASS_VALUES.contains(&c));
        }
    }

    #[test]
    fn domain_as_str_is_snake_case() {
        for (d, s) in [
            (Domain::Website, "website"),
            (Domain::Mod, "mod"),
            (Domain::Schema, "schema"),
            (Domain::Engine, "engine"),
            (Domain::Repo, "repo"),
        ] {
            assert_eq!(d.as_str(), s);
        }
    }

    #[test]
    fn status_name_roundtrip() {
        for s in [
            "idea",
            "queued",
            "ready",
            "running",
            "review",
            "shipped",
            "deferred",
            "cancelled",
        ] {
            assert_eq!(StatusName::parse(s).unwrap().as_str(), s);
        }
        assert!(StatusName::parse("nope").is_none());
    }
}
