//! The save and restore bodies: the draft's base revision, the stored fields, and a restore over
//! the current revision.

use super::*;
use frontend_test_support::fixtures::golden;
use serde_json::json;

/// The field manual as the server answers it.
fn field_manual() -> WikiArticle {
    serde_json::from_str(golden!("GET__wiki__field-manual.json")).unwrap()
}

#[test]
fn wiki_save_request_sends_the_draft_with_the_revision_it_started_from() {
    let mut article = field_manual();
    article.revision = 7;
    let draft = WikiDraft {
        body_md: "# TBD Field Manual\n\nRevised.".into(),
        base_revision: 5,
    };
    let request = draft_save_request(&article, Some(&draft));
    assert_eq!(
        request,
        WikiSaveRequest {
            category: article.category.clone(),
            title: article.title.clone(),
            icon: article.icon.clone(),
            nav_order: article.nav_order,
            body_md: "# TBD Field Manual\n\nRevised.".into(),
            base_revision: Some(5),
        }
    );
}

#[test]
fn wiki_save_request_without_a_draft_resends_the_stored_body_at_its_revision() {
    let article = field_manual();
    let request = draft_save_request(&article, None);
    assert_eq!(request.body_md, article.body_md);
    assert_eq!(request.base_revision, Some(article.revision));
}

#[test]
fn wiki_save_request_serialises_every_field_and_a_numeric_base() {
    let article = field_manual();
    let body = serde_json::to_value(draft_save_request(&article, None)).unwrap();
    assert_eq!(
        body,
        json!({
            "category": article.category,
            "title": article.title,
            "icon": article.icon,
            "nav_order": article.nav_order,
            "body_md": article.body_md,
            "base_revision": article.revision,
        })
    );
}

#[test]
fn wiki_restore_request_sends_the_old_revision_over_the_current_one() {
    let revision = WikiRevision {
        slug: "field-manual".into(),
        revision: 2,
        category: "Doctrine".into(),
        title: "Field Manual (2025)".into(),
        icon: String::new(),
        nav_order: 3,
        body_md: "# Field Manual\n\nThe old text.".into(),
        author_id: Some("000000000000000001".into()),
        created_at: "2026-05-01T09:00:00Z".into(),
        blocks: Vec::new(),
    };
    let request = restore_request(&revision, 9);
    assert_eq!(
        request,
        WikiSaveRequest {
            category: "Doctrine".into(),
            title: "Field Manual (2025)".into(),
            icon: String::new(),
            nav_order: 3,
            body_md: "# Field Manual\n\nThe old text.".into(),
            base_revision: Some(9),
        }
    );
    let body = serde_json::to_value(&request).unwrap();
    assert_eq!(body["base_revision"], json!(9));
    assert_eq!(
        body["icon"],
        json!(""),
        "an icon-less revision sends the empty icon"
    );
}
