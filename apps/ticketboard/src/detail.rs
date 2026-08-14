//! Detail-panel body model (T-918.3 / B.3) — pure, unit-tested, no egui types.
//!
//! The ten v2 body fields (T-917 spec §Body) render as clearly separated labeled
//! sections in ONE pinned order — [`body_field_order`] is the authority, test-pinned.
//! Each label carries its one-line anti-blend definition ([`BodyField::definition`])
//! as a hover tooltip, so the acceptance-vs-verify distinction is visible exactly
//! where authors blend them. `migration_legacy` is NOT one of the ten: it is the
//! T-917.3 byte-reversible wall quarantine, rendered AFTER them as its own visually
//! quarantined section ([`BodySection::Quarantine`]) with the copy-for-triage
//! affordance ([`triage_block`]) feeding the Program T drain.

use crate::board::TicketView;

/// The ten typed body fields of ticket schema v2 (spec §Body).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyField {
    Summary,
    UserStory,
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
            BodyField::UserStory => "user_story",
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
            BodyField::UserStory => "Who benefits and why",
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
    /// three scalars — summary, user_story, notes — render as wrapped text.
    pub fn is_list(self) -> bool {
        match self {
            BodyField::Summary | BodyField::UserStory | BodyField::Notes => false,
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

/// The PINNED render order of the ten body fields — the T-918.3 acceptance pin
/// (test-asserted; the detail panel iterates exactly this through
/// [`body_region_order`]).
pub fn body_field_order() -> [BodyField; 10] {
    [
        BodyField::Summary,
        BodyField::UserStory,
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

/// One row of the detail body region: a typed field section, or the quarantine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodySection {
    Field(BodyField),
    /// `migration_legacy` — not a body field; always after all ten.
    Quarantine,
}

/// Everything the body region renders, in order: the ten pinned fields, then the
/// quarantine. The UI iterates exactly this list, so the "migration_legacy renders
/// after the ten" acceptance is pinned by the same fn the paint path consumes.
pub fn body_region_order() -> Vec<BodySection> {
    body_field_order()
        .into_iter()
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
    /// Scalar field (summary / user_story / notes) — wrapped text.
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
        BodyField::UserStory => scalar(v.user_story),
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
mod tests {
    use super::*;
    use crate::board::view;
    use crate::testutil::work;

    /// T-918.3 acceptance: the ten-field order is PINNED — exactly these labels,
    /// exactly this order; `migration_legacy` is NOT one of the ten and the body
    /// region renders it strictly after all of them.
    #[test]
    fn section_order_is_pinned_and_quarantine_is_last() {
        let labels: Vec<&str> = body_field_order().iter().map(|f| f.as_str()).collect();
        assert_eq!(
            labels,
            vec![
                "summary",
                "user_story",
                "context",
                "requirement",
                "current_state",
                "approach",
                "verify",
                "acceptance",
                "citations",
                "notes",
            ]
        );
        assert!(
            !labels.contains(&"migration_legacy"),
            "migration_legacy must not be a body field"
        );
        let region = body_region_order();
        assert_eq!(region.len(), 11);
        for (i, field) in body_field_order().into_iter().enumerate() {
            assert_eq!(region[i], BodySection::Field(field));
        }
        assert_eq!(
            region[10],
            BodySection::Quarantine,
            "quarantine renders after all ten body fields"
        );
        assert_eq!(
            region
                .iter()
                .filter(|s| **s == BodySection::Quarantine)
                .count(),
            1
        );
    }

    /// Anti-blend tooltips: every field carries a nonempty definition, all ten
    /// are distinct, and the verify-vs-acceptance pair names its distinction
    /// (commands-to-run vs outcome criteria).
    #[test]
    fn definitions_are_distinct_anti_blend_one_liners() {
        let defs: Vec<&str> = body_field_order().iter().map(|f| f.definition()).collect();
        assert!(defs.iter().all(|d| !d.is_empty()));
        let mut unique = defs.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), defs.len(), "definitions must not blend");
        assert_eq!(
            BodyField::Verify.definition(),
            "Commands to run — how to prove"
        );
        assert_eq!(
            BodyField::Acceptance.definition(),
            "Outcome criteria — what must be true"
        );
    }

    /// Em-dash-on-absent predicate: missing/blank scalars and empty lists come
    /// back `Absent` (rendered as the pinned muted marker); present content maps
    /// to the field's shape.
    #[test]
    fn absent_and_present_content_model() {
        assert_eq!(ABSENT_MARKER, "—");
        // A bare work ticket: no summary/user_story/notes, every list empty.
        let bare = work("T-1", "status = \"idea\"", "");
        let v = view(&bare);
        for field in body_field_order() {
            assert_eq!(
                section_content(field, &v),
                SectionContent::Absent,
                "{} must be Absent on a bare ticket",
                field.as_str()
            );
        }
        // Whitespace-blank scalar is still absent.
        let blank = work("T-2", "status = \"idea\"", "summary = \"   \"\n");
        assert_eq!(
            section_content(BodyField::Summary, &view(&blank)),
            SectionContent::Absent
        );
        // Present content: scalars → Text, lists → verbatim Lines.
        let full = work(
            "T-3",
            "status = \"idea\"",
            "summary = \"one breath\"\nuser_story = \"who and why\"\nnotes = \"leftover\"\n\
             context = [\"why now\"]\nrequirement = [\"ask 1\", \"ask 2\"]\n\
             current_state = [\"repro\"]\napproach = [\"step\"]\nverify = [\"cargo test\"]\n\
             acceptance = [\"outcome\"]\ncitations = [\"docs/spec.md\"]\n",
        );
        let v = view(&full);
        assert_eq!(
            section_content(BodyField::Summary, &v),
            SectionContent::Text("one breath".into())
        );
        assert_eq!(
            section_content(BodyField::UserStory, &v),
            SectionContent::Text("who and why".into())
        );
        assert_eq!(
            section_content(BodyField::Notes, &v),
            SectionContent::Text("leftover".into())
        );
        assert_eq!(
            section_content(BodyField::Requirement, &v),
            SectionContent::Lines(vec!["ask 1".into(), "ask 2".into()])
        );
        assert_eq!(
            section_content(BodyField::Verify, &v),
            SectionContent::Lines(vec!["cargo test".into()])
        );
        // Every field is list-or-scalar exactly as the skeleton claims.
        for field in body_field_order() {
            match section_content(field, &v) {
                SectionContent::Lines(_) => assert!(field.is_list()),
                SectionContent::Text(_) => assert!(!field.is_list()),
                SectionContent::Absent => panic!("{} set on the full ticket", field.as_str()),
            }
        }
    }

    /// List rendering model: 1-based `N. entry` lines, order preserved, entries
    /// verbatim.
    #[test]
    fn numbered_lines_model() {
        assert!(numbered_lines(&[]).is_empty());
        let lines = numbered_lines(&["first".into(), "second — verbatim".into()]);
        assert_eq!(lines, vec!["1. first", "2. second — verbatim"]);
    }

    /// Collapse threshold logic: everything at/under ~8 lines or when expanded;
    /// first 8 + hidden remainder otherwise.
    #[test]
    fn legacy_collapse_threshold() {
        assert_eq!(LEGACY_COLLAPSE_THRESHOLD, 8);
        assert_eq!(legacy_visible(0, false), (0, 0));
        assert_eq!(legacy_visible(8, false), (8, 0));
        assert_eq!(legacy_visible(9, false), (8, 1));
        assert_eq!(legacy_visible(30, false), (8, 22));
        assert_eq!(legacy_visible(30, true), (30, 0));
        assert_eq!(legacy_visible(9, true), (9, 0));
    }

    /// The triage block carries the id, the verbatim newline-joined legacy text,
    /// and the empty ten-field skeleton in pinned order — the Program T feed.
    #[test]
    fn triage_block_template_content() {
        let legacy = vec![
            "wall line one, verbatim".to_owned(),
            "wall line two: paths/and \"quotes\" survive".to_owned(),
        ];
        let block = triage_block("T-123.4", &legacy);
        assert!(block.contains("T-123.4"));
        assert!(block.contains("Program T drain"));
        // Verbatim join — the byte-reversible \n join, unparaphrased.
        assert!(
            block.contains("wall line one, verbatim\nwall line two: paths/and \"quotes\" survive")
        );
        // The skeleton: all ten keys in pinned order, scalars "" and lists [].
        let skeleton = triage_skeleton();
        assert!(block.contains(&skeleton));
        assert_eq!(
            skeleton,
            "summary = \"\"\n\
             user_story = \"\"\n\
             context = []\n\
             requirement = []\n\
             current_state = []\n\
             approach = []\n\
             verify = []\n\
             acceptance = []\n\
             citations = []\n\
             notes = \"\""
        );
        // The skeleton is EMPTY fields only — triage fills them by hand.
        assert!(!skeleton.contains("migration_legacy"));
    }
}
