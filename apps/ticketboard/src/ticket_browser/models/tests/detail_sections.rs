use super::*;
use crate::test_support::work;
use crate::ticket_registry::models::projection::view;

/// acceptance, DELIBERATELY updated at the ten-FIELD
/// order stays pinned (the triage skeleton depends on it), but the RENDER
/// region drops the header fields — body sections start at context — and
/// `migration_legacy` still renders strictly after every body section.
#[test]
fn section_order_is_pinned_and_quarantine_is_last() {
    let labels: Vec<&str> = body_field_order().iter().map(|f| f.as_str()).collect();
    assert_eq!(
        labels,
        vec![
            "summary",
            "main_goal",
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
    // The RENDER order: header fields gone, context first,
    // quarantine last — exactly once.
    let expected = [
        BodyField::Context,
        BodyField::Requirement,
        BodyField::CurrentState,
        BodyField::Approach,
        BodyField::Verify,
        BodyField::Acceptance,
        BodyField::Citations,
        BodyField::Notes,
    ];
    let region = body_region_order();
    assert_eq!(region.len(), expected.len() + 1);
    assert_eq!(
        region[0],
        BodySection::Field(BodyField::Context),
        "body sections start at context (T-920.2 acceptance)"
    );
    for (i, field) in expected.into_iter().enumerate() {
        assert_eq!(region[i], BodySection::Field(field));
    }
    assert_eq!(
        region[expected.len()],
        BodySection::Quarantine,
        "quarantine renders after every body section"
    );
    assert_eq!(
        region
            .iter()
            .filter(|s| **s == BodySection::Quarantine)
            .count(),
        1
    );
    for header_field in header_field_order() {
        assert!(
            !region.contains(&BodySection::Field(header_field)),
            "{} renders in the header, never in the body list",
            header_field.as_str()
        );
    }
}

/// header model pin: main_goal then summary render in the header
/// (and nowhere else — header ∪ body-render fields partition the ten
/// exactly); present scalars come through in order, absent AND
/// whitespace-blank fields are OMITTED outright (the em-dash marker stays
/// a body-list rule).
#[test]
fn header_model_is_pinned() {
    assert_eq!(
        header_field_order(),
        [BodyField::MainGoal, BodyField::Summary],
        "main_goal directly under the title, the first thing read"
    );
    // Partition: header + body-rendered fields cover the ten with no
    // overlap (10 slots, all 10 distinct fields present ⇒ each once).
    let rendered = body_region_order().into_iter().filter_map(|s| match s {
        BodySection::Field(field) => Some(field),
        BodySection::Quarantine => None,
    });
    let union: Vec<BodyField> = header_field_order().into_iter().chain(rendered).collect();
    assert_eq!(union.len(), 10, "header and body partition the ten fields");
    for field in body_field_order() {
        assert!(
            union.contains(&field),
            "{} lost in the split",
            field.as_str()
        );
    }

    // Content model: present scalars in header order.
    let full = work(
        "T-1",
        "status = \"idea\"",
        "summary = \"one breath\"\nmain_goal = \"the goal, read first\"\n",
    );
    assert_eq!(
        header_lines(&view(&full)),
        vec![
            (BodyField::MainGoal, "the goal, read first".to_owned()),
            (BodyField::Summary, "one breath".to_owned()),
        ]
    );
    // Absent = omitted — the header never renders an em-dash.
    let bare = work("T-2", "status = \"idea\"", "");
    assert!(
        header_lines(&view(&bare)).is_empty(),
        "absent header fields are omitted, never marked"
    );
    // Whitespace-blank is absent too; the other field still renders.
    let blank_goal = work(
        "T-3",
        "status = \"idea\"",
        "summary = \"s\"\nmain_goal = \"   \"\n",
    );
    assert_eq!(
        header_lines(&view(&blank_goal)),
        vec![(BodyField::Summary, "s".to_owned())],
        "blank main_goal is omitted; summary still renders"
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
    // A bare work ticket: no summary/main_goal/notes, every list empty.
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
        "summary = \"one breath\"\nmain_goal = \"who and why\"\nnotes = \"leftover\"\n\
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
        section_content(BodyField::MainGoal, &v),
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
    assert!(block.contains("wall line one, verbatim\nwall line two: paths/and \"quotes\" survive"));
    // The skeleton: all ten keys in pinned order, scalars "" and lists [].
    let skeleton = triage_skeleton();
    assert!(block.contains(&skeleton));
    assert_eq!(
        skeleton,
        "summary = \"\"\n\
             main_goal = \"\"\n\
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
