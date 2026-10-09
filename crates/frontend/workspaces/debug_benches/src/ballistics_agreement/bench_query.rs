//! The ballistics agreement bench's URL parameters and its choice of catalog version.
//!
//! **Role:** parses `?seed=&count=&catalog=&version=` into a [`BenchQuery`] and picks the
//! catalog version the bench solves against from the public catalog list.
//! **Position:** the pure half of [`super`]; the browser half passes it
//! `window.location.search` and the decoded `GET /api/v1/ballistics-catalogs` answer.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a parameter that is present but malformed is an error, never a silent
//! default; `count` lies in `1..=`[`super::MAX_COUNT`]; a `version` without a `catalog` is an
//! error; the chosen version is always one the list names.

use frontend_api_dtos::ballistics_catalogs::BallisticsCatalogList;
use frontend_api_dtos::identifiers::BallisticsCatalogId;

use super::{DEFAULT_COUNT, DEFAULT_SEED, MAX_COUNT};
use crate::error::{Error, Result};

/// One run's parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchQuery {
    /// Seed of the case lattice (decimal in the URL).
    pub seed: u64,
    /// Number of cases, `1..=MAX_COUNT`.
    pub count: usize,
    /// Catalog to solve against; `None` takes the first catalog id of the list.
    pub catalog_id: Option<BallisticsCatalogId>,
    /// Version of that catalog; `None` takes its newest version.
    pub catalog_version: Option<u32>,
}

/// The catalog version a run solves against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogChoice {
    /// Lowercase slug naming the catalog across its versions.
    pub catalog_id: BallisticsCatalogId,
    /// Version number, one or more.
    pub catalog_version: u32,
}

impl CatalogChoice {
    /// API path of this version's catalog document.
    pub fn document_path(&self) -> String {
        format!(
            "/ballistics-catalogs/{}/versions/{}",
            self.catalog_id, self.catalog_version
        )
    }
}

/// Parses a location search string (with or without its leading `?`).
///
/// # Errors
///
/// A sentence naming the malformed parameter.
pub fn parse_bench_query(search: &str) -> Result<BenchQuery> {
    let mut query = BenchQuery {
        seed: DEFAULT_SEED,
        count: DEFAULT_COUNT,
        catalog_id: None,
        catalog_version: None,
    };
    for pair in search.trim_start_matches('?').split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        match name {
            "seed" => {
                query.seed = value.parse().map_err(|_| {
                    Error::MalformedBenchQuery(format!(
                        "seed `{value}` is not an unsigned 64-bit integer"
                    ))
                })?;
            }
            "count" => {
                let count: usize = value.parse().map_err(|_| {
                    Error::MalformedBenchQuery(format!("count `{value}` is not a whole number"))
                })?;
                if count == 0 || count > MAX_COUNT {
                    return Err(Error::MalformedBenchQuery(format!(
                        "count {count} lies outside 1..={MAX_COUNT}"
                    )));
                }
                query.count = count;
            }
            "catalog" => {
                if value.is_empty()
                    || !value.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-'
                    })
                {
                    return Err(Error::MalformedBenchQuery(format!(
                        "catalog `{value}` is not a catalog slug"
                    )));
                }
                query.catalog_id = Some(BallisticsCatalogId::from(value));
            }
            "version" => {
                let version: u32 = value
                    .parse()
                    .ok()
                    .filter(|version| *version >= 1)
                    .ok_or_else(|| {
                        Error::MalformedBenchQuery(format!(
                            "version `{value}` is not a version number"
                        ))
                    })?;
                query.catalog_version = Some(version);
            }
            _ => {}
        }
    }
    if query.catalog_version.is_some() && query.catalog_id.is_none() {
        return Err(Error::MalformedBenchQuery(
            "version is given without a catalog".to_string(),
        ));
    }
    Ok(query)
}

/// The catalog version to solve against: the named version, else the newest version of the
/// named catalog, else the newest version of the lowest catalog id in the list.
///
/// # Errors
///
/// A sentence when the list is empty or does not name the requested catalog or version.
pub fn choose_catalog_version(
    list: &BallisticsCatalogList,
    query: &BenchQuery,
) -> Result<CatalogChoice> {
    let catalog_id = match &query.catalog_id {
        Some(catalog_id) => catalog_id.clone(),
        None => list
            .data
            .iter()
            .map(|summary| summary.catalog_id.clone())
            .min()
            .ok_or_else(|| Error::CatalogNotListed("the catalog list is empty".to_string()))?,
    };
    let versions = list
        .data
        .iter()
        .filter(|summary| summary.catalog_id == catalog_id)
        .map(|summary| summary.catalog_version);
    let catalog_version = match query.catalog_version {
        Some(version) => versions
            .into_iter()
            .find(|listed| *listed == version)
            .ok_or_else(|| {
                Error::CatalogNotListed(format!(
                    "the catalog list does not name {catalog_id} v{version}"
                ))
            })?,
        None => versions.max().ok_or_else(|| {
            Error::CatalogNotListed(format!("the catalog list does not name {catalog_id}"))
        })?,
    };
    Ok(CatalogChoice {
        catalog_id,
        catalog_version,
    })
}
