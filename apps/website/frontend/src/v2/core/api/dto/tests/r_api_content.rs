//! Captured-response round trips for the published-content and log payloads.

use super::*;

#[test]
fn modpack_current() {
    assert_golden::<ModpackDto>(golden!("GET__modpacks__current.json"), &[]);
}

#[test]
fn modpacks_list_envelope() {
    assert_golden::<DataEnvelope<Value>>(golden!("GET__modpacks.json"), &["data/*"]);
}

/// Still `Value`, and that is the honest statement: no DTO reads `/announcements` — the page
/// itself takes `Paginated<Value>`. Type this the day an `AnnouncementDto` lands.
#[test]
fn announcements_envelope() {
    assert_golden::<Paginated<Value>>(golden!("GET__announcements.json"), &["data/*"]);
}

#[test]
fn wiki_envelope() {
    assert_golden::<DataEnvelope<Value>>(golden!("GET__wiki.json"), &["data/*"]);
}

#[test]
fn vehicle_db_envelope() {
    assert_golden::<DataEnvelope<Value>>(golden!("GET__vehicle-database.json"), &["data/*"]);
}

/// Cursor envelope, not offset/total — and `Value` is honest here: the audit page reads
/// `CursorList<Value>` too.
#[test]
fn audit_logs_envelope() {
    assert_golden::<CursorList<Value>>(
        golden!("GET__admin__audit-logs.json"),
        &["data/*", "next_cursor"],
    );
}
