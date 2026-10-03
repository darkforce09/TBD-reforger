//! The browser download of the offline pack: fetches the pack list, checks the quota and stores
//! every missing file with every header it arrived with.
//!
//! **Role:** applies the policy of [`super::offline_pack`] in the browser: lists the pack
//! ([`super::offline_manifest`]), keeps the saved copy of a listing or file the server cannot
//! answer ([`super::saved_copies`]), fetches the missing files six at a time and reports the
//! [`FinishedDownload`].
//! **Position:** wasm32 only; [`super::offline_pack::ensure_offline_pack`] runs [`download`]
//! once per page lifetime and publishes its result; progress is published here while it runs.
//! **Signals & state:** writes the page-wide status through [`super::publish_status`] while
//! downloading; reads and writes Cache Storage.
//! **Invariants:** every file lands in the cache its worker class names, under the key the
//! worker reads; a response the worker answered from the saved copy (marked with
//! [`SAVED_COPY_HEADER`]) is never stored, so a cache entry is never marked; an essential file
//! whose refresh the server cannot answer counts as a failure only when its saved copy is missing
//! ([`FailedFiles::record_refresh_failure`]).

use futures::stream::{self, StreamExt};
use offline_cache_policy::cache_names::CacheNames;
use offline_cache_policy::network_fallback::{
    prefers_saved_copy, NetworkAnswer, SAVED_COPY_HEADER,
};
use offline_cache_policy::offline_pack::{manifest_url, terrain_pack, tile_index_url};
use offline_cache_policy::request_classification::RequestClass;
use offline_cache_policy::TerrainId;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Cache, Response};

use super::offline_manifest::{
    self as manifest, OfflineTarget, MAP_ASSETS_ROOT, OFFLINE_TERRAIN_ID,
};
use super::offline_pack::{final_outcome, FailedFiles, PackProgress, PARALLEL_DOWNLOADS};
use super::saved_copies::{self, ReadSource, TextRead};
use super::service_worker_registration::{
    current_build_id, document_asset_urls, offline_supported,
};
use super::storage_quota::{self, QuotaVerdict};
use super::{publish_status, OfflineState, OfflineStatus, OptionalFiles, PackRefresh};

/// How a download ends: the status, the optional-file coverage and the refresh outcome.
pub(super) struct FinishedDownload {
    /// The final state and progress.
    pub(super) status: OfflineStatus,
    /// [`OptionalFiles::Unknown`] when no file download ran.
    pub(super) optional_files: OptionalFiles,
    /// [`PackRefresh::Unknown`] when no file download ran.
    pub(super) refresh: PackRefresh,
}

fn status(state: OfflineState, progress_percent: u8) -> OfflineStatus {
    OfflineStatus {
        state,
        progress_percent,
    }
}

fn unfinished(state: OfflineState) -> FinishedDownload {
    FinishedDownload {
        status: status(state, 0),
        optional_files: OptionalFiles::Unknown,
        refresh: PackRefresh::Unknown,
    }
}

fn js_error(error: wasm_bindgen::JsValue) -> String {
    format!("{error:?}")
}

/// Runs the download and returns how it finished.
pub(super) async fn download() -> FinishedDownload {
    if !offline_supported() {
        return unfinished(OfflineState::Unsupported);
    }
    publish_status(status(OfflineState::Downloading, 0));
    match download_pack().await {
        Ok(finished) => finished,
        Err(reason) => {
            leptos::logging::warn!("offline pack download failed: {reason}");
            unfinished(OfflineState::Failed)
        }
    }
}

/// The caches of the running build, opened once; each class finds its cache through the
/// worker's own [`RequestClass::cache_name`].
struct BuildCaches {
    names: CacheNames,
    opened: Vec<(String, Cache)>,
}

impl BuildCaches {
    async fn open(names: CacheNames) -> Result<Self, String> {
        let storage = web_sys::window()
            .ok_or("no window")?
            .caches()
            .map_err(js_error)?;
        let mut opened = Vec::new();
        for name in names.all() {
            let cache = JsFuture::from(storage.open(name))
                .await
                .map_err(js_error)?
                .dyn_into::<Cache>()
                .map_err(js_error)?;
            opened.push((name.to_owned(), cache));
        }
        Ok(Self { names, opened })
    }

    fn for_class(&self, class: RequestClass) -> Option<&Cache> {
        let name = class.cache_name(&self.names)?;
        self.opened
            .iter()
            .find(|(opened_name, _)| opened_name == name)
            .map(|(_, cache)| cache)
    }
}

async fn fetch(url: &str) -> Result<Response, String> {
    let window = web_sys::window().ok_or("no window")?;
    JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|error| format!("{url}: {}", js_error(error)))?
        .dyn_into::<Response>()
        .map_err(js_error)
}

/// The body of the optional icon font stylesheet `url`.
async fn stylesheet_text(url: &str) -> Result<String, String> {
    let response = fetch(url).await?;
    if !response.ok() {
        return Err(format!("{url}: HTTP {}", response.status()));
    }
    let text = JsFuture::from(response.text().map_err(js_error)?)
        .await
        .map_err(js_error)?;
    text.as_string()
        .ok_or_else(|| format!("{url}: body is not text"))
}

/// One essential listing read, from the server or its saved copy (recorded in `failed`);
/// `None` on `404`.
async fn listing_text(
    origin: &str,
    url: &str,
    failed: &mut FailedFiles,
) -> Result<Option<String>, String> {
    let target = manifest::target_for(origin, url, None)
        .ok_or_else(|| format!("{url}: not a cache-backed file"))?;
    match saved_copies::read_text(&target)
        .await
        .map_err(|reason| format!("{url}: {reason}"))?
    {
        TextRead::NotFound => Ok(None),
        TextRead::Found { body, source } => {
            if source != ReadSource::Server {
                leptos::logging::warn!("offline pack listing {url}: kept the saved copy");
                failed.record_kept_saved_copy();
            }
            Ok(Some(body))
        }
    }
}

async fn required_listing(
    origin: &str,
    url: &str,
    failed: &mut FailedFiles,
) -> Result<String, String> {
    listing_text(origin, url, failed)
        .await?
        .ok_or_else(|| format!("{url}: HTTP 404"))
}

async fn is_cached(cache: &Cache, key: &str) -> Result<bool, String> {
    let found = JsFuture::from(cache.match_with_str(key))
        .await
        .map_err(js_error)?;
    Ok(!found.is_undefined() && !found.is_null())
}

/// Why a file was not stored, and whether the server could not answer (so a saved copy may
/// stand in).
struct StoreFailure {
    reason: String,
    server_unavailable: bool,
}

/// Fetches `target` and stores the response, headers included, under its key; a saved copy the
/// worker answered with is never stored.
async fn store(caches: &BuildCaches, target: &OfflineTarget) -> Result<(), StoreFailure> {
    let failure = |reason: String, server_unavailable: bool| StoreFailure {
        reason: format!("{}: {reason}", target.key),
        server_unavailable,
    };
    let cache = caches
        .for_class(target.class)
        .ok_or_else(|| failure("no cache".into(), false))?;
    let response = fetch(&target.key)
        .await
        .map_err(|reason| failure(reason, prefers_saved_copy(NetworkAnswer::Unreachable)))?;
    if !response.ok() {
        let answer = NetworkAnswer::Status(response.status());
        return Err(failure(
            format!("HTTP {}", response.status()),
            prefers_saved_copy(answer),
        ));
    }
    if response
        .headers()
        .get(SAVED_COPY_HEADER)
        .ok()
        .flatten()
        .is_some()
    {
        return Err(failure("answered from the saved copy".into(), true));
    }
    JsFuture::from(cache.put_with_str(&target.key, &response))
        .await
        .map_err(|error| failure(js_error(error), false))?;
    Ok(())
}

/// The pack list: every file and whether the list is complete.
struct PackListing {
    targets: Vec<OfflineTarget>,
    pack_complete: bool,
}

/// Every file of the pack; only an essential listing that is neither answered nor saved is an
/// error. Unreadable icon font stylesheets and kept saved copies are recorded in `failed`.
async fn pack_targets(origin: &str, failed: &mut FailedFiles) -> Result<PackListing, String> {
    let mut targets = manifest::document_targets(origin, &document_asset_urls());
    let stylesheets: Vec<String> = targets
        .iter()
        .filter(|target| target.class == RequestClass::IconFont)
        .map(|target| target.key.clone())
        .collect();
    for stylesheet in stylesheets {
        match stylesheet_text(&stylesheet).await {
            Ok(css) => targets.extend(manifest::icon_font_file_targets(origin, &css)),
            Err(reason) => {
                failed.record_unlisted_optional_files();
                leptos::logging::warn!("offline pack optional icon font unlisted: {reason}");
            }
        }
    }
    let list_target = manifest::catalog_list_target(origin).ok_or("no catalog list target")?;
    let catalog_list = required_listing(origin, &list_target.key, failed).await?;
    targets.extend(
        manifest::catalog_targets(origin, &catalog_list)
            .map_err(|unreadable| format!("catalog list: {}", unreadable.0))?,
    );
    let terrain = TerrainId::new(OFFLINE_TERRAIN_ID);
    let terrain_manifest =
        required_listing(origin, &manifest_url(MAP_ASSETS_ROOT, &terrain), failed).await?;
    let tile_index =
        listing_text(origin, &tile_index_url(MAP_ASSETS_ROOT, &terrain), failed).await?;
    let pack = terrain_pack(MAP_ASSETS_ROOT, &terrain_manifest, tile_index.as_deref())
        .map_err(|error| format!("terrain pack: {error:?}"))?;
    if !pack.is_complete() {
        leptos::logging::warn!("offline pack incomplete: {:?}", pack.completeness);
    }
    targets.extend(manifest::terrain_targets(origin, &pack));
    let mut seen = std::collections::HashSet::new();
    targets.retain(|target| seen.insert(target.key.clone()));
    Ok(PackListing {
        targets,
        pack_complete: pack.is_complete(),
    })
}

async fn download_pack() -> Result<FinishedDownload, String> {
    let origin = web_sys::window()
        .ok_or("no window")?
        .location()
        .origin()
        .map_err(js_error)?;
    let caches = BuildCaches::open(CacheNames::for_build(&current_build_id())).await?;
    let mut failed = FailedFiles::default();
    let listing = pack_targets(&origin, &mut failed).await?;

    // The catalog list is refreshed on every run; every other file is fetched only when
    // its cache does not hold it yet.
    let mut missing = Vec::new();
    for target in listing.targets {
        let cache = caches
            .for_class(target.class)
            .ok_or_else(|| format!("{}: no cache", target.key))?;
        if target.class == RequestClass::CatalogList || !is_cached(cache, &target.key).await? {
            missing.push(target);
        }
    }

    let needed = manifest::declared_bytes(&missing);
    if let QuotaVerdict::Short { needed, available } =
        storage_quota::quota_verdict(storage_quota::estimate().await, needed)
    {
        leptos::logging::warn!(
            "offline pack needs {needed} bytes, the origin has {available} free"
        );
        return Ok(unfinished(OfflineState::QuotaShort));
    }
    if storage_quota::request_persistence().await == Some(false) {
        leptos::logging::warn!("persistent storage not granted; the pack may be evicted");
    }

    let mut progress = PackProgress::new(missing.len(), needed);
    publish_status(status(OfflineState::Downloading, progress.percent()));
    let caches = &caches;
    let mut stored = stream::iter(missing.into_iter().map(|target| async move {
        let outcome = store(caches, &target).await;
        (target, outcome)
    }))
    .buffer_unordered(PARALLEL_DOWNLOADS);
    while let Some((target, outcome)) = stored.next().await {
        match outcome {
            Ok(()) => progress.record_stored(target.expected_bytes),
            Err(failure) => {
                let cached = match caches.for_class(target.class) {
                    Some(cache) => is_cached(cache, &target.key).await.unwrap_or(false),
                    None => false,
                };
                let before = failed;
                failed.record_refresh_failure(&target, failure.server_unavailable, cached);
                if failed.kept_saved_copies > before.kept_saved_copies {
                    progress.record_stored(target.expected_bytes);
                    leptos::logging::warn!("offline pack kept the saved copy: {}", failure.reason);
                } else {
                    let kind = if target.is_optional() {
                        "optional"
                    } else {
                        "essential"
                    };
                    leptos::logging::warn!("offline pack {kind} file failed: {}", failure.reason);
                }
            }
        }
        publish_status(status(OfflineState::Downloading, progress.percent()));
    }
    let outcome = final_outcome(listing.pack_complete, failed);
    let refresh = if outcome.kept_saved_copy {
        let saved_on = match manifest::catalog_list_target(&origin) {
            Some(target) => saved_copies::saved_copy_date(&target).await,
            None => None,
        };
        PackRefresh::KeptSavedCopy { saved_on }
    } else {
        PackRefresh::Refreshed
    };
    Ok(FinishedDownload {
        status: status(outcome.state, progress.percent()),
        optional_files: outcome.optional_files,
        refresh,
    })
}
