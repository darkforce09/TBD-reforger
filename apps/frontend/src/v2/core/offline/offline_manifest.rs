//! The offline manifest: every file the offline pack downloads, with the cache and key it lands
//! under.
//!
//! **Role:** turns what the page can see — its own document's asset URLs, the ballistics catalog
//! list, the icon font stylesheet and the terrain pack — into [`OfflineTarget`]s, each classified
//! by the worker's own [`classify`] so it names the cache the worker reads it from.
//! **Position:** pure; [`super::offline_pack`] builds its download list here and writes each
//! target under [`OfflineTarget::key`] into the cache of [`OfflineTarget::class`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** a URL the worker would pass through (another API route, a satellite XYZ tile,
//! the worker's own scripts, a foreign origin other than the icon font) is never a target; keys
//! are absolute URLs without a fragment, the form the worker looks responses up by; targets are
//! unique by key and keep their first-seen order; a target is optional exactly when it is the
//! cross-origin icon font (its stylesheet or a font file), every other target is essential.
//! @contract ballistics-catalog.schema.json#/definitions/BallisticsCatalogList

use offline_cache_policy::offline_pack::OfflinePack;
use offline_cache_policy::request_classification::{
    cache_key, classify, InterceptedRequest, RequestClass, CATALOG_LIST_PATH,
};
use serde::Deserialize;
use url::Url;

/// The terrain whose pack the page downloads.
pub const OFFLINE_TERRAIN_ID: &str = "everon";

/// The served root of every terrain's files.
pub const MAP_ASSETS_ROOT: &str = "/map-assets";

/// The origin that serves the icon font's files.
pub const ICON_FONT_FILE_ORIGIN: &str = "https://fonts.gstatic.com";

/// One file of the offline pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineTarget {
    /// The absolute URL the file is fetched from and cached under.
    pub key: String,
    /// The worker's class of the URL, which names its cache.
    pub class: RequestClass,
    /// The size the pack list declares, when it declares one.
    pub expected_bytes: Option<u64>,
}

impl OfflineTarget {
    /// Whether the pack is usable without this file: only the cross-origin icon font's
    /// stylesheet and font files are optional, because the calculator works without them (its
    /// icons then show as their ligature text) and a browser may be unable to reach their host.
    pub fn is_optional(&self) -> bool {
        self.class == RequestClass::IconFont
    }
}

/// Why the catalog list cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogListUnreadable(pub String);

/// The target for `url` (absolute, or relative to `origin`), or `None` when the worker passes the
/// URL through or it does not parse.
pub fn target_for(origin: &str, url: &str, expected_bytes: Option<u64>) -> Option<OfflineTarget> {
    let absolute = Url::parse(origin).ok()?.join(url).ok()?;
    let request = InterceptedRequest {
        method: "GET",
        url: absolute.as_str(),
        is_navigation: false,
        worker_origin: origin,
    };
    let class = classify(&request);
    if matches!(class, RequestClass::Passthrough(_)) {
        return None;
    }
    Some(OfflineTarget {
        key: cache_key(class, absolute.as_str())?,
        class,
        expected_bytes,
    })
}

/// The shell files and icon font stylesheets among the document's asset URLs (`link[href]`,
/// `script[src]`); every other URL is dropped.
pub fn document_targets(origin: &str, asset_urls: &[String]) -> Vec<OfflineTarget> {
    unique(
        asset_urls
            .iter()
            .filter_map(|url| target_for(origin, url, None))
            .filter(|target| {
                matches!(
                    target.class,
                    RequestClass::ShellAsset | RequestClass::IconFont
                )
            })
            .collect(),
    )
}

/// The catalog list itself and every catalog version it names, from the body of
/// `GET /api/v1/ballistics-catalogs`.
pub fn catalog_targets(
    origin: &str,
    list_json: &str,
) -> Result<Vec<OfflineTarget>, CatalogListUnreadable> {
    let list: CatalogListProjection = serde_json::from_str(list_json)
        .map_err(|error| CatalogListUnreadable(error.to_string()))?;
    let mut targets: Vec<OfflineTarget> = catalog_list_target(origin).into_iter().collect();
    for summary in &list.data {
        match catalog_version_target(origin, &summary.catalog_id, summary.catalog_version) {
            Some(target) => targets.push(target),
            None => {
                return Err(CatalogListUnreadable(format!(
                    "catalog id {:?} does not form a catalog version path",
                    summary.catalog_id
                )))
            }
        }
    }
    Ok(unique(targets))
}

/// The catalog list's target: the one key both the pack writes the list under and the page
/// reads its saved copy from.
pub fn catalog_list_target(origin: &str) -> Option<OfflineTarget> {
    target_for(origin, CATALOG_LIST_PATH, None)
        .filter(|target| target.class == RequestClass::CatalogList)
}

/// One catalog version's target: the one key both the pack writes the document under and the
/// page reads its saved copy from; `None` when the id does not form a catalog version path.
pub fn catalog_version_target(
    origin: &str,
    catalog_id: &str,
    catalog_version: u32,
) -> Option<OfflineTarget> {
    let path = format!("{CATALOG_LIST_PATH}/{catalog_id}/versions/{catalog_version}");
    target_for(origin, &path, None).filter(|target| target.class == RequestClass::CatalogVersion)
}

/// The font files an icon font stylesheet names in its `url(...)` values, restricted to
/// [`ICON_FONT_FILE_ORIGIN`].
pub fn icon_font_file_targets(origin: &str, stylesheet: &str) -> Vec<OfflineTarget> {
    let mut targets = Vec::new();
    let mut rest = stylesheet;
    while let Some(start) = rest.find("url(") {
        rest = &rest[start + 4..];
        let Some(end) = rest.find(')') else {
            break;
        };
        let raw = rest[..end].trim().trim_matches(|c| c == '"' || c == '\'');
        rest = &rest[end + 1..];
        if raw.starts_with(ICON_FONT_FILE_ORIGIN) {
            targets.extend(target_for(origin, raw, None));
        }
    }
    unique(targets)
}

/// Every file of the terrain pack.
pub fn terrain_targets(origin: &str, pack: &OfflinePack) -> Vec<OfflineTarget> {
    unique(
        pack.entries
            .iter()
            .filter_map(|entry| target_for(origin, &entry.url, entry.expected_bytes))
            .collect(),
    )
}

/// Sum of the declared sizes of `targets`; undeclared sizes count zero.
pub fn declared_bytes(targets: &[OfflineTarget]) -> u64 {
    targets
        .iter()
        .filter_map(|target| target.expected_bytes)
        .sum()
}

fn unique(targets: Vec<OfflineTarget>) -> Vec<OfflineTarget> {
    let mut seen = std::collections::HashSet::new();
    targets
        .into_iter()
        .filter(|target| seen.insert(target.key.clone()))
        .collect()
}

/// The catalog list fields the pack reads; every other field is ignored.
#[derive(Deserialize)]
struct CatalogListProjection {
    data: Vec<CatalogSummaryProjection>,
}

#[derive(Deserialize)]
struct CatalogSummaryProjection {
    catalog_id: String,
    catalog_version: u32,
}

#[cfg(test)]
#[path = "tests/offline_manifest.rs"]
mod tests;
