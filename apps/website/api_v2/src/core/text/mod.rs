//! Text handling shared across the crate: HTML sanitation with the preview helpers built on it,
//! the URL write-boundary guard every stored link passes through, and the content URL policy for
//! the link targets and image sources of authored content.

pub mod content_url_policy;
pub mod html_sanitizer;
pub mod http_url_guard;
