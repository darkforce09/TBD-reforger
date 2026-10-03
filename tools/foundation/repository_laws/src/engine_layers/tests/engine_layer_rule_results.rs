//! Tests for [`super`] — the per-rule results of [`super::EngineLayerReport`]: a clean checkout
//! judges every rule at zero findings, a breach is recorded against its own rule, and a refused
//! run judges none.

use super::fixture_repository::*;
use super::*;

/// The clean fixture judges all six rules, each at zero findings, and passes.
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

/// A checkout the walk cannot read judges no rule at all, so no caller can mistake it for six
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
