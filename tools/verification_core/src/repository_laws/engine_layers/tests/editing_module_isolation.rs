//! Tests for [`super`] — rules 4 and 7 over the map engine's `editing` module, and the
//! per-rule results of [`super::EngineLayerReport`].
//!
//! `editing` is one of the eleven top-level modules of `website-map-engine`, so the mission
//! compiler under `data/scenario/` and the document under `data/` may name it no more than they
//! may name `streaming`: the Mission Creator's tools sit above the document, never below it.

use super::fixture_repository::*;
use super::*;

/// RULE 4, RED — the mission compiler reaching up into the Mission Creator's tools, by `crate::`
/// path and by a `super::` chain.
#[test]
fn the_scenario_tree_naming_the_editing_module_fails() {
    let r = Repo::new("rule4-editing");
    r.map(
        "data/scenario/compiler/flatten/commands.rs",
        "use crate::editing::hosted_commands::summarise;\n\
         use super::super::super::editing::history::UndoStack;\n",
    );
    r.expect(
        1,
        &[
            "FAIL: the authored mission reaches outside its own tree:",
            "  unpinned file — 2 site(s):",
            "apps/website/map-engine/src/data/scenario/compiler/flatten/commands.rs:1:\
             use crate::editing::hosted_commands::summarise;",
            "apps/website/map-engine/src/data/scenario/compiler/flatten/commands.rs:2:\
             use super::super::super::editing::history::UndoStack;",
            RULE4_TAIL[0],
            "1 scenario-isolation finding(s)",
        ],
    );
}

/// RULE 7, RED — the document naming the Mission Creator's tools, by `crate::` path and by a
/// `super::` chain.
#[test]
fn the_document_naming_the_editing_module_breaches_the_wall() {
    let r = Repo::new("rule7-editing");
    r.map(
        "data/store/rows/undo.rs",
        "use crate::editing::history::UndoStack;\n\
         use super::super::super::editing::history::UndoStack;\n",
    );
    r.expect(
        1,
        &[
            "FAIL: the world/data wall is breached:",
            "  data/ names the world — apps/website/map-engine/src/data/store/rows/undo.rs:1:\
             use crate::editing::history::UndoStack;",
            "  data/ names the world — apps/website/map-engine/src/data/store/rows/undo.rs:2:\
             use super::super::super::editing::history::UndoStack;",
            RULE7_TAIL[0],
            "2 world/data finding(s)",
        ],
    );
}

/// Rules 4 and 7's matchers over `editing`, one line at a time: both spellings match, and a
/// module whose name only starts with `editing` does not.
#[test]
fn rules_4_and_7_match_the_editing_module_and_nothing_adjacent() {
    for (name, matcher) in [("rule 4", RULE4_RE), ("rule 7 data", RULE7_DATA_RE)] {
        let p = Pattern::regex(matcher).unwrap();
        for bad in [
            "use crate::editing::history::UndoStack;",
            "    let s = crate::editing::hosted_commands::summarise(&doc);",
            "use super::super::editing::history::UndoStack;",
        ] {
            assert!(p.is_match(bad), "{name} must match {bad:?}");
        }
        for ok in [
            "use crate::editing_notes::Note;",
            "use crate::data::scenario::editing_notes::Note;",
            "// the editing tools read this module; it never reads them",
        ] {
            assert!(!p.is_match(ok), "{name} must not match {ok:?}");
        }
    }
}

/// The clean fixture judges all eight rules, each at zero findings, and passes.
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
    r.src("lib.rs", "use website_map_engine::frame::DrawBatch;\n");
    let report = check_engine_layers(&r.0);
    assert_eq!(report.exit_code, 1);
    assert!(report.judged_every_rule());
    let rule_1 = report
        .rule(EngineLayerRule::GraphicsEngineImportsNoMapEngine)
        .expect("rule 1 is judged");
    assert_eq!(rule_1.findings, 1);
    assert_eq!(
        rule_1.detail,
        ["apps/website/graphics-engine/src/lib.rs:1:use website_map_engine::frame::DrawBatch;"]
    );
    for result in &report.rule_results {
        if result.rule != EngineLayerRule::GraphicsEngineImportsNoMapEngine {
            assert_eq!(result.findings, 0, "rule {}", result.rule.number());
        }
    }
}

/// A checkout the walk cannot read judges no rule at all, so no caller can mistake it for eight
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
