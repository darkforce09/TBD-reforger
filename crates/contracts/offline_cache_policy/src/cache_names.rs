//! Cache Storage names, derived from the build identifier the worker is registered with.
//!
//! **Role:** names the four caches the offline service worker owns and decides which existing
//! caches are stale when a new worker activates.
//! **Position:** the page registers `/service_worker.js?build=<id>` ([`BuildId::script_url`]);
//! the worker reads the query back ([`BuildId::from_script_query`]) and builds [`CacheNames`]
//! from it; the page builds the same names to write the offline pack.
//! **Signals & state:** none; pure functions.
//! **Invariants:** every owned cache name starts with [`CACHE_PREFIX`]; only the shell cache
//! carries the build identifier, so a new build replaces the app shell while the map assets,
//! catalogs and icon font survive; a cache without the prefix is never reported stale, so the
//! worker never deletes a cache it does not own.

/// The prefix of every cache the offline service worker owns.
pub const CACHE_PREFIX: &str = "tbd-offline-";

/// The generation of the data caches (map assets, catalogs, icon font); raising it discards them.
pub const DATA_CACHE_GENERATION: u32 = 1;

/// The path the page registers the service worker from; its scope is the whole origin.
pub const SERVICE_WORKER_SCRIPT_PATH: &str = "/service_worker.js";

/// The query key of the service worker script URL that carries the [`BuildId`].
pub const BUILD_QUERY_KEY: &str = "build";

/// The longest accepted build identifier, in bytes.
pub const BUILD_ID_MAX_LEN: usize = 64;

/// The identifier of one deployed build of the single-page app.
///
/// It holds 1 to [`BUILD_ID_MAX_LEN`] ASCII letters, digits, `-`, `_` or `.`, so it is safe in a
/// cache name and in a URL query without escaping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildId(String);

impl BuildId {
    /// The identifier a worker uses when its script URL carries no valid [`BUILD_QUERY_KEY`].
    pub const UNVERSIONED: &'static str = "unversioned";

    /// Accepts `raw` when it is a well-formed identifier, and `None` otherwise.
    pub fn parse(raw: &str) -> Option<Self> {
        let well_formed = !raw.is_empty()
            && raw.len() <= BUILD_ID_MAX_LEN
            && raw
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        well_formed.then(|| Self(raw.to_owned()))
    }

    /// The identifier in a script URL query such as `?build=abc123`; a missing or malformed
    /// value yields [`BuildId::UNVERSIONED`].
    pub fn from_script_query(search: &str) -> Self {
        search
            .trim_start_matches('?')
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == BUILD_QUERY_KEY)
            .and_then(|(_, value)| Self::parse(value))
            .unwrap_or_else(|| Self(Self::UNVERSIONED.to_owned()))
    }

    /// The identifier as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The URL the page registers the worker from: [`SERVICE_WORKER_SCRIPT_PATH`] with this
    /// identifier under [`BUILD_QUERY_KEY`].
    pub fn script_url(&self) -> String {
        format!("{SERVICE_WORKER_SCRIPT_PATH}?{BUILD_QUERY_KEY}={}", self.0)
    }
}

/// The names of the caches one build of the offline service worker owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheNames {
    /// The app shell: the page document, its script, module, stylesheet and web manifest.
    pub shell: String,
    /// Ballistics catalogs: every pinned catalog version and the last catalog list.
    pub catalogs: String,
    /// Terrain files under `/map-assets`: manifest, elevation model, satellite mosaic, map tiles.
    pub map_assets: String,
    /// The icon font's stylesheet and font files.
    pub icon_font: String,
}

impl CacheNames {
    /// The cache names of the build `build`.
    pub fn for_build(build: &BuildId) -> Self {
        Self {
            shell: format!("{CACHE_PREFIX}shell-{}", build.as_str()),
            catalogs: format!("{CACHE_PREFIX}catalogs-v{DATA_CACHE_GENERATION}"),
            map_assets: format!("{CACHE_PREFIX}map-assets-v{DATA_CACHE_GENERATION}"),
            icon_font: format!("{CACHE_PREFIX}icon-font-v{DATA_CACHE_GENERATION}"),
        }
    }

    /// Every name this build owns.
    pub fn all(&self) -> [&str; 4] {
        [
            self.shell.as_str(),
            self.catalogs.as_str(),
            self.map_assets.as_str(),
            self.icon_font.as_str(),
        ]
    }

    /// Whether the existing cache `name` belongs to the offline service worker but not to this
    /// build, so activation deletes it.
    pub fn is_stale(&self, name: &str) -> bool {
        name.starts_with(CACHE_PREFIX) && !self.all().contains(&name)
    }

    /// The names in `existing` that activation deletes, in their original order.
    pub fn stale_names<'a>(&self, existing: &'a [String]) -> Vec<&'a str> {
        existing
            .iter()
            .map(String::as_str)
            .filter(|name| self.is_stale(name))
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/cache_names.rs"]
mod tests;
