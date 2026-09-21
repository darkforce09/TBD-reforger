//! Detail-panel body model (T-918.3 / B.3; header split T-920.2) — pure,
//! unit-tested, no egui types.
//!
//! The ten v2 body fields (T-917 spec §Body) keep ONE pinned field authority —
//! [`body_field_order`], test-pinned; the triage skeleton derives from it — but
//! they RENDER split in two since T-920.2 (T-920 decision log #1): the header
//! fields ([`header_field_order`] — main_goal first, then summary) render
//! label-free in the detail HEADER directly under the title, the first thing
//! read, with absent fields OMITTED outright; the body section list
//! ([`body_region_order`]) renders the rest as clearly separated labeled
//! sections starting at context. Each body label carries its one-line
//! anti-blend definition ([`BodyField::definition`]) as a hover tooltip, so the
//! acceptance-vs-verify distinction is visible exactly where authors blend
//! them. `migration_legacy` is NOT one of the ten: it is the T-917.3
//! byte-reversible wall quarantine, rendered AFTER the body sections as its own
//! visually quarantined section ([`BodySection::Quarantine`]) with the
//! copy-for-triage affordance ([`triage_block`]) feeding the Program T drain.

use crate::board::TicketView;

/// The ten typed body fields of ticket schema v2 (spec §Body).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyField {
    Summary,
    MainGoal,
    Context,
    Requirement,
    CurrentState,
    Approach,
    Verify,
    Acceptance,
    Citations,
    Notes,
}

impl BodyField {
    /// Registry key — the section label text.
    pub fn as_str(self) -> &'static str {
        match self {
            BodyField::Summary => "summary",
            BodyField::MainGoal => "main_goal",
            BodyField::Context => "context",
            BodyField::Requirement => "requirement",
            BodyField::CurrentState => "current_state",
            BodyField::Approach => "approach",
            BodyField::Verify => "verify",
            BodyField::Acceptance => "acceptance",
            BodyField::Citations => "citations",
            BodyField::Notes => "notes",
        }
    }

    /// The one-line anti-blend definition from the spec table (§Body) — the
    /// section-label hover tooltip. Verify names commands, acceptance names
    /// outcomes: "we don't want it to blend together" (Decisions log #5).
    pub fn definition(self) -> &'static str {
        match self {
            BodyField::Summary => "What this ticket is, one breath",
            BodyField::MainGoal => "The one main goal — read first",
            BodyField::Context => "Why now; background facts",
            BodyField::Requirement => "The operator's ask, line by line",
            BodyField::CurrentState => "What exists today; bug repro lives here",
            BodyField::Approach => "Planned steps",
            BodyField::Verify => "Commands to run — how to prove",
            BodyField::Acceptance => "Outcome criteria — what must be true",
            BodyField::Citations => "Files/tickets/docs consulted, reference-only",
            BodyField::Notes => "Freeform leftover",
        }
    }

    /// Line-list fields (numbered rendering; `[]` in the triage skeleton). The
    /// three scalars — summary, main_goal, notes — render as wrapped text.
    pub fn is_list(self) -> bool {
        match self {
            BodyField::Summary | BodyField::MainGoal | BodyField::Notes => false,
            BodyField::Context
            | BodyField::Requirement
            | BodyField::CurrentState
            | BodyField::Approach
            | BodyField::Verify
            | BodyField::Acceptance
            | BodyField::Citations => true,
        }
    }
}

/// The PINNED order of the ten body fields — the T-918.3 acceptance pin
/// (test-asserted). Since T-920.2 this is the FIELD authority (the triage
/// skeleton, the header/body partition), not the render list itself: the
/// detail panel renders [`header_field_order`] in the header and
/// [`body_region_order`] — this order minus the header fields — below.
pub fn body_field_order() -> [BodyField; 10] {
    [
        BodyField::Summary,
        BodyField::MainGoal,
        BodyField::Context,
        BodyField::Requirement,
        BodyField::CurrentState,
        BodyField::Approach,
        BodyField::Verify,
        BodyField::Acceptance,
        BodyField::Citations,
        BodyField::Notes,
    ]
}

/// The fields the detail HEADER renders (T-920.2), in order, directly under
/// the title: `main_goal` first (T-920 decision log #1 — "directly under the
/// title, the first thing read"), then `summary`. Lifted OUT of the body
/// section list, which therefore starts at context. Header rendering is
/// label-free and OMITS absent fields entirely — the em-dash absence marker
/// ([`ABSENT_MARKER`]) is a body-list rule; the header carries content or
/// nothing.
pub fn header_field_order() -> [BodyField; 2] {
    [BodyField::MainGoal, BodyField::Summary]
}

/// The header content model ([`header_field_order`] projected through
/// [`section_content`]): what the header renders, in order — PRESENT fields
/// only, each with its text. Scalar by construction: only
/// [`SectionContent::Text`] passes, so a list field could never appear here.
pub fn header_lines(v: &TicketView<'_>) -> Vec<(BodyField, String)> {
    header_field_order()
        .into_iter()
        .filter_map(|field| match section_content(field, v) {
            SectionContent::Text(text) => Some((field, text)),
            SectionContent::Absent | SectionContent::Lines(_) => None,
        })
        .collect()
}

/// One row of the detail body region: a typed field section, or the quarantine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodySection {
    Field(BodyField),
    /// `migration_legacy` — not a body field; always after every body section.
    Quarantine,
}

/// Everything the body REGION renders, in order (T-920.2): the pinned fields
/// MINUS the header fields — so the list starts at context — then the
/// quarantine. Derived from [`body_field_order`] and [`header_field_order`],
/// so the three pins can never drift apart. The UI iterates exactly this
/// list, so "body sections start at context" and "migration_legacy renders
/// after them" are pinned by the same fn the paint path consumes.
pub fn body_region_order() -> Vec<BodySection> {
    let header = header_field_order();
    body_field_order()
        .into_iter()
        .filter(|field| !header.contains(field))
        .map(BodySection::Field)
        .chain(std::iter::once(BodySection::Quarantine))
        .collect()
}

/// The muted marker an absent/empty section renders after its label — an explicit
/// em-dash, never a silently skipped label (absence is data here).
pub const ABSENT_MARKER: &str = "—";

/// Rendered content of one body section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionContent {
    /// Absent or empty — label + the muted [`ABSENT_MARKER`].
    Absent,
    /// Scalar field (summary / main_goal / notes) — wrapped text.
    Text(String),
    /// List field — one numbered monospace line per entry ([`numbered_lines`]).
    Lines(Vec<String>),
}

/// Project one body field out of the uniform ticket view. Scalars count as absent
/// when missing OR whitespace-blank (an empty `summary = ""` is not content);
/// lists when empty. Nonempty list entries pass through verbatim — never trimmed,
/// never reflowed.
pub fn section_content(field: BodyField, v: &TicketView<'_>) -> SectionContent {
    fn scalar(value: Option<&str>) -> SectionContent {
        match value {
            Some(s) if !s.trim().is_empty() => SectionContent::Text(s.to_owned()),
            Some(_) | None => SectionContent::Absent,
        }
    }
    fn list(items: &[String]) -> SectionContent {
        if items.is_empty() {
            SectionContent::Absent
        } else {
            SectionContent::Lines(items.to_vec())
        }
    }
    match field {
        BodyField::Summary => scalar(Some(v.summary)),
        BodyField::MainGoal => scalar(v.main_goal),
        BodyField::Context => list(v.context),
        BodyField::Requirement => list(v.requirement),
        BodyField::CurrentState => list(v.current_state),
        BodyField::Approach => list(v.approach),
        BodyField::Verify => list(v.verify),
        BodyField::Acceptance => list(v.acceptance),
        BodyField::Citations => list(v.citations),
        BodyField::Notes => scalar(v.notes),
    }
}

/// Display transform for list sections: `N. entry`, one line per entry, 1-based —
/// monospace-friendly (verify commands and citation paths line up).
pub fn numbered_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}. {line}", i + 1))
        .collect()
}

// ---- migration_legacy quarantine ----

/// The quarantine banner text — T-919 is Program T, the wall-triage drain.
pub const QUARANTINE_LABEL: &str = "quarantine — pending triage (T-919)";

/// Legacy lines shown while collapsed; beyond this the block collapses by
/// default behind an expand affordance (~8 lines per the T-918.3 brief).
pub const LEGACY_COLLAPSE_THRESHOLD: usize = 8;

/// `(visible, hidden)` split of the quarantined lines: everything when expanded
/// or at/under the threshold, else the first [`LEGACY_COLLAPSE_THRESHOLD`].
pub fn legacy_visible(total: usize, expanded: bool) -> (usize, usize) {
    if expanded || total <= LEGACY_COLLAPSE_THRESHOLD {
        (total, 0)
    } else {
        (LEGACY_COLLAPSE_THRESHOLD, total - LEGACY_COLLAPSE_THRESHOLD)
    }
}

/// The empty ten-field skeleton appended to every triage block — TOML-shaped, in
/// the pinned section order, scalars as `""` and line lists as `[]`. Derived from
/// [`body_field_order`] so the skeleton can never drift from the pin.
pub fn triage_skeleton() -> String {
    body_field_order()
        .into_iter()
        .map(|f| {
            if f.is_list() {
                format!("{} = []", f.as_str())
            } else {
                format!("{} = \"\"", f.as_str())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The "Copy for triage" clipboard payload — the Program T drain feed: the ticket
/// id, the legacy prose joined with `\n` (the byte-reversible join the T-917.3
/// quarantine proved per file), and the empty ten-field skeleton to decompose
/// into. Ready to paste into a triage batch; nothing here is paraphrased.
pub fn triage_block(id: &str, legacy: &[String]) -> String {
    format!(
        "# {id} — migration_legacy triage (Program T drain)\n\
         # Decompose the verbatim legacy prose into the ten typed fields below,\n\
         # then delete migration_legacy from the ticket in the same edit.\n\
         \n\
         ## legacy (verbatim)\n\
         {}\n\
         \n\
         ## ten-field skeleton\n\
         {}\n",
        legacy.join("\n"),
        triage_skeleton()
    )
}

#[cfg(test)]
#[path = "tests/detail_tests.rs"]
mod tests;
