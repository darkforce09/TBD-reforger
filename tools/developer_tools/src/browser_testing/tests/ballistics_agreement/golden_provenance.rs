//! Unit tests for the proof that the served catalog goldens are the committed catalog.

use serde_json::{Value, json};

use super::*;
use crate::browser_testing::ballistics_agreement::COMMITTED_CATALOG;
use crate::repository_layout::compiled_checkout_root;

fn committed() -> (Vec<u8>, BallisticsCatalog) {
    let path = compiled_checkout_root()
        .expect("repository root")
        .join(COMMITTED_CATALOG);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "the committed catalog {} is unreadable: {error}",
            path.display()
        )
    });
    let catalog =
        BallisticsCatalog::from_json_slice(&bytes).expect("the committed catalog decodes");
    (bytes, catalog)
}

/// A fresh fixtures folder holding a list golden naming `sha256` and, when given, a document
/// golden with `document` as its body.
fn fixtures(
    name: &str,
    catalog: &BallisticsCatalog,
    sha256: &str,
    document: Option<&[u8]>,
) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ballistics-agreement-goldens-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a fixtures folder");
    let list = json!({
        "data": [{
            "catalog_id": catalog.catalog_id,
            "catalog_version": catalog.catalog_version,
            "title": catalog.title,
            "game_build": catalog.game_build,
            "export_generation_id": catalog.export_generation_id,
            "catalog_sha256": sha256,
            "uploaded_at": "2026-09-28T00:00:00Z",
        }]
    });
    std::fs::write(
        dir.join(LIST_GOLDEN),
        serde_json::to_vec_pretty(&list).unwrap(),
    )
    .unwrap();
    if let Some(document) = document {
        std::fs::write(
            dir.join(document_golden(
                catalog.catalog_id.as_str(),
                catalog.catalog_version,
            )),
            document,
        )
        .unwrap();
    }
    dir
}

/// The committed catalog as the API answers it: the same values, re-serialised.
fn reserialised(bytes: &[u8]) -> Vec<u8> {
    let value: Value = serde_json::from_slice(bytes).expect("JSON");
    serde_json::to_vec(&value).expect("serialises")
}

#[test]
fn the_document_golden_is_named_after_the_path_the_bench_reads() {
    assert_eq!(
        document_golden("vanilla_mortars", 1),
        "GET__ballistics-catalogs__vanilla_mortars__versions__1.json"
    );
}

#[test]
fn goldens_of_the_committed_catalog_are_served_byte_for_byte() {
    let (bytes, catalog) = committed();
    let document = reserialised(&bytes);
    assert_ne!(
        document, bytes,
        "the served document differs in bytes, not in values"
    );
    let dir = fixtures("match", &catalog, &sha256_hex(&bytes), Some(&document));
    let served = check_served_goldens(&dir, &bytes, &catalog).expect("the goldens are the catalog");
    assert_eq!(served.document_body, document);
    assert_eq!(
        served.list_body,
        std::fs::read(dir.join(LIST_GOLDEN)).unwrap()
    );
}

#[test]
fn a_list_naming_another_sha256_fails() {
    let (bytes, catalog) = committed();
    let dir = fixtures("sha", &catalog, &"0".repeat(64), Some(&bytes));
    let cause = check_served_goldens(&dir, &bytes, &catalog).expect_err("another hash");
    assert!(cause.contains("catalog_sha256"), "{cause}");
}

#[test]
fn a_document_of_another_catalog_fails() {
    let (bytes, catalog) = committed();
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    value["gravity_m_s2"] = json!(9.8);
    let dir = fixtures(
        "other",
        &catalog,
        &sha256_hex(&bytes),
        Some(&serde_json::to_vec(&value).unwrap()),
    );
    let cause = check_served_goldens(&dir, &bytes, &catalog).expect_err("another catalog");
    assert!(cause.contains("another catalog"), "{cause}");
}

#[test]
fn a_missing_golden_fails_with_its_cause() {
    let (bytes, catalog) = committed();
    let dir = fixtures("missing", &catalog, &sha256_hex(&bytes), None);
    let cause = check_served_goldens(&dir, &bytes, &catalog).expect_err("no document golden");
    assert!(cause.contains("unreadable"), "{cause}");
}

#[test]
fn a_list_without_the_version_fails() {
    let (bytes, mut catalog) = committed();
    let dir = fixtures("unlisted", &catalog, &sha256_hex(&bytes), Some(&bytes));
    catalog.catalog_version += 1;
    std::fs::copy(
        dir.join(document_golden(
            catalog.catalog_id.as_str(),
            catalog.catalog_version - 1,
        )),
        dir.join(document_golden(
            catalog.catalog_id.as_str(),
            catalog.catalog_version,
        )),
    )
    .unwrap();
    let cause = check_served_goldens(&dir, &bytes, &catalog).expect_err("not listed");
    assert!(cause.contains("does not list"), "{cause}");
}
