use super::*;

/// RED: cap the doubling at the wrong ceiling, or stop doubling — the chain below diverges.
#[test]
fn backoff_doubles_and_caps() {
    let mut d = BACKOFF_INITIAL;
    let mut chain = Vec::new();
    for _ in 0..8 {
        chain.push(d.as_millis());
        d = next_backoff(d);
    }
    assert_eq!(chain, [250, 500, 1000, 2000, 4000, 5000, 5000, 5000]);
    assert_eq!(next_backoff(BACKOFF_MAX), BACKOFF_MAX);
}

/// RED: forward every channel, or map a non-numeric payload to `None` — the asserts fire.
#[test]
fn signal_for_reads_the_row_id_and_ignores_other_channels() {
    assert_eq!(signal_for(AUDIT_CHANNEL, "42"), Some(AuditSignal::Row(42)));
    assert_eq!(signal_for(AUDIT_CHANNEL, " 7 "), Some(AuditSignal::Row(7)));
    assert_eq!(
        signal_for(AUDIT_CHANNEL, "not-an-id"),
        Some(AuditSignal::Resync)
    );
    assert_eq!(signal_for(AUDIT_CHANNEL, ""), Some(AuditSignal::Resync));
    assert_eq!(signal_for("server_status", "42"), None);
}
