//! Text handling shared across the crate: HTML sanitation with the preview helpers built on it,
//! and the content URL policy for the link targets and image sources of authored content. The
//! guard every stored link passes through is the `http_url_guard` crate.

pub mod content_url_policy;
pub mod html_sanitizer;
