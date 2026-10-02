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

/// A pool that never connects and spawns no maintenance task, so it needs no database and no
/// runtime: its connect-options allocation is the only thing the registry keys on.
fn unconnected_pool(database: &str) -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_lazy_with(
            sqlx::postgres::PgConnectOptions::new()
                .host("registry-test.invalid")
                .port(5432)
                .username("registry_test")
                .database(database),
        )
}

/// RED: key the registry on the connect-options address alone and never prune — the dropped
/// pool's entry either survives (the registry still owns its state) or is handed to the next
/// pool allocated at the freed address (the two handles share one state).
#[test]
fn a_dropped_pool_leaves_no_entry_for_a_new_pool_to_reuse() {
    let first = unconnected_pool("audit_notifier_first");
    let first_notify = AuditNotify::for_pool(&first);
    drop(first);

    let second = unconnected_pool("audit_notifier_second");
    let second_notify = AuditNotify::for_pool(&second);

    assert!(
        !Arc::ptr_eq(&first_notify.shared, &second_notify.shared),
        "the new pool reused the dropped pool's listener state"
    );
    assert_eq!(
        second_notify.shared.label,
        "registry_test@registry-test.invalid:5432/audit_notifier_second"
    );
    assert_eq!(
        Arc::strong_count(&first_notify.shared),
        1,
        "the registry still holds the dropped pool's listener state"
    );
}
