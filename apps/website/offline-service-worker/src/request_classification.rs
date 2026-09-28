//! Request classes: how the offline service worker serves each intercepted request.
//!
//! **Role:** maps one intercepted request (method, absolute URL, navigation flag, worker origin)
//! to a [`RequestClass`], its [`CacheStrategy`], the cache that backs it and its cache key.
//! **Position:** the worker's fetch handler calls [`classify`] for every request and applies the
//! strategy; the page's offline pack writer uses [`cache_key`] so it stores entries under the
//! keys the worker reads.
//! **Signals & state:** none; pure functions.
//! **Invariants:** only `GET` requests are ever answered from a cache; every API path except the
//! two ballistics catalog routes passes through, so an authenticated response is never cached;
//! satellite XYZ tiles and the worker's own scripts pass through; navigations never reach
//! `/api/` or `/map-assets/` handling, so sign-in redirects stay on the network.

use url::Url;

use crate::cache_names::{CacheNames, SERVICE_WORKER_SCRIPT_PATH};

/// Origins of the icon font: the stylesheet host and the font file host.
pub const ICON_FONT_ORIGINS: [&str; 2] =
    ["https://fonts.googleapis.com", "https://fonts.gstatic.com"];

/// The file stem Trunk gives the worker's bindgen script and WebAssembly module at the site root.
pub const WORKER_BUNDLE_STEM: &str = "/offline_service_worker";

/// The ballistics catalog list route.
pub const CATALOG_LIST_PATH: &str = "/api/v1/ballistics-catalogs";

/// One intercepted request, as the worker sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterceptedRequest<'a> {
    /// The HTTP method, as the browser reports it (upper case).
    pub method: &'a str,
    /// The absolute request URL.
    pub url: &'a str,
    /// Whether the request is a navigation (`Request.mode === "navigate"`).
    pub is_navigation: bool,
    /// The worker's own origin, such as `https://tbd.example`.
    pub worker_origin: &'a str,
}

/// How a class of request is served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    /// Answer from the cache; on a miss, fetch, store a successful response and answer with it.
    CacheFirst,
    /// Fetch and store a successful response; when the network fails, answer from the cache.
    NetworkFirst,
    /// Fetch and answer with the network response; never read or write a cache.
    Passthrough,
}

/// Why a request passes through untouched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassthroughReason {
    /// The method is not `GET`.
    NotGet,
    /// The URL does not parse.
    UnparsableUrl,
    /// A cross-origin request that is not the icon font.
    CrossOrigin,
    /// An API route other than the two ballistics catalog routes.
    ApiRoute,
    /// A satellite XYZ tile, which the renderer never needs offline.
    SatelliteTile,
    /// The worker's loader, bindgen script or module.
    WorkerScript,
}

/// The class of one intercepted request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestClass {
    /// A navigation: the single-page app's document, stored under the site root.
    ShellDocument,
    /// A same-origin static file of the app shell (script, module, stylesheet, web manifest).
    ShellAsset,
    /// The ballistics catalog list, `GET /api/v1/ballistics-catalogs`.
    CatalogList,
    /// One immutable catalog version, `GET /api/v1/ballistics-catalogs/{id}/versions/{version}`.
    CatalogVersion,
    /// A terrain file under `/map-assets/`; a `Range` request is sliced from the cached body.
    MapAsset,
    /// The icon font's stylesheet or a font file.
    IconFont,
    /// Not handled; see the reason.
    Passthrough(PassthroughReason),
}

impl RequestClass {
    /// How requests of this class are served.
    pub fn strategy(self) -> CacheStrategy {
        match self {
            Self::ShellDocument | Self::CatalogList => CacheStrategy::NetworkFirst,
            Self::ShellAsset | Self::CatalogVersion | Self::MapAsset | Self::IconFont => {
                CacheStrategy::CacheFirst
            }
            Self::Passthrough(_) => CacheStrategy::Passthrough,
        }
    }

    /// The cache that backs this class, or `None` for a passthrough.
    pub fn cache_name(self, names: &CacheNames) -> Option<&str> {
        match self {
            Self::ShellDocument | Self::ShellAsset => Some(names.shell.as_str()),
            Self::CatalogList | Self::CatalogVersion => Some(names.catalogs.as_str()),
            Self::MapAsset => Some(names.map_assets.as_str()),
            Self::IconFont => Some(names.icon_font.as_str()),
            Self::Passthrough(_) => None,
        }
    }

    /// Whether a `Range` header on this class is answered by slicing the cached full body.
    pub fn slices_ranges(self) -> bool {
        self == Self::MapAsset
    }
}

/// The class of `request`.
pub fn classify(request: &InterceptedRequest<'_>) -> RequestClass {
    if request.method != "GET" {
        return RequestClass::Passthrough(PassthroughReason::NotGet);
    }
    let Ok(url) = Url::parse(request.url) else {
        return RequestClass::Passthrough(PassthroughReason::UnparsableUrl);
    };
    let origin = url.origin().ascii_serialization();
    if origin != request.worker_origin {
        return if ICON_FONT_ORIGINS.contains(&origin.as_str()) {
            RequestClass::IconFont
        } else {
            RequestClass::Passthrough(PassthroughReason::CrossOrigin)
        };
    }
    let path = url.path();
    if path == "/api" || path.starts_with("/api/") {
        return classify_api_path(path);
    }
    if path.starts_with("/map-assets/") {
        return classify_map_asset_path(path);
    }
    if path == SERVICE_WORKER_SCRIPT_PATH || path.starts_with(WORKER_BUNDLE_STEM) {
        return RequestClass::Passthrough(PassthroughReason::WorkerScript);
    }
    if request.is_navigation {
        RequestClass::ShellDocument
    } else {
        RequestClass::ShellAsset
    }
}

/// The key a response of `class` for `url` is stored under: the site root for the shell document
/// (every navigation of the single-page app answers with the same document), and the URL
/// without its fragment otherwise. `None` when the URL does not parse.
pub fn cache_key(class: RequestClass, url: &str) -> Option<String> {
    let mut parsed = Url::parse(url).ok()?;
    if class == RequestClass::ShellDocument {
        parsed.set_path("/");
        parsed.set_query(None);
    }
    parsed.set_fragment(None);
    Some(parsed.into())
}

fn classify_api_path(path: &str) -> RequestClass {
    if path == CATALOG_LIST_PATH {
        return RequestClass::CatalogList;
    }
    let segments: Vec<&str> = path.split('/').skip(1).collect();
    match segments.as_slice() {
        [
            "api",
            "v1",
            "ballistics-catalogs",
            catalog_id,
            "versions",
            version,
        ] if !catalog_id.is_empty() && !version.is_empty() => RequestClass::CatalogVersion,
        _ => RequestClass::Passthrough(PassthroughReason::ApiRoute),
    }
}

fn classify_map_asset_path(path: &str) -> RequestClass {
    let segments: Vec<&str> = path.split('/').skip(1).collect();
    match segments.as_slice() {
        ["map-assets", _terrain, "tiles", "satellite", ..] => {
            RequestClass::Passthrough(PassthroughReason::SatelliteTile)
        }
        _ => RequestClass::MapAsset,
    }
}

#[cfg(test)]
#[path = "tests/request_classification.rs"]
mod tests;
