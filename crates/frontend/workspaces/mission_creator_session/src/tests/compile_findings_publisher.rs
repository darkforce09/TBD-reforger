//! The compile-findings publisher: the unregistered default, one run per call, and replacement.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mission_validation::{Finding, Primitive, Severity};

use super::{publish_compile_findings, register_compile_findings_publisher};

fn finding(rule_id: &'static str) -> Finding {
    Finding {
        rule_id: rule_id.into(),
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        message: "the compile dropped a value".to_string(),
        subject: "/editor/slots/0/rank".to_string(),
        subject_id: Some("s1".into()),
    }
}

#[test]
fn an_unregistered_publisher_shows_the_findings_nowhere() {
    assert!(
        !publish_compile_findings(&[finding("COMPILE-DROP-SLOT-RANK")]),
        "with nothing registered the findings reach no panel"
    );
}

#[test]
fn the_registered_publisher_runs_once_per_call_with_the_findings() {
    let seen = Rc::new(RefCell::new(Vec::<Vec<String>>::new()));
    let sink = Rc::clone(&seen);
    register_compile_findings_publisher(Rc::new(move |findings: &[Finding]| {
        sink.borrow_mut()
            .push(findings.iter().map(|f| f.rule_id.to_string()).collect());
    }));
    let findings = [
        finding("COMPILE-DROP-SQUAD-LEADER"),
        finding("COMPILE-DROP-SLOT-RANK"),
    ];
    assert!(publish_compile_findings(&findings));
    assert!(
        publish_compile_findings(&[]),
        "a clean compile publishes its empty list too"
    );
    assert_eq!(
        *seen.borrow(),
        [
            vec![
                "COMPILE-DROP-SQUAD-LEADER".to_string(),
                "COMPILE-DROP-SLOT-RANK".to_string()
            ],
            Vec::new()
        ]
    );
}

#[test]
fn a_second_registration_replaces_the_first() {
    let first = Rc::new(Cell::new(0u32));
    let second = Rc::new(Cell::new(0u32));
    let first_count = Rc::clone(&first);
    register_compile_findings_publisher(Rc::new(move |_findings: &[Finding]| {
        first_count.set(first_count.get() + 1);
    }));
    let second_count = Rc::clone(&second);
    register_compile_findings_publisher(Rc::new(move |_findings: &[Finding]| {
        second_count.set(second_count.get() + 1);
    }));
    assert!(publish_compile_findings(&[]));
    assert_eq!(
        (first.get(), second.get()),
        (0, 1),
        "a remount's registration replaces the earlier one and never stacks"
    );
}
