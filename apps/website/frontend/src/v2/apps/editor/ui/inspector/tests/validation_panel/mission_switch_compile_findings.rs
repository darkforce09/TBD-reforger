//! Validation panel mission switch compile findings tests.

use super::{
    clear_compile_findings, compile_findings, evaluate_now, publish_compile_findings, PanelFinding,
};
use website_map_engine::data::scenario::validate::Primitive;
use website_map_engine::data::scenario::validate::Severity;

fn mission_a_compile_row() -> PanelFinding {
    PanelFinding {
        rule_id: "COMPILE-DROP-SQUAD-LEADER".into(),
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        message: "mission A squad dropped".into(),
        subject: "A/Alpha".into(),
        subject_id: Some("mission-a-squad-1".into()),
    }
}

#[test]
fn a_second_mission_does_not_inherit_the_previous_missions_compile_findings() {
    publish_compile_findings(vec![mission_a_compile_row()]);
    assert_eq!(
        compile_findings().len(),
        1,
        "precondition: mission A published a compile finding"
    );
    assert_eq!(
        compile_findings()[0].subject_id.as_deref(),
        Some("mission-a-squad-1")
    );

    clear_compile_findings();

    assert!(
        compile_findings().is_empty(),
        "T-761: after hydrate clear, mission B must not inherit mission A's compile findings"
    );
    assert!(
        evaluate_now()
            .iter()
            .all(|r| r.subject_id.as_deref() != Some("mission-a-squad-1")),
        "T-761: evaluate_now must not surface mission A's subject_id on mission B; got {:?}",
        evaluate_now()
    );
}

#[test]
fn clear_compile_findings_is_the_hydrate_reset_seam() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let src = live_code(super::VALIDATION_PANEL_SOURCE);
    let body = only_body(&src, "pub fn clear_compile_findings(");
    assert!(
            body.contains("Vec::new()"),
            "T-761: clear_compile_findings must empty via Vec::new() in the production body; got:\n{body}"
        );
    assert!(
        body.contains("publish_compile_findings"),
        "T-761: clear must route through publish_compile_findings; got:\n{body}"
    );
}
