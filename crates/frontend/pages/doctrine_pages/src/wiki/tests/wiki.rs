//! The wiki's route helpers: slug resolution, category order and the captured index.

use super::super::category_nav::category_order;
use super::resolve_slug;
use frontend_api_dtos::DataEnvelope;
use frontend_api_dtos::wiki::WikiPageSummary;
use frontend_test_support::fixtures::golden;

/// A summary with only the fields these tests read.
fn summary(slug: &str, category: &str, title: &str) -> WikiPageSummary {
    WikiPageSummary {
        slug: slug.into(),
        category: category.into(),
        title: title.into(),
        icon: String::new(),
        nav_order: 0,
        revision: 1,
        updated_at: "2026-07-14T10:12:00Z".into(),
    }
}

#[test]
fn slug_resolution_falls_back_to_first() {
    let pages = vec![
        summary("field-manual", "Doctrine", "Field Manual"),
        summary("radio-procedure", "Doctrine", "Radio"),
    ];
    assert_eq!(resolve_slug(&pages, None).as_deref(), Some("field-manual"));
    assert_eq!(
        resolve_slug(&pages, Some("radio-procedure")).as_deref(),
        Some("radio-procedure")
    );
    assert_eq!(
        resolve_slug(&pages, Some("nope")).as_deref(),
        Some("field-manual")
    );
    assert_eq!(resolve_slug(&[], Some("field-manual")), None);
}

#[test]
fn categories_preserve_first_seen_order() {
    let pages = vec![
        summary("a", "Doctrine", "A"),
        summary("b", "Administration", "B"),
        summary("c", "Doctrine", "C"),
        summary("d", "", "D"),
    ];
    assert_eq!(
        category_order(&pages),
        vec!["Doctrine".to_string(), "Administration".to_string()]
    );
}

#[test]
fn wiki_index_reads_the_captured_summaries_in_their_order() {
    let list: DataEnvelope<WikiPageSummary> =
        serde_json::from_str(golden!("GET__wiki.json")).unwrap();
    assert_eq!(
        resolve_slug(&list.data, None).as_deref(),
        Some("field-manual"),
        "the oracle's /wiki route opens the first manual"
    );
    assert_eq!(
        category_order(&list.data),
        vec!["Doctrine".to_string(), "Administration".to_string()]
    );
}
