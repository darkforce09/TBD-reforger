//! Scope.

use super::*;

/// Scope v2 domain — the ONE level that stays a closed Rust enum (T-917 spec §Scope v2:
/// "changes ~never"). Everything below it (`layer`/`component`/`surface`) is validated
/// data from `.ai/tickets/scope-vocab.toml`, resolved at [`crate::Corpus::load`] and in
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
/// weakening on [`crate::parse_ticket_toml`].
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
/// gate and [`crate::ops::stamp_sha`] all judge SHA-shapedness through this one predicate
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
pub const TITLE_DEBT_PIN: usize = 0;

/// T-920.1 shrink-only debt pin: queued/ready/running/review WORK tickets with empty
/// `main_goal` (instrument: [`main_goal_is_debt`]) — measured on the live tree at
/// land time. The queued-tier main_goal obligation (t920 spec Decisions log #1) binds
/// as this metered ratchet instead of an instant corpus-wide red because the debt is
/// history-wide; NEW offenders are impossible: the ops post-image gate refuses a
/// changed non-quarantined queued+ work ticket without main_goal. Quarantined
/// carriers (nonempty `migration_legacy`) ARE counted — the wall holds the content
/// unprocessed, and the T-919 drain fills main_goal when it decomposes the wall,
/// shrinking this pin in the same commit.
pub const MAIN_GOAL_DEBT_PIN: usize = 0;

/// The one main_goal-debt instrument (t920 spec §Schema changes): a live
/// (queued/ready/running/review) work ticket whose `main_goal` is empty/absent.
/// Shared by the check-side counter and the store ratchet test.
pub fn main_goal_is_debt(w: &WorkTicket) -> bool {
    w.status.name().is_live() && w.main_goal.as_deref().unwrap_or("").trim().is_empty()
}

/// T-920.1 ready-tier body obligation (t920 spec Decisions log #2): the six fields a
/// ready/running/review/shipped WORK ticket must carry nonempty, in the
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
