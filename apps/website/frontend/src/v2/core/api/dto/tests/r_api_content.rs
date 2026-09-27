//! Captured-response round trips for the published-content and log payloads.

use super::*;
use crate::v2::core::api::dto::administration::AuditLogEntry;
use crate::v2::core::api::dto::vehicles::Vehicle;
use crate::v2::core::api::dto::wiki::{
    WikiArticle, WikiPageSummary, WikiRevision, WikiRevisionPage,
};

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

/// The navigation list: summaries only, no markdown. The capture holds a page without an icon, so
/// the absent-icon arm is covered.
#[test]
fn wiki_page_list() {
    const G: &str = golden!("GET__wiki.json");
    assert_golden::<DataEnvelope<WikiPageSummary>>(G, &[]);
    let list: DataEnvelope<WikiPageSummary> = serde_json::from_str(G).unwrap();
    assert!(
        list.data.iter().any(|page| page.icon.is_empty())
            && list.data.iter().any(|page| !page.icon.is_empty()),
        "the wiki list golden must hold a page with and a page without an icon"
    );
}

/// The article the V-suite's `/wiki/field-manual` route reads.
#[test]
fn wiki_article_field_manual() {
    assert_golden::<WikiArticle>(golden!("GET__wiki__field-manual.json"), &[]);
}

/// The formatting guide shows every construct the tree can carry, so this round trip decodes and
/// re-encodes every block and inline variant (`tests/wiki.rs` checks the coverage).
#[test]
fn wiki_article_formatting_guide() {
    assert_golden::<WikiArticle>(golden!("GET__wiki__wiki-formatting-guide.json"), &[]);
}

#[test]
fn wiki_revision_history() {
    const G: &str = golden!("GET__wiki__field-manual__revisions.json");
    assert_golden::<WikiRevisionPage>(G, &[]);
    let history: WikiRevisionPage = serde_json::from_str(G).unwrap();
    assert_eq!(history.total, history.items.len() as i64);
    assert_eq!(
        history.items[0].revision, 1,
        "a seeded page starts at revision 1"
    );
}

/// A revision is the article's content under the revision's own keys: the author and creation
/// time take the place of the editor and update time, and there is no page `id`. Built from the
/// captured article, so every block shape the capture holds is decoded as a revision too.
#[test]
fn wiki_revision_built_from_the_captured_article() {
    let article: Value = serde_json::from_str(golden!("GET__wiki__field-manual.json")).unwrap();
    let revision = json!({
        "slug": article["slug"],
        "revision": article["revision"],
        "category": article["category"],
        "title": article["title"],
        "icon": article["icon"],
        "nav_order": article["nav_order"],
        "body_md": article["body_md"],
        "author_id": article["updated_by"],
        "created_at": article["updated_at"],
        "blocks": article["blocks"],
    });
    assert_golden::<WikiRevision>(&revision.to_string(), &[]);
}

/// The capture holds a row with every optional field and one with none of them, so both arms of
/// the absent-when-empty fields are covered.
#[test]
fn vehicle_database_list() {
    const G: &str = golden!("GET__vehicle-database.json");
    assert_golden::<DataEnvelope<Vehicle>>(G, &[]);
    let list: DataEnvelope<Vehicle> = serde_json::from_str(G).unwrap();
    assert!(list.data.iter().any(|v| v.amphibious.is_empty()
        && v.primary_threat.is_empty()
        && v.profile_image_url.is_empty()));
    assert!(list.data.iter().any(|v| !v.amphibious.is_empty()
        && !v.primary_threat.is_empty()
        && !v.profile_image_url.is_empty()));
}

/// Cursor envelope, not offset/total. `metadata` is the writer's own context and is carried as
/// sent, and `next_cursor` is the shared envelope's opaque cursor, so those two stay unread.
#[test]
fn audit_logs_envelope() {
    assert_golden::<CursorList<AuditLogEntry>>(
        golden!("GET__admin__audit-logs.json"),
        &["data/*/metadata", "next_cursor"],
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
