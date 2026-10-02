//! Tests for [`super`] — request classes, strategies, backing caches and cache keys.

use super::*;
use crate::cache_names::BuildId;

const ORIGIN: &str = "https://tbd.example";

fn class_of(method: &str, url: &str, is_navigation: bool) -> RequestClass {
    classify(&InterceptedRequest {
        method,
        url,
        is_navigation,
        worker_origin: ORIGIN,
    })
}

fn get(url: &str) -> RequestClass {
    class_of("GET", url, false)
}

#[test]
fn every_request_class_is_recognised_from_its_url() {
    for (url, is_navigation, expected) in [
        ("https://tbd.example/", true, RequestClass::ShellDocument),
        (
            "https://tbd.example/tools/mortar",
            true,
            RequestClass::ShellDocument,
        ),
        (
            "https://tbd.example/frontend-423f_bg.wasm",
            false,
            RequestClass::ShellAsset,
        ),
        (
            "https://tbd.example/manifest.webmanifest",
            false,
            RequestClass::ShellAsset,
        ),
        (
            "https://tbd.example/api/v1/ballistics-catalogs",
            false,
            RequestClass::CatalogList,
        ),
        (
            "https://tbd.example/api/v1/ballistics-catalogs?page=2",
            false,
            RequestClass::CatalogList,
        ),
        (
            "https://tbd.example/api/v1/ballistics-catalogs/vanilla-mortars/versions/3",
            false,
            RequestClass::CatalogVersion,
        ),
        (
            "https://tbd.example/map-assets/everon/manifest.json",
            false,
            RequestClass::MapAsset,
        ),
        (
            "https://tbd.example/map-assets/everon/satellite/everon-sat.tbd-sat",
            false,
            RequestClass::MapAsset,
        ),
        (
            "https://tbd.example/map-assets/everon/tiles/map/6/12/40.webp",
            false,
            RequestClass::MapAsset,
        ),
        (
            "https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined",
            false,
            RequestClass::IconFont,
        ),
        (
            "https://fonts.gstatic.com/s/materialsymbols/v1/font.woff2",
            false,
            RequestClass::IconFont,
        ),
    ] {
        assert_eq!(class_of("GET", url, is_navigation), expected, "{url}");
    }
}

#[test]
fn everything_the_worker_must_not_cache_passes_through() {
    use PassthroughReason as Reason;
    for (method, url, is_navigation, reason) in [
        (
            "POST",
            "https://tbd.example/api/v1/fire-missions",
            false,
            Reason::NotGet,
        ),
        (
            "PUT",
            "https://tbd.example/map-assets/everon/manifest.json",
            false,
            Reason::NotGet,
        ),
        ("GET", "not a url", false, Reason::UnparsableUrl),
        (
            "GET",
            "https://cdn.other.example/lib.js",
            false,
            Reason::CrossOrigin,
        ),
        ("GET", "http://tbd.example/", true, Reason::CrossOrigin),
        (
            "GET",
            "https://tbd.example:8443/",
            true,
            Reason::CrossOrigin,
        ),
        (
            "GET",
            "https://tbd.example/api/v1/auth/dev-login?role=admin",
            true,
            Reason::ApiRoute,
        ),
        (
            "GET",
            "https://tbd.example/api/v1/profile",
            false,
            Reason::ApiRoute,
        ),
        (
            "GET",
            "https://tbd.example/api/v1/ballistics-catalogs/x/versions",
            false,
            Reason::ApiRoute,
        ),
        (
            "GET",
            "https://tbd.example/api/v1/ballistics-catalogs/x/versions/1/extra",
            false,
            Reason::ApiRoute,
        ),
        (
            "GET",
            "https://tbd.example/api/v1/ballistics-catalogs//versions/1",
            false,
            Reason::ApiRoute,
        ),
        ("GET", "https://tbd.example/api", false, Reason::ApiRoute),
        (
            "GET",
            "https://tbd.example/map-assets/everon/tiles/satellite/3/1/2.webp",
            false,
            Reason::SatelliteTile,
        ),
        (
            "GET",
            "https://tbd.example/service_worker.js?build=abc",
            false,
            Reason::WorkerScript,
        ),
        (
            "GET",
            "https://tbd.example/offline_service_worker.js",
            false,
            Reason::WorkerScript,
        ),
        (
            "GET",
            "https://tbd.example/offline_service_worker_bg.wasm",
            false,
            Reason::WorkerScript,
        ),
    ] {
        assert_eq!(
            class_of(method, url, is_navigation),
            RequestClass::Passthrough(reason),
            "{method} {url}"
        );
    }
}

#[test]
fn each_class_has_its_strategy_and_cache() {
    let names = CacheNames::for_build(&BuildId::parse("b1").unwrap());
    for (class, strategy, cache) in [
        (
            RequestClass::ShellDocument,
            CacheStrategy::NetworkFirst,
            Some(names.shell.as_str()),
        ),
        (
            RequestClass::ShellAsset,
            CacheStrategy::CacheFirst,
            Some(names.shell.as_str()),
        ),
        (
            RequestClass::CatalogList,
            CacheStrategy::NetworkFirst,
            Some(names.catalogs.as_str()),
        ),
        (
            RequestClass::CatalogVersion,
            CacheStrategy::CacheFirst,
            Some(names.catalogs.as_str()),
        ),
        (
            RequestClass::MapAsset,
            CacheStrategy::CacheFirst,
            Some(names.map_assets.as_str()),
        ),
        (
            RequestClass::IconFont,
            CacheStrategy::CacheFirst,
            Some(names.icon_font.as_str()),
        ),
        (
            RequestClass::Passthrough(PassthroughReason::ApiRoute),
            CacheStrategy::Passthrough,
            None,
        ),
    ] {
        assert_eq!(class.strategy(), strategy, "{class:?}");
        assert_eq!(class.cache_name(&names), cache, "{class:?}");
        assert_eq!(
            class.slices_ranges(),
            class == RequestClass::MapAsset,
            "{class:?}"
        );
    }
}

#[test]
fn every_navigation_shares_the_site_root_key_and_other_keys_drop_only_the_fragment() {
    assert_eq!(
        cache_key(
            RequestClass::ShellDocument,
            "https://tbd.example/tools/mortar?grid=1#top"
        )
        .as_deref(),
        Some("https://tbd.example/")
    );
    assert_eq!(
        cache_key(
            RequestClass::CatalogList,
            "https://tbd.example/api/v1/ballistics-catalogs?page=2#x"
        )
        .as_deref(),
        Some("https://tbd.example/api/v1/ballistics-catalogs?page=2")
    );
    assert_eq!(
        cache_key(
            RequestClass::MapAsset,
            "https://tbd.example/map-assets/everon/manifest.json"
        )
        .as_deref(),
        Some("https://tbd.example/map-assets/everon/manifest.json")
    );
    assert_eq!(cache_key(RequestClass::MapAsset, "::"), None);
}

#[test]
fn classification_is_driven_by_the_request_and_not_by_the_navigation_flag_alone() {
    assert_eq!(
        get("https://tbd.example/tools/mortar"),
        RequestClass::ShellAsset
    );
    assert_eq!(
        class_of(
            "GET",
            "https://tbd.example/map-assets/everon/manifest.json",
            true
        ),
        RequestClass::MapAsset
    );
}
