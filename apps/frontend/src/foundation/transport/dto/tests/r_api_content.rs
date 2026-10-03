//! Captured-response round trips for the published-content and log payloads.

use super::*;
use crate::foundation::transport::dto::administration::AuditLogEntry;
use crate::foundation::transport::dto::vehicles::Vehicle;
use crate::foundation::transport::dto::wiki::{
    WikiArticle, WikiPageSummary, WikiRevision, WikiRevisionPage,
};

#[test]
fn modpack_current() {
    assert_golden::<ModpackDto>(golden!("GET__modpacks__current.json"), &[]);
}

/// The modpacks page reads the list as `DataEnvelope<ModpackDto>`, and every key of every row is
/// a named field.
#[test]
fn modpacks_list_envelope() {
    const G: &str = golden!("GET__modpacks.json");
    assert_golden::<DataEnvelope<ModpackDto>>(G, &[]);
    let list: DataEnvelope<ModpackDto> = serde_json::from_str(G).unwrap();
    assert!(
        list.data.iter().filter(|p| p.modpack.is_current).count() <= 1,
        "at most one pack is current"
    );
}

/// The captured packs carry no mods, so a pack with two mod rows is built here: one that records
/// every optional id and one that records none. Both arms of the absent-when-empty fields
/// round-trip, and every key of a mod row is a named field.
#[test]
fn modpack_mod_rows_round_trip_with_and_without_their_optional_ids() {
    let mut pack: Value = serde_json::from_str(golden!("GET__modpacks__current.json")).unwrap();
    pack["mods"] = json!([
        {
            "id": "00000000-0000-4000-a100-000000000001",
            "modpack_id": pack["id"],
            "name": "RHS Status Quo",
            "is_key_dependency": true,
            "sort_order": 0,
            "workshop_id": "595F2BF2F44836FB",
            "mod_guid": "595F2BF2F44836FB",
            "version": "1.0.2"
        },
        {
            "id": "00000000-0000-4000-a100-000000000002",
            "modpack_id": pack["id"],
            "name": "Local texture pack",
            "is_key_dependency": false,
            "sort_order": 1
        }
    ]);
    let wire = pack.to_string();
    assert_golden::<ModpackDto>(&wire, &[]);
    let decoded: ModpackDto = serde_json::from_str(&wire).unwrap();
    let (full, bare) = (&decoded.mods[0], &decoded.mods[1]);
    assert!(full.is_key_dependency && full.version == "1.0.2");
    assert!(bare.workshop_id.is_empty() && bare.mod_guid.is_empty() && bare.version.is_empty());
}

/// The public feed: published rows only, each a named `Announcement` field for every key. The
/// capture holds a row with every optional key and one without a preview line, hero image or
/// chat message, so both arms of the absent-when-empty fields round-trip.
#[test]
fn announcements_envelope() {
    const G: &str = golden!("GET__announcements.json");
    assert_golden::<Paginated<Announcement>>(G, &[]);
    let page: Paginated<Announcement> = serde_json::from_str(G).unwrap();
    assert!(page
        .data
        .iter()
        .all(|a| a.status == "published" && a.published_at.is_some()));
    assert!(page.data.iter().any(|a| !a.snippet.is_empty()
        && !a.thumbnail_url.is_empty()
        && !a.discord_message_id.is_empty()));
    assert!(page.data.iter().any(|a| a.snippet.is_empty()
        && a.thumbnail_url.is_empty()
        && a.discord_message_id.is_empty()));
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
/// The content manager reads it as `Paginated<Announcement>`; a draft round-trips without a
/// publication instant.
#[test]
fn cms_announcements_envelope() {
    const G: &str = golden!("GET__cms__announcements.json");
    assert_golden::<Paginated<Announcement>>(G, &[]);
    let page: Paginated<Announcement> = serde_json::from_str(G).unwrap();
    let draft = page
        .data
        .iter()
        .find(|a| a.status == "draft")
        .expect("the CMS capture holds a draft");
    assert!(draft.published_at.is_none() && !draft.pushed_to_discord);
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

/// One revision read by number: the seeded page is at its first revision, so the revision holds
/// the article's content and every block shape the article decodes, and it agrees with the
/// history's only row on its author and time.
#[test]
fn wiki_revision_read_by_number() {
    const G: &str = golden!("GET__wiki__field-manual__revisions__1.json");
    assert_golden::<WikiRevision>(G, &[]);
    let revision: WikiRevision = serde_json::from_str(G).unwrap();
    let article: WikiArticle =
        serde_json::from_str(golden!("GET__wiki__field-manual.json")).unwrap();
    assert_eq!(
        (revision.revision, &revision.title, &revision.body_md),
        (article.revision, &article.title, &article.body_md)
    );
    assert_eq!(revision.blocks, article.blocks);
    let history: WikiRevisionPage =
        serde_json::from_str(golden!("GET__wiki__field-manual__revisions.json")).unwrap();
    let listed = &history.items[0];
    assert_eq!(
        (&revision.author_id, &revision.created_at),
        (&listed.author_id, &listed.created_at)
    );
}

/// A replaced modpack answers the stored pack: its id and creation time stay, the new version
/// and the replaced (here empty) mod list come back.
#[test]
fn modpack_replacement() {
    const G: &str = golden!("PUT__modpacks__00000000-0000-4000-a000-000000000001.json");
    assert_golden::<ModpackDto>(G, &[]);
    let replaced: Value = serde_json::from_str(G).unwrap();
    let before: Value = serde_json::from_str(golden!("GET__modpacks__current.json")).unwrap();
    assert_eq!(
        (&replaced["id"], &replaced["created_at"]),
        (&before["id"], &before["created_at"])
    );
    assert_eq!(
        (&before["version"], &replaced["version"]),
        (&json!("2.1"), &json!("2.2"))
    );
    assert_eq!(replaced["mods"], json!([]));
}

/// A replaced vehicle answers the stored row with every field; a deleted one answers the row it
/// soft-deleted exactly as the list carried it, here a row that records no optional field.
#[test]
fn vehicle_replacement_and_deletion() {
    const REPLACED: &str =
        golden!("PUT__vehicle-database__00000000-0000-4000-3000-000000000003.json");
    const DELETED: &str =
        golden!("DELETE__vehicle-database__00000000-0000-4000-3000-000000000006.json");
    assert_golden::<Vehicle>(REPLACED, &[]);
    assert_golden::<Vehicle>(DELETED, &[]);
    let list: DataEnvelope<Vehicle> =
        serde_json::from_str(golden!("GET__vehicle-database.json")).unwrap();
    let replaced: Vehicle = serde_json::from_str(REPLACED).unwrap();
    let listed = list.data.iter().find(|v| v.id == replaced.id).unwrap();
    assert!(listed.profile_image_url.is_empty() && !replaced.profile_image_url.is_empty());
    assert_eq!(
        (&replaced.name, &replaced.faction),
        (&listed.name, &listed.faction)
    );
    let deleted: Vehicle = serde_json::from_str(DELETED).unwrap();
    assert_eq!(
        Some(&deleted),
        list.data.iter().find(|v| v.id == deleted.id)
    );
    assert!(deleted.amphibious.is_empty() && deleted.primary_threat.is_empty());
}

// ── content writes ──
// Each answer carries server-generated ids or request-time stamps; the goldens hold the fixed
// placeholders of the API's golden normalisation table at those fields.

/// A created pack answers with its mods, each tied to the new pack.
#[test]
fn modpack_created() {
    const G: &str = golden!("POST__modpacks.json");
    assert_golden::<ModpackDto>(G, &[]);
    let pack: ModpackDto = serde_json::from_str(G).unwrap();
    assert!(!pack.modpack.is_current);
    assert_eq!(pack.mods.len(), 1);
    assert_eq!(pack.mods[0].modpack_id, pack.modpack.id);
}

#[test]
fn vehicle_created() {
    assert_golden::<Vehicle>(golden!("POST__vehicle-database.json"), &[]);
}

/// A page saved without a base revision is created at revision 1, its body parsed into blocks.
#[test]
fn wiki_page_created() {
    const G: &str = golden!("PUT__wiki__night-operations.json");
    assert_golden::<WikiArticle>(G, &[]);
    let page: WikiArticle = serde_json::from_str(G).unwrap();
    assert_eq!((page.slug.as_str(), page.revision), ("night-operations", 1));
    assert!(!page.blocks.is_empty());
}
