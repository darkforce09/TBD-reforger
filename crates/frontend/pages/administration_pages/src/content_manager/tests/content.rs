//! Guards on the content manager: the category tags, the server ids, the announcement mapping and
//! the Markdown tools.

use super::{
    apply_md_tool, category_tag, date_ymd, doc_from_announcement, is_server_id, tag_category,
};
use frontend_api_dtos::Announcement;
use serde_json::json;

/// Every category the editor offers, standing orders included, must resolve to a stored tag.
#[test]
fn category_tag_covers_all_ui_categories_including_sop() {
    assert_eq!(category_tag("announcement"), Some("update"));
    assert_eq!(
        category_tag("sop"),
        Some("update"),
        "SOP has no enum variant — closest tag is update (perturbation: return None for sop)"
    );
    assert_eq!(category_tag("event"), Some("event"));
    assert_eq!(category_tag("modpack"), Some("modpack_update"));
    assert_eq!(category_tag("important"), Some("important"));
    assert_eq!(category_tag("nope"), None);
}

#[test]
fn server_id_detects_uuid_not_local_mock() {
    assert!(is_server_id("44fa4c17-5bd5-4c6b-b02d-4ccd52af6910"));
    assert!(!is_server_id("d1"));
    assert!(!is_server_id("new-1710000000000"));
    assert!(!is_server_id(""));
}

#[test]
fn tag_category_round_trips_live_tags() {
    assert_eq!(tag_category("update"), "announcement");
    assert_eq!(tag_category("event"), "event");
    assert_eq!(tag_category("modpack_update"), "modpack");
    assert_eq!(tag_category("important"), "important");
}

#[test]
fn doc_from_announcement_maps_status_and_dates() {
    let row: Announcement = serde_json::from_value(json!({
        "id": "44fa4c17-5bd5-4c6b-b02d-4ccd52af6910",
        "title": "Live row",
        "body": "hello",
        "tag": "modpack_update",
        "author_id": "000000000000000001",
        "status": "published",
        "is_pinned": false,
        "pushed_to_discord": false,
        "published_at": "2026-07-27T12:00:00Z",
        "created_at": "2026-07-01T00:00:00Z",
        "updated_at": "2026-07-27T12:00:00Z",
    }))
    .expect("the row decodes into Announcement");
    let doc = doc_from_announcement(&row).expect("row maps");
    assert_eq!(doc.id, "44fa4c17-5bd5-4c6b-b02d-4ccd52af6910");
    assert_eq!(doc.category, "modpack");
    assert!(doc.published);
    assert_eq!(doc.date, "2026-07-27");
    assert_eq!(date_ymd("2026-06-18T00:00:00Z"), "2026-06-18");
}

/// The captured CMS list maps row for row: a published post shows its publication day, and a
/// draft, which has no publication instant, shows the day it was last changed.
#[test]
fn the_captured_cms_list_maps_drafts_and_published_posts() {
    use frontend_api_dtos::Paginated;
    use frontend_test_support::fixtures::golden;
    let page: Paginated<Announcement> =
        serde_json::from_str(golden!("GET__cms__announcements.json")).expect("the golden decodes");
    let docs: Vec<_> = page.data.iter().filter_map(doc_from_announcement).collect();
    assert_eq!(docs.len(), page.data.len());
    let draft = docs
        .iter()
        .find(|d| !d.published)
        .expect("the capture holds a draft");
    assert_eq!(
        (
            draft.id.as_str(),
            draft.date.as_str(),
            draft.category.as_str()
        ),
        (
            "00000000-0000-4000-1000-000000000005",
            "2026-07-25",
            "event"
        )
    );
    let pinned = &docs[0];
    assert!(pinned.published);
    assert_eq!(
        (pinned.date.as_str(), pinned.category.as_str()),
        ("2026-07-22", "modpack")
    );
    assert_eq!(
        pinned.thumbnail_url,
        "https://cdn.tbd-reforger.example/news/modpack-21.jpg"
    );
}

#[test]
fn apply_md_tool_inserts_real_markers() {
    assert_eq!(apply_md_tool("", "Bold"), "**bold**");
    assert_eq!(apply_md_tool("hi", "Italic"), "hi *italic*");
    assert!(apply_md_tool("x", "Link").contains("](https://)"));
    assert!(apply_md_tool("x", "List").contains("- item"));
    assert!(apply_md_tool("", "Image").starts_with("![alt]"));
}
