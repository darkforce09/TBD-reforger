//! The roster's address: which page of which search the personnel screen shows.
//!
//! **Role:** reads `page`, `per_page` and `q` out of the page URL, writes them back in one
//! canonical order, builds the roster request path from them, and derives the next address when
//! the search, the page size or the page changes.
//! **Position:** between the router and the personnel screen: `page.rs` parses the URL query into
//! a [`RosterQuery`], keys the roster fetch on [`RosterQuery::api_path`], and navigates to
//! [`RosterQuery::to_url_query`] whenever a control asks for another address.
//! **Signals & state:** none; pure functions over plain values.
//! **Invariants:** a parsed query always holds a page of at least 1 and a page size from
//! [`PER_PAGE_OPTIONS`], so a hand-edited or junk URL value falls back to its default instead of
//! reaching the API as a 400. The search text is kept exactly as typed, so the search box never
//! loses a space mid-word; the request sends it trimmed and omits it when blank. A new search or
//! a new page size always starts again at the first page.

#[cfg(any(target_arch = "wasm32", test))]
use url::form_urlencoded::Serializer;

/// The page a roster address starts on, and falls back to.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const FIRST_PAGE: i64 = 1;

/// The page size a roster address starts on, and falls back to.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const DEFAULT_PER_PAGE: i64 = 20;

/// The page sizes the per-page control offers, as wire value and label.
///
/// These are also the only page sizes a URL may ask for: any other value falls back to
/// [`DEFAULT_PER_PAGE`], so the control always shows the size the table was fetched with.
#[cfg(any(target_arch = "wasm32", test))]
pub(super) const PER_PAGE_OPTIONS: &[(&str, &str)] =
    &[("10", "10"), ("20", "20"), ("50", "50"), ("100", "100")];

/// The roster request route the address is appended to.
#[cfg(any(target_arch = "wasm32", test))]
const ROSTER_API_PATH: &str = "/admin/users";

/// One roster address: the 1-based page, the page size and the search text as typed.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RosterQuery {
    /// The 1-based page asked for; at least [`FIRST_PAGE`].
    pub page: i64,
    /// Members per page; always one of [`PER_PAGE_OPTIONS`].
    pub per_page: i64,
    /// The search text exactly as typed, spaces included.
    pub q: String,
}

#[cfg(any(target_arch = "wasm32", test))]
impl Default for RosterQuery {
    fn default() -> Self {
        Self {
            page: FIRST_PAGE,
            per_page: DEFAULT_PER_PAGE,
            q: String::new(),
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
impl RosterQuery {
    /// Read an address from the raw URL values of `page`, `per_page` and `q`.
    ///
    /// A missing, non-numeric or out-of-range `page` or `per_page` takes its default; `q` is
    /// taken as written.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn from_url_values(
        page: Option<&str>,
        per_page: Option<&str>,
        q: Option<&str>,
    ) -> Self {
        Self {
            page: page.and_then(parse_page).unwrap_or(FIRST_PAGE),
            per_page: per_page
                .and_then(parse_per_page)
                .unwrap_or(DEFAULT_PER_PAGE),
            q: q.unwrap_or_default().to_string(),
        }
    }

    /// The URL query this address is written as: `?page=…&per_page=…`, then `&q=…` when the
    /// search text is not empty.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn to_url_query(&self) -> String {
        let mut query = Serializer::new(String::new());
        query.append_pair("page", &self.page.to_string());
        query.append_pair("per_page", &self.per_page.to_string());
        if !self.q.is_empty() {
            query.append_pair("q", &self.q);
        }
        format!("?{}", query.finish())
    }

    /// The roster request path for this address, relative to the API base: the page and page
    /// size always, and the trimmed search text only when it holds more than whitespace.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn api_path(&self) -> String {
        let mut query = Serializer::new(String::new());
        query.append_pair("page", &self.page.to_string());
        query.append_pair("per_page", &self.per_page.to_string());
        let search = self.q.trim();
        if !search.is_empty() {
            query.append_pair("q", search);
        }
        format!("{ROSTER_API_PATH}?{}", query.finish())
    }

    /// The address after the search text changes: the same page size, from the first page.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn with_search(&self, q: String) -> Self {
        Self {
            page: FIRST_PAGE,
            per_page: self.per_page,
            q,
        }
    }

    /// The address after the page size control picks `raw`: the same search, from the first
    /// page. A value the control does not offer takes the default size.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn with_per_page(&self, raw: &str) -> Self {
        Self {
            page: FIRST_PAGE,
            per_page: parse_per_page(raw).unwrap_or(DEFAULT_PER_PAGE),
            q: self.q.clone(),
        }
    }

    /// The address of another page of the same search and page size; a page below the first is
    /// read as the first.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(super) fn with_page(&self, page: i64) -> Self {
        Self {
            page: page.max(FIRST_PAGE),
            per_page: self.per_page,
            q: self.q.clone(),
        }
    }
}

/// A URL `page` value as a page number: decimal digits only, and at least [`FIRST_PAGE`].
#[cfg(any(target_arch = "wasm32", test))]
fn parse_page(raw: &str) -> Option<i64> {
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<i64>().ok().filter(|page| *page >= FIRST_PAGE)
}

/// A URL or control `per_page` value as a page size, when it is one of [`PER_PAGE_OPTIONS`].
#[cfg(any(target_arch = "wasm32", test))]
fn parse_per_page(raw: &str) -> Option<i64> {
    PER_PAGE_OPTIONS
        .iter()
        .find(|(value, _)| *value == raw)
        .and_then(|(value, _)| value.parse::<i64>().ok())
}

#[cfg(test)]
#[path = "tests/roster_query.rs"]
mod tests;
