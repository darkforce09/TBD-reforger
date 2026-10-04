//! Shape checks for the audit stream's control events, which no captured fixture carries: the
//! `data` of an SSE event is read with these types, so their spelling is pinned here.

use super::*;

#[test]
fn a_ready_event_reads_its_cursor_and_floor_and_writes_them_back_unchanged() {
    const DATA: &str = r#"{"resume_after":42,"retained_after":7}"#;
    let ready: AuditStreamReady = serde_json::from_str(DATA).expect("ready data decodes");
    assert_eq!(
        ready,
        AuditStreamReady {
            resume_after: 42,
            retained_after: 7,
        }
    );
    assert_eq!(serde_json::to_string(&ready).unwrap(), DATA);
}

#[test]
fn a_reset_event_reads_both_reasons_in_snake_case() {
    for (reason, spelled) in [
        (AuditStreamResetReason::CursorAhead, "cursor_ahead"),
        (
            AuditStreamResetReason::HistoryUnavailable,
            "history_unavailable",
        ),
    ] {
        let data = format!(r#"{{"reason":"{spelled}","resume_after":90,"retained_after":12}}"#);
        let reset: AuditStreamReset = serde_json::from_str(&data).expect("reset data decodes");
        assert_eq!(
            reset,
            AuditStreamReset {
                reason,
                resume_after: 90,
                retained_after: 12,
            }
        );
        assert_eq!(serde_json::to_string(&reset).unwrap(), data);
    }
}

#[test]
fn a_reset_reason_outside_the_contract_fails_the_read() {
    let data = r#"{"reason":"rewound","resume_after":1,"retained_after":0}"#;
    assert!(serde_json::from_str::<AuditStreamReset>(data).is_err());
}

/// A system line has no actor and no target: the keys are absent on the wire, and writing the
/// line back must not invent empty strings or `null`s for them.
#[test]
fn a_system_audit_line_writes_back_without_its_absent_keys() {
    const LINE: &str = r#"{"id":15,"severity":"info","actor_name":"system","action":"event.auto_live","message":"start time reached","created_at":"2026-09-26T22:16:33Z"}"#;
    let entry: AuditLogEntry = serde_json::from_str(LINE).expect("a system line decodes");
    assert_eq!(entry.severity, AuditSeverity::Info);
    assert!(entry.actor_id.is_none() && entry.metadata.is_none());
    assert!(entry.target_type.is_empty() && entry.target_id.as_str().is_empty());
    assert_eq!(serde_json::to_string(&entry).unwrap(), LINE);
}

#[test]
fn every_audit_severity_reads_in_its_wire_spelling() {
    for (spelled, severity) in [
        ("info", AuditSeverity::Info),
        ("warn", AuditSeverity::Warn),
        ("crit", AuditSeverity::Crit),
    ] {
        let decoded: AuditSeverity = serde_json::from_str(&format!("\"{spelled}\"")).unwrap();
        assert_eq!(decoded, severity);
    }
}
