//! Text handling shared across the crate: HTML sanitation with the preview helpers built on it,
//! and the URL write-boundary guard every stored link passes through.

pub mod html_sanitizer;
pub mod http_url_guard;
