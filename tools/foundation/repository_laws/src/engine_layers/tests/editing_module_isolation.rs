//! Tests for [`super`] — rule 7 over the map engine's `editing` module, and the per-rule results
//! of [`super::EngineLayerReport`].
//!
//! `editing` hosts the live mission document and its undo drive, so the static world may name it
//! no more than it may name `mission_document`: the Mission Creator's tools sit above the world,
//! never below it.

use super::fixture_repository::*;
use super::*;

/// RULE 7, RED — the static world naming the Mission Creator's editing module, by `crate::` path
/// and by a `super::` chain.
#[test]
fn the_world_naming_the_editing_module_breaches_the_wall() {
    let r = Repo::new("rule7-editing");
    r.map(
        "world/terrain/dem/undo.rs",
        "use crate::editing::history::UndoStack;\n\
         use super::super::super::editing::history::UndoStack;\n",
    );
    r.expect(
        1,
        &[
            "FAIL: the static world names the authored document:",
            "  world/ names the document — legacy/map_engine/src/world/terrain/dem/undo.rs:1:\
             use crate::editing::history::UndoStack;",
            "  world/ names the document — legacy/map_engine/src/world/terrain/dem/undo.rs:2:\
             use super::super::super::editing::history::UndoStack;",
            RULE7_TAIL[0],
            "2 world-document finding(s)",
        ],
    );
}

/// Rule 7's matcher over `editing`, one line at a time: both spellings match, and a module whose
/// name only starts with `editing`, or prose naming it, does not.
#[test]
fn rule_7_matches_the_editing_module_and_nothing_adjacent() {
    let p = Pattern::regex(RULE7_WORLD_RE).unwrap();
    for bad in [
        "use crate::editing::history::UndoStack;",
        "    let s = crate::editing::hosted_commands::summarise(&doc);",
        "use super::super::editing::history::UndoStack;",
    ] {
        assert!(p.is_match(bad), "rule 7 must match {bad:?}");
    }
    for ok in [
        "use crate::editing_notes::Note;",
        "use crate::world::editing_hints::Hint;",
        "// the editing tools read this module; it never reads them",
    ] {
        assert!(!p.is_match(ok), "rule 7 must not match {ok:?}");
    }
}

/// The clean fixture judges all seven rules, each at zero findings, and passes.
#[test]
fn a_clean_checkout_judges_every_rule_at_zero_findings() {
    let r = Repo::new("results-clean");
    let report = check_engine_layers(&r.0);
    assert_eq!(report.exit_code, 0, "{:#?}", report.lines);
    assert!(report.judged_every_rule());
    for result in &report.rule_results {
        assert_eq!(result.findings, 0, "rule {}", result.rule.number());
        assert!(result.detail.is_empty(), "rule {}", result.rule.number());
    }
}

/// A breach of one rule is recorded against that rule alone, with the report line it printed.
#[test]
fn a_breach_is_recorded_against_its_own_rule() {
    let r = Repo::new("results-breach");
    r.src("lib.rs", "use map_engine::frame::DrawBatch;\n");
    let report = check_engine_layers(&r.0);
    assert_eq!(report.exit_code, 1);
    assert!(report.judged_every_rule());
    let rule_1 = report
        .rule(EngineLayerRule::GraphicsEngineImportsNoMapEngine)
        .expect("rule 1 is judged");
    assert_eq!(rule_1.findings, 1);
    assert_eq!(
        rule_1.detail,
        ["legacy/graphics_engine/src/lib.rs:1:use map_engine::frame::DrawBatch;"]
    );
    for result in &report.rule_results {
        if result.rule != EngineLayerRule::GraphicsEngineImportsNoMapEngine {
            assert_eq!(result.findings, 0, "rule {}", result.rule.number());
        }
    }
}

/// A checkout the walk cannot read judges no rule at all, so no caller can mistake it for seven
/// clean ones.
#[test]
fn a_refused_run_judges_no_rule() {
    let report = check_engine_layers(std::path::Path::new(
        "/nonexistent/tbd-engine-layers/results",
    ));
    assert_eq!(report.exit_code, 2);
    assert!(report.rule_results.is_empty());
    assert!(!report.judged_every_rule());
}
