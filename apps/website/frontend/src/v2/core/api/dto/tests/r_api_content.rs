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

/// The Comms Broadcaster's own list, which is drafts **and** published — the public feed is
/// published only, so a corpus built from it alone leaves the draft branch of the surface unfed.
/// Still `Paginated<Value>` because that is what the page reads; the structural claim below is
/// what pins the wire shape.
#[test]
fn cms_announcements_envelope() {
    assert_golden::<Paginated<Value>>(golden!("GET__cms__announcements.json"), &["data/*"]);
}

/// The CMS rows and the public-feed rows are the same backend model, so a CMS row may carry no key
/// the published feed does not. The four optional keys are the ones the model skips when empty —
/// a draft has no publication instant, and an announcement with no hero image sends neither
/// `thumbnail_url` nor an empty string for it.
#[test]
fn cms_announcement_rows_carry_the_public_feed_shape_plus_drafts() {
    const OPTIONAL: [&str; 4] = [
        "published_at",
        "snippet",
        "thumbnail_url",
        "discord_message_id",
    ];

    let cms: Paginated<Value> =
        serde_json::from_str(golden!("GET__cms__announcements.json")).unwrap();
    let public: Paginated<Value> =
        serde_json::from_str(golden!("GET__announcements.json")).unwrap();

    let keys = |row: &Value| -> Vec<String> {
        row.as_object()
            .expect("an announcement row is an object")
            .keys()
            .cloned()
            .collect()
    };
    let known: Vec<String> = keys(&public.data[0]);
    let required: Vec<&String> = known
        .iter()
        .filter(|k| !OPTIONAL.contains(&k.as_str()))
        .collect();

    for row in &cms.data {
        let present = keys(row);
        for key in &present {
            assert!(
                known.contains(key),
                "CMS row {} carries {key}, which the published feed's model does not",
                row["id"]
            );
        }
        for key in &required {
            assert!(
                present.contains(key),
                "CMS row {} is missing the always-sent key {key}",
                row["id"]
            );
        }
    }

    let statuses: Vec<&str> = cms
        .data
        .iter()
        .filter_map(|r| r["status"].as_str())
        .collect();
    assert!(
        statuses.contains(&"draft") && statuses.contains(&"published"),
        "the CMS corpus must exercise both branches the list query returns, got {statuses:?}"
    );
    assert!(
        statuses.iter().all(|s| *s != "archived"),
        "list_cms_announcements filters archived rows out"
    );
    // A draft has not been published, so it must not claim a publication instant.
    for row in cms.data.iter().filter(|r| r["status"] == "draft") {
        assert!(
            row.get("published_at").is_none(),
            "draft {} must not carry published_at",
            row["id"]
        );
    }
}
