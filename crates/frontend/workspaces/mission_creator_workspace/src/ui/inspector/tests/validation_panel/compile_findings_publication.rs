//! The compile's structured result reaches the validation panel: published directly as panel
//! rows, and through the publisher the panel registers for the session's compiled export.

use mission_validation::Finding;
use mission_validation::Primitive;
use mission_validation::Severity;

use super::{
    PanelFinding, Rollup, evaluate_now, publish_compile_findings,
    register_compile_findings_publisher,
};
use mission_creator_session::compile_findings_publisher;

/// A finding shaped like the ones a compile emits.
fn finding(rule_id: &'static str, severity: Severity, subject_id: Option<&str>) -> Finding {
    Finding {
        rule_id: rule_id.into(),
        severity,
        primitive: Primitive::PerObjectInvariant,
        message: "the compile dropped a value".to_string(),
        subject: "/editor/slots/0/rank".to_string(),
        subject_id: subject_id.map(Into::into),
    }
}

#[test]
fn compile_findings_reach_the_validation_panel() {
    // Baseline: nothing published, nothing shown (no payload source is registered on the host).
    publish_compile_findings(Vec::new());
    assert!(
        Rollup::of(&evaluate_now()).is_empty(),
        "the panel starts empty"
    );

    let findings = [
        finding("COMPILE-DROP-SQUAD-LEADER", Severity::Warning, Some("sq1")),
        finding("COMPILE-DROP-SLOT-RANK", Severity::Info, Some("s1")),
    ];
    publish_compile_findings(findings.iter().map(PanelFinding::from_finding).collect());

    let rows = evaluate_now();
    assert_eq!(rows.len(), 2, "{rows:?}");
    let rollup = Rollup::of(&rows);
    assert_eq!((rollup.errors, rollup.warnings, rollup.infos), (0, 1, 1));
    assert_eq!(rollup.chip_text(), "1 warning · 1 info");
    // The owning entity id survived, which is what makes the row clickable — the validation
    // findings' `subject_id` vocabulary reused rather than a parallel one invented.
    let leader = rows
        .iter()
        .find(|r| r.rule_id == "COMPILE-DROP-SQUAD-LEADER")
        .expect("the leader finding rendered");
    assert_eq!(
        leader.subject_id.as_ref().map(|id| id.as_str()),
        Some("sq1")
    );
    assert!(leader.is_selectable());

    // A clean compile CLEARS the previous build report — otherwise the panel would show a stale
    // list after the author fixed everything in it.
    publish_compile_findings(Vec::new());
    assert!(
        Rollup::of(&evaluate_now()).is_empty(),
        "a clean compile must clear the previous compile's findings"
    );
}

/// The session's compiled export reaches the panel only through the registered publisher: before
/// the panel registers, a publish shows nothing; after it, the findings become panel rows, and an
/// empty publish clears them.
#[test]
fn the_registered_publisher_carries_the_export_findings_into_the_panel() {
    publish_compile_findings(Vec::new());
    let findings = [
        finding("COMPILE-DROP-SQUAD-LEADER", Severity::Warning, Some("sq1")),
        finding("COMPILE-DROP-SLOT-RANK", Severity::Info, Some("s1")),
    ];
    assert!(
        !compile_findings_publisher::publish_compile_findings(&findings),
        "nothing is registered before the panel registers"
    );
    assert!(Rollup::of(&evaluate_now()).is_empty());

    register_compile_findings_publisher();
    assert!(compile_findings_publisher::publish_compile_findings(
        &findings
    ));
    let rows = evaluate_now();
    assert_eq!(
        rows,
        findings
            .iter()
            .map(PanelFinding::from_finding)
            .collect::<Vec<_>>()
    );

    // A remount registers again: the registration replaces, so one publish still yields one set.
    register_compile_findings_publisher();
    assert!(compile_findings_publisher::publish_compile_findings(
        &findings
    ));
    assert_eq!(evaluate_now().len(), 2);

    assert!(compile_findings_publisher::publish_compile_findings(&[]));
    assert!(
        Rollup::of(&evaluate_now()).is_empty(),
        "a clean compile published through the hook clears the panel"
    );
}
