//! The doctrine wiki through the real HTTP router: the optimistic `base_revision` rules decide
//! create, update or conflict, and members read pages while only administrators save them.
//!
//! Each case owns its administrator, its member and its slugs, so every row it counts is its
//! own.

use crate::wiki_support;

use axum::http::StatusCode;
use serde_json::json;

use wiki_support::{WikiSuite, assert_refusal, page_uri, revisions_uri, save_body, unique_slug};

const SUITE: &str = "wiki_features";

#[tokio::test]
async fn wiki_features_base_revision_decides_create_update_or_conflict() {
    let suite = WikiSuite::new(SUITE).await;
    let slug = unique_slug("base");

    // A numeric base names a revision of a page that does not exist.
    let (status, body) = suite.save(&slug, save_body("Base", "body", Some(1))).await;
    assert_refusal(status, &body, StatusCode::NOT_FOUND, None);
    assert!(suite.page_row(&slug).await.is_none());

    // null on a new page creates it.
    let created = suite.create(&slug, "Base", "first body").await;
    assert_eq!(created["revision"], 1);

    // null on an existing page is a conflict with its current revision.
    let (status, body) = suite.save(&slug, save_body("Base", "again", None)).await;
    assert_refusal(
        status,
        &body,
        StatusCode::CONFLICT,
        Some("wiki_revision_conflict"),
    );
    assert_eq!(body["details"]["current_revision"], 1);

    let saved = suite.update(&slug, 1, "Base", "second body").await;
    assert_eq!(saved["revision"], 2);
    let before = suite.stored(&slug).await;

    // A stale base and a base from the future both name the current revision.
    for base in [1, 3] {
        let (status, body) = suite
            .save(&slug, save_body("Overwrite", "lost edit", Some(base)))
            .await;
        assert_refusal(
            status,
            &body,
            StatusCode::CONFLICT,
            Some("wiki_revision_conflict"),
        );
        assert_eq!(body["details"]["current_revision"], 2, "base {base}");
    }
    for base in [0, -1] {
        let (status, body) = suite
            .save(&slug, save_body("Overwrite", "lost edit", Some(base)))
            .await;
        assert_refusal(status, &body, StatusCode::BAD_REQUEST, None);
    }
    assert_eq!(suite.stored(&slug).await, before, "refusals change nothing");
    let (_, article) = suite.read(&page_uri(&slug)).await;
    assert_eq!(
        (&article["revision"], &article["body_md"]),
        (&json!(2), &json!("second body"))
    );
}

#[tokio::test]
async fn wiki_features_members_read_but_only_administrators_save() {
    let suite = WikiSuite::new(SUITE).await;
    let slug = unique_slug("tiers");
    suite.create(&slug, "Tiers", "admin body").await;
    let before = suite.stored(&slug).await;

    for uri in [
        "/api/v1/wiki".to_owned(),
        page_uri(&slug),
        revisions_uri(&slug),
        format!("{}/1", revisions_uri(&slug)),
    ] {
        let (status, body) = suite.read(&uri).await;
        assert_eq!(status, StatusCode::OK, "member reads {uri}: {body}");
        let (status, body) = suite.call(None, "GET", &uri, None).await;
        assert_refusal(status, &body, StatusCode::UNAUTHORIZED, None);
    }

    let fresh = unique_slug("tiers-fresh");
    for (target, base) in [(&slug, Some(1)), (&fresh, None)] {
        let body = save_body("Member edit", "member body", base);
        let (status, answer) = suite.save_as(&suite.member, target, body.clone()).await;
        assert_refusal(status, &answer, StatusCode::FORBIDDEN, None);
        let (status, answer) = suite.call(None, "PUT", &page_uri(target), Some(body)).await;
        assert_refusal(status, &answer, StatusCode::UNAUTHORIZED, None);
    }
    assert_eq!(suite.stored(&slug).await, before);
    assert!(suite.page_row(&fresh).await.is_none());
    assert_eq!(suite.wiki_audit_count_by(&suite.member).await, 0);
}
