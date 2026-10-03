//! Unit tests for the ballistics agreement bench's URL parameters and catalog-version choice.

use super::*;
use crate::foundation::transport::dto::ballistics_catalogs::BallisticsCatalogSummary;

fn summary(catalog_id: &str, catalog_version: u32) -> BallisticsCatalogSummary {
    BallisticsCatalogSummary {
        catalog_id: catalog_id.to_string(),
        catalog_version,
        title: format!("{catalog_id} v{catalog_version}"),
        game_build: "1.8.0.13".to_string(),
        export_generation_id: "6A6F008DC5395616".to_string(),
        catalog_sha256: "0".repeat(64),
        uploaded_at: "2026-09-28T00:00:00Z".to_string(),
    }
}

fn list(rows: &[(&str, u32)]) -> BallisticsCatalogList {
    BallisticsCatalogList {
        data: rows
            .iter()
            .map(|(catalog_id, version)| summary(catalog_id, *version))
            .collect(),
    }
}

fn query(search: &str) -> BenchQuery {
    parse_bench_query(search).expect("the search parses")
}

#[test]
fn an_empty_search_takes_the_defaults() {
    assert_eq!(
        query(""),
        BenchQuery {
            seed: DEFAULT_SEED,
            count: DEFAULT_COUNT,
            catalog_id: None,
            catalog_version: None,
        }
    );
}

#[test]
fn every_parameter_is_read_and_unknown_ones_are_ignored() {
    assert_eq!(
        query("?seed=18446744073709551615&count=512&catalog=vanilla_mortars&version=3&force=webgl"),
        BenchQuery {
            seed: u64::MAX,
            count: MAX_COUNT,
            catalog_id: Some("vanilla_mortars".to_string()),
            catalog_version: Some(3),
        }
    );
}

#[test]
fn a_malformed_parameter_is_an_error_never_a_default() {
    for search in [
        "?seed=-1",
        "?seed=0x10",
        "?seed=",
        "?count=0",
        "?count=513",
        "?count=many",
        "?catalog=Vanilla",
        "?catalog=",
        "?catalog=a/b",
        "?catalog=vanilla_mortars&version=0",
        "?catalog=vanilla_mortars&version=v1",
        "?version=1",
    ] {
        assert!(
            parse_bench_query(search).is_err(),
            "`{search}` must be refused"
        );
    }
}

#[test]
fn the_named_version_is_chosen_when_listed() {
    let choice = choose_catalog_version(
        &list(&[("vanilla_mortars", 1), ("vanilla_mortars", 2)]),
        &query("?catalog=vanilla_mortars&version=1"),
    )
    .expect("listed");
    assert_eq!(
        choice,
        CatalogChoice {
            catalog_id: "vanilla_mortars".to_string(),
            catalog_version: 1,
        }
    );
    assert_eq!(
        choice.document_path(),
        "/ballistics-catalogs/vanilla_mortars/versions/1"
    );
}

#[test]
fn a_named_catalog_without_a_version_takes_its_newest_version() {
    let choice = choose_catalog_version(
        &list(&[
            ("vanilla_mortars", 2),
            ("vanilla_mortars", 5),
            ("a_modded", 9),
        ]),
        &query("?catalog=vanilla_mortars"),
    )
    .expect("listed");
    assert_eq!(choice.catalog_version, 5);
}

#[test]
fn no_catalog_takes_the_newest_version_of_the_lowest_catalog_id() {
    let choice = choose_catalog_version(
        &list(&[("vanilla_mortars", 1), ("b_modded", 1), ("b_modded", 4)]),
        &query(""),
    )
    .expect("listed");
    assert_eq!(
        choice,
        CatalogChoice {
            catalog_id: "b_modded".to_string(),
            catalog_version: 4,
        }
    );
}

#[test]
fn an_empty_or_silent_list_is_an_error() {
    assert!(choose_catalog_version(&list(&[]), &query("")).is_err());
    assert!(
        choose_catalog_version(&list(&[("vanilla_mortars", 1)]), &query("?catalog=other")).is_err()
    );
    assert!(choose_catalog_version(
        &list(&[("vanilla_mortars", 1)]),
        &query("?catalog=vanilla_mortars&version=2")
    )
    .is_err());
}

#[test]
fn the_state_attribute_names_each_state() {
    use crate::workspaces::debug::ballistics_agreement::BenchState;
    assert_eq!(BenchState::Loading.as_attribute(), "loading");
    assert_eq!(BenchState::Ready.as_attribute(), "ready");
    assert_eq!(BenchState::Failed.as_attribute(), "failed");
}
