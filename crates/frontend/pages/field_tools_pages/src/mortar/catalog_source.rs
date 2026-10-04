//! Where the mortar calculator's ballistics catalogs come from, and what the page says about it.
//!
//! **Role:** reads the public catalog list and one catalog version's document, picks the newest
//! version of each catalog, checks that the document answers the version asked for, and words
//! the source the page is solving from: the live API, the offline copy (with the date the server
//! gave it), or nothing — with the reason taken from the offline pack's state.
//! **Position:** fed by the public `GET /api/v1/ballistics-catalogs` and
//! `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}` routes through
//! `frontend_offline::saved_copies::read_text`: the offline service worker answers
//! them from its cache when the server cannot (the list network-first, a version cache-first),
//! and the page reads the saved copy itself when no worker answers; consumed by the page, which
//! hands the decoded [`BallisticsCatalog`] to the inputs and the solve bridge.
//! **Signals & state:** pure functions over plain values, plus two browser-only fetches and a
//! browser-only thread-local holding the [`CatalogOrigin`] of the latest list read, which the
//! document read that follows it inherits. The page reads
//! [`frontend_offline::offline_status`] and passes its state in.
//! **Invariants:** a catalog version is immutable, so a cached document is as good as a live
//! one, and the page is solving from the offline copy exactly when the list or the document came
//! from a saved copy; a saved copy is read under the key the offline pack stored it under
//! (`CatalogKey::saved_copy_target` and
//! [`frontend_offline::offline_manifest::catalog_list_target`]); a document whose
//! `catalog_id` or `catalog_version` differs from the one requested is refused, never solved
//! against; the reads never carry credentials, so the page works signed out.
//!
//! [`BallisticsCatalog`]: ballistics_model::catalog::BallisticsCatalog

#[cfg(any(target_arch = "wasm32", test))]
use ballistics_model::catalog::BallisticsCatalog;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_api_dtos::ballistics_catalogs::{BallisticsCatalogList, BallisticsCatalogSummary};
#[cfg(any(target_arch = "wasm32", test))]
use frontend_offline::OfflineState;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_offline::OfflineStatus;
#[cfg(any(target_arch = "wasm32", test))]
use frontend_offline::offline_manifest::{OfflineTarget, catalog_version_target};
#[cfg(any(target_arch = "wasm32", test))]
use frontend_offline::saved_copies::ReadSource;

/// API path of the public catalog list.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) const CATALOG_LIST_PATH: &str = "/ballistics-catalogs";

/// One catalog version: the identity a solution and a save pin.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CatalogKey {
    /// Lowercase slug naming the catalog across its versions.
    pub(crate) catalog_id: String,
    /// Version number, one or more.
    pub(crate) catalog_version: u32,
}

#[cfg(any(target_arch = "wasm32", test))]
impl CatalogKey {
    /// The key a stored summary names.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn of_summary(summary: &BallisticsCatalogSummary) -> Self {
        Self {
            catalog_id: summary.catalog_id.to_string(),
            catalog_version: summary.catalog_version,
        }
    }

    /// API path of this version's catalog document.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn document_path(&self) -> String {
        format!(
            "{CATALOG_LIST_PATH}/{}/versions/{}",
            self.catalog_id, self.catalog_version
        )
    }

    /// The offline target of this version's document on `origin`: the URL the page reads and
    /// the key the offline pack stores the document under.
    pub(crate) fn saved_copy_target(&self, origin: &str) -> Option<OfflineTarget> {
        catalog_version_target(
            origin,
            &self.catalog_id.as_str().into(),
            self.catalog_version,
        )
    }
}

/// The newest version of every catalog in `list`, ordered by catalog id.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn latest_catalog_versions(
    list: &BallisticsCatalogList,
) -> Vec<BallisticsCatalogSummary> {
    let mut latest: Vec<BallisticsCatalogSummary> = Vec::new();
    for summary in &list.data {
        match latest
            .iter_mut()
            .find(|kept| kept.catalog_id == summary.catalog_id)
        {
            Some(kept) if kept.catalog_version < summary.catalog_version => {
                *kept = summary.clone();
            }
            Some(_) => {}
            None => latest.push(summary.clone()),
        }
    }
    latest.sort_by(|a, b| a.catalog_id.cmp(&b.catalog_id));
    latest
}

/// The catalog to solve against: the current choice while the list still offers it, else the
/// first offered one; `None` when the list is empty.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn choose_catalog(
    offered: &[BallisticsCatalogSummary],
    current: Option<&CatalogKey>,
) -> Option<CatalogKey> {
    current
        .filter(|key| offered.iter().any(|s| CatalogKey::of_summary(s) == **key))
        .cloned()
        .or_else(|| offered.first().map(CatalogKey::of_summary))
}

/// Checks that a fetched document is the version that was requested.
///
/// # Errors
///
/// A sentence naming both identities when they differ.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn check_document_matches(
    requested: &CatalogKey,
    document: &BallisticsCatalog,
) -> Result<(), String> {
    if document.catalog_id == *requested.catalog_id
        && document.catalog_version == requested.catalog_version
    {
        Ok(())
    } else {
        Err(format!(
            "The server answered catalog {} v{} for {} v{}.",
            document.catalog_id,
            document.catalog_version,
            requested.catalog_id,
            requested.catalog_version
        ))
    }
}

/// Where a successfully read catalog came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogOrigin {
    /// The server answered.
    Network,
    /// The server could not be reached (unreachable, or a gateway or server failure) and the
    /// copy saved on this device answered; `saved_on` is the date the server gave that copy.
    OfflineCopy {
        /// The saved copy's date, worded for a person; `None` when it carries none.
        saved_on: Option<String>,
    },
}

/// The origin of one read from `source`.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn origin_of_read(source: ReadSource) -> CatalogOrigin {
    match source {
        ReadSource::Server => CatalogOrigin::Network,
        ReadSource::SavedCopy { saved_on } => CatalogOrigin::OfflineCopy { saved_on },
    }
}

/// The origin of a document read from `document` after a list read from `list`: the offline copy
/// when either came from it — the list's date first, because the list decides what is offered.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn combined_origin(list: &CatalogOrigin, document: CatalogOrigin) -> CatalogOrigin {
    match (list, document) {
        (CatalogOrigin::OfflineCopy { .. }, _) => list.clone(),
        (CatalogOrigin::Network, document) => document,
    }
}

/// The line shown over the inputs for a catalog that loaded; `None` for a live read.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn origin_notice(origin: CatalogOrigin) -> Option<String> {
    match origin {
        CatalogOrigin::Network => None,
        CatalogOrigin::OfflineCopy { saved_on } => {
            let copy = match saved_on {
                Some(date) => format!("Offline copy from {date}"),
                None => "Offline copy (undated)".to_string(),
            };
            Some(format!(
                "{copy}: the server cannot be reached, so the calculator solves with the \
                 catalog saved on this device."
            ))
        }
    }
}

/// Why no catalog is available to solve against.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CatalogFailure {
    /// The list could not be read (network and offline copy both failed).
    ListUnreadable(String),
    /// The list is empty: no catalog has been uploaded.
    NoCatalogs,
    /// The chosen version's document could not be read or was not the version requested.
    DocumentUnreadable(String),
}

/// The sentence the page shows for a catalog failure, with the offline pack's state explaining
/// whether an offline copy could have helped.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn failure_message(failure: &CatalogFailure, offline: OfflineStatus) -> String {
    let cause = match failure {
        CatalogFailure::NoCatalogs => {
            return "No ballistics catalog has been published yet; an administrator uploads one \
                    under Ballistics Catalogs."
                .to_string();
        }
        CatalogFailure::ListUnreadable(reason) => {
            format!("The ballistics catalogs could not be loaded ({reason}).")
        }
        CatalogFailure::DocumentUnreadable(reason) => {
            format!("The chosen ballistics catalog could not be loaded ({reason}).")
        }
    };
    format!("{cause} {}", offline_explanation(offline))
}

/// What the offline pack's state means for a catalog read that failed.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn offline_explanation(offline: OfflineStatus) -> String {
    match offline.state {
        OfflineState::Ready | OfflineState::Incomplete => {
            "The offline copy on this device holds no readable catalog either.".to_string()
        }
        OfflineState::Downloading => format!(
            "The offline copy is still downloading ({}%); try again once it is ready.",
            offline.progress_percent
        ),
        OfflineState::QuotaShort => {
            "This device lacks the free storage for an offline copy.".to_string()
        }
        OfflineState::Unsupported => "This browser cannot keep an offline copy.".to_string(),
        OfflineState::Idle | OfflineState::Failed => {
            "No offline copy is saved on this device yet; open this page once while online."
                .to_string()
        }
    }
}

/// The browser half: the reads through the offline core's saved copies.
#[cfg(target_arch = "wasm32")]
mod browser {
    use std::cell::RefCell;

    use frontend_offline::offline_manifest::{OfflineTarget, catalog_list_target};
    use frontend_offline::saved_copies::{TextRead, read_text};

    use super::CatalogOrigin;

    thread_local! {
        /// The origin of the latest list read; the document read that follows inherits it.
        pub(super) static LATEST_LIST_ORIGIN: RefCell<CatalogOrigin> =
            const { RefCell::new(CatalogOrigin::Network) };
    }

    /// The page's own origin, which prefixes every catalog address the offline core saves.
    pub(super) fn page_origin() -> Result<String, String> {
        web_sys::window()
            .ok_or("no window")?
            .location()
            .origin()
            .map_err(|_| "no page origin".to_string())
    }

    /// The catalog list's [`OfflineTarget`] on this page's origin, so the list read can fall
    /// back to its saved copy.
    pub(super) fn list_target() -> Result<OfflineTarget, String> {
        catalog_list_target(&page_origin()?).ok_or_else(|| "no catalog list address".to_string())
    }

    /// The body and origin of `target`, from the server or its saved copy.
    pub(super) async fn read_json(
        target: &OfflineTarget,
    ) -> Result<(String, CatalogOrigin), String> {
        match read_text(target).await.map_err(|error| error.to_string())? {
            TextRead::NotFound => Err("Request failed (404)".to_string()),
            TextRead::Found { body, source } => Ok((body, super::origin_of_read(source))),
        }
    }
}

/// Reads the public catalog list, from the server or the copy saved on this device when the
/// server cannot be reached.
///
/// # Errors
///
/// The request's failure message when neither answers, or the body's decoding error.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn fetch_catalog_list() -> Result<BallisticsCatalogList, String> {
    let (body, origin) = browser::read_json(&browser::list_target()?).await?;
    let list = serde_json::from_str(&body).map_err(|error| error.to_string())?;
    browser::LATEST_LIST_ORIGIN.with(|latest| *latest.borrow_mut() = origin);
    Ok(list)
}

/// Reads one catalog version's document and checks that it is the version requested; the
/// second value is the [`CatalogOrigin`] the page is solving from ([`combined_origin`] of the
/// latest list read and this read).
///
/// # Errors
///
/// The request's failure message, the body's decoding error, or the mismatch from
/// [`check_document_matches`].
#[cfg(target_arch = "wasm32")]
pub(crate) async fn fetch_catalog_document(
    key: &CatalogKey,
) -> Result<(BallisticsCatalog, CatalogOrigin), String> {
    let target = key
        .saved_copy_target(&browser::page_origin()?)
        .ok_or_else(|| format!("{} is not a catalog address", key.document_path()))?;
    let (body, read_from) = browser::read_json(&target).await?;
    let document: BallisticsCatalog =
        serde_json::from_str(&body).map_err(|error| error.to_string())?;
    check_document_matches(key, &document)?;
    let list = browser::LATEST_LIST_ORIGIN.with(|latest| latest.borrow().clone());
    Ok((document, combined_origin(&list, read_from)))
}

#[cfg(test)]
#[path = "tests/catalog_source.rs"]
mod tests;
