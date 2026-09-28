//! Tests for [`super`] — build identifiers, cache names and stale-cache selection.

use super::*;

#[test]
fn a_well_formed_build_id_is_accepted_and_anything_else_is_refused() {
    for accepted in [
        "a",
        "423f29d64f4d9707",
        "v1.2.3_rc-1",
        &"x".repeat(BUILD_ID_MAX_LEN),
    ] {
        assert_eq!(
            BuildId::parse(accepted).map(|id| id.as_str().to_owned()),
            Some(accepted.to_owned())
        );
    }
    for refused in [
        "",
        "has space",
        "slash/",
        "percent%20",
        "quote\"",
        "émoji",
        &"x".repeat(BUILD_ID_MAX_LEN + 1),
    ] {
        assert_eq!(BuildId::parse(refused), None, "{refused:?}");
    }
}

#[test]
fn the_script_query_carries_the_build_id_and_a_missing_or_bad_one_is_unversioned() {
    assert_eq!(
        BuildId::from_script_query("?build=abc123").as_str(),
        "abc123"
    );
    assert_eq!(
        BuildId::from_script_query("?other=1&build=abc123&x=y").as_str(),
        "abc123"
    );
    for search in [
        "",
        "?",
        "?other=1",
        "?build=",
        "?build=bad/id",
        "?builder=abc",
    ] {
        assert_eq!(
            BuildId::from_script_query(search).as_str(),
            BuildId::UNVERSIONED,
            "{search:?}"
        );
    }
}

#[test]
fn the_script_url_round_trips_through_the_query_parser() {
    let build = BuildId::parse("423f29d64f4d9707").unwrap();
    let url = build.script_url();
    assert_eq!(url, "/service_worker.js?build=423f29d64f4d9707");
    let search = &url[url.find('?').unwrap()..];
    assert_eq!(BuildId::from_script_query(search), build);
}

#[test]
fn only_the_shell_cache_carries_the_build_id() {
    let first = CacheNames::for_build(&BuildId::parse("one").unwrap());
    let second = CacheNames::for_build(&BuildId::parse("two").unwrap());
    assert_eq!(first.shell, "tbd-offline-shell-one");
    assert_ne!(first.shell, second.shell);
    assert_eq!(first.catalogs, second.catalogs);
    assert_eq!(first.map_assets, second.map_assets);
    assert_eq!(first.icon_font, second.icon_font);
    assert_eq!(first.map_assets, "tbd-offline-map-assets-v1");
    for name in first.all() {
        assert!(name.starts_with(CACHE_PREFIX), "{name}");
    }
}

#[test]
fn activation_deletes_the_previous_shell_and_never_a_foreign_or_current_cache() {
    let names = CacheNames::for_build(&BuildId::parse("two").unwrap());
    let existing: Vec<String> = [
        "tbd-offline-shell-one",
        "tbd-offline-shell-two",
        "tbd-offline-map-assets-v1",
        "tbd-offline-map-assets-v0",
        "tbd-offline-catalogs-v1",
        "tbd-offline-icon-font-v1",
        "someone-elses-cache",
        "workbox-precache",
    ]
    .iter()
    .map(|name| (*name).to_owned())
    .collect();
    assert_eq!(
        names.stale_names(&existing),
        vec!["tbd-offline-shell-one", "tbd-offline-map-assets-v0"]
    );
}
