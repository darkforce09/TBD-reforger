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
        // An id that exists only in mission A — clicking it on B selects nothing.
        subject_id: Some("mission-a-squad-1".into()),
    }
}

/// Behaviour: hydrate clear drops the previous mission's compile findings so evaluate_now
/// cannot surface their subject_ids on the next mission.
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

    // Mission B's editor hydrate — the production call site in MissionEditorPage.
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

/// Class-R — the clear is the named hydrate seam, not an accidental empty publish buried
/// only in tests.
#[test]
fn clear_compile_findings_is_the_hydrate_reset_seam() {
    // wave-136 F3 — scope to the production body only. Whole-file `src.contains(…)` self-feeds
    // off this assert's own string literal, and a string decoy in production greened without
    // `live_code`.
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let src = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/validation_panel.rs"
    )));
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
