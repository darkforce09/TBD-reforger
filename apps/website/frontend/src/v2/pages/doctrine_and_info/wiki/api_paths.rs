//! The paths of the wiki's requests, relative to the API root, and the revision paging math.
//!
//! **Role:** builds the path of every call the wiki makes — the page list, one article, one page
//! of its revision history, one revision — and counts the pages of that history.
//! **Position:** read by the page's list fetch, the article pane, the revision list and view,
//! and the save submission.
//! **Signals & state:** none; pure functions over constants.
//! **Invariants:** a slug is written into a path as one percent-encoded segment, so no slug can
//! add a segment, a query or a fragment. The history is asked for [`REVISIONS_PER_PAGE`] entries
//! at a time, and a page number is at least 1.

/// `GET /wiki`: every page's summary.
pub(super) const PAGE_LIST_PATH: &str = "/wiki";

/// How many revisions one page of the history lists.
pub(super) const REVISIONS_PER_PAGE: i64 = 10;

/// `slug` as one path segment: the unreserved characters kept, every other byte
/// percent-encoded. A slug as the API writes it (`[a-z0-9-]`) comes back unchanged.
fn path_segment(slug: &str) -> String {
    let mut out = String::with_capacity(slug.len());
    for byte in slug.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `GET` and `PUT /wiki/{slug}`: one article, and its save.
pub(super) fn article_path(slug: &str) -> String {
    format!("/wiki/{}", path_segment(slug))
}

/// `GET /wiki/{slug}/revisions`: page `page` (at least 1) of the history, newest first.
pub(super) fn revision_list_path(slug: &str, page: i64) -> String {
    format!(
        "/wiki/{}/revisions?page={}&per_page={REVISIONS_PER_PAGE}",
        path_segment(slug),
        page.max(1)
    )
}

/// `GET /wiki/{slug}/revisions/{revision}`: one revision.
pub(super) fn revision_path(slug: &str, revision: i64) -> String {
    format!("/wiki/{}/revisions/{revision}", path_segment(slug))
}

/// How many pages a history of `total` entries fills at `per_page` a page; at least 1, so an
/// empty history still reads "page 1 of 1".
pub(super) fn revision_page_count(total: i64, per_page: i64) -> i64 {
    let per_page = per_page.max(1);
    ((total.max(0) + per_page - 1) / per_page).max(1)
}

#[cfg(test)]
#[path = "tests/api_paths.rs"]
mod tests;
