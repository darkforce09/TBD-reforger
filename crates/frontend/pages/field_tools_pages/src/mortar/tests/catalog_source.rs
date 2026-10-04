//! Tests for [`super`] — the saved-copy keys the page reads and the offline origin it words.

use super::*;
use frontend_offline::offline_manifest::{catalog_list_target, catalog_targets};
use offline_cache_policy::request_classification::RequestClass;

const ORIGIN: &str = "https://tbd.example";

const LIST_JSON: &str = r#"{"data":[
    {"catalog_id":"vanilla_mortars","catalog_version":1},
    {"catalog_id":"vanilla_mortars","catalog_version":2},
    {"catalog_id":"m252_extended","catalog_version":3}
]}"#;

#[test]
fn the_page_reads_every_catalog_saved_copy_under_the_key_the_offline_pack_writes() {
    let written: Vec<String> = catalog_targets(ORIGIN, LIST_JSON)
        .expect("a readable list")
        .into_iter()
        .map(|target| target.key)
        .collect();
    let list = catalog_list_target(ORIGIN).expect("the list target");
    assert_eq!(list.class, RequestClass::CatalogList);
    assert_eq!(written.first(), Some(&list.key));
    let listed = [
        ("vanilla_mortars", 1),
        ("vanilla_mortars", 2),
        ("m252_extended", 3),
    ];
    for (catalog_id, catalog_version) in listed {
        let key = CatalogKey {
            catalog_id: catalog_id.into(),
            catalog_version,
        };
        let read = key.saved_copy_target(ORIGIN).expect("a version target");
        assert_eq!(read.class, RequestClass::CatalogVersion);
        assert!(written.contains(&read.key), "{} is never written", read.key);
        assert_eq!(
            read.key,
            format!("{ORIGIN}/api/v1{}", key.document_path()),
            "the saved copy key is the URL the page reads"
        );
    }
    assert_eq!(written.len(), 1 + listed.len());
}

#[test]
fn a_catalog_id_that_leaves_the_catalog_routes_has_no_saved_copy() {
    let escaping = CatalogKey {
        catalog_id: "../fire-missions".into(),
        catalog_version: 1,
    };
    assert_eq!(escaping.saved_copy_target(ORIGIN), None);
}

#[test]
fn the_page_solves_from_the_offline_copy_when_the_list_or_the_document_came_from_it() {
    let dated = |date: &str| CatalogOrigin::OfflineCopy {
        saved_on: Some(date.into()),
    };
    let list_saved = dated("28 Sep 2026, 14:05 UTC");
    let document_saved = dated("27 Sep 2026, 09:00 UTC");
    assert_eq!(
        combined_origin(&CatalogOrigin::Network, CatalogOrigin::Network),
        CatalogOrigin::Network
    );
    assert_eq!(
        combined_origin(&list_saved, CatalogOrigin::Network),
        list_saved
    );
    assert_eq!(
        combined_origin(&CatalogOrigin::Network, document_saved.clone()),
        document_saved
    );
    assert_eq!(
        combined_origin(&list_saved, document_saved),
        list_saved,
        "the list's date wins: the list decides what is offered"
    );
}

#[test]
fn the_offline_notice_names_the_saved_copy_date_or_says_it_is_undated() {
    let dated = origin_notice(CatalogOrigin::OfflineCopy {
        saved_on: Some("28 Sep 2026, 14:05 UTC".into()),
    })
    .expect("a notice");
    assert!(
        dated.starts_with("Offline copy from 28 Sep 2026, 14:05 UTC: the server cannot be reached"),
        "{dated}"
    );
    assert!(
        dated.ends_with("the catalog saved on this device."),
        "{dated}"
    );
    let undated = origin_notice(CatalogOrigin::OfflineCopy { saved_on: None }).expect("a notice");
    assert!(undated.starts_with("Offline copy (undated):"), "{undated}");
    assert!(!dated.contains("No offline copy") && !undated.contains("No offline copy"));
}
