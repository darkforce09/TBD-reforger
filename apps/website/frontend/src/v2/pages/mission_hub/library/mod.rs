//! The mission library page: the catalogue, the dossier it slides over, and everything they use.
//!
//! **Role:** declares the route component, the header and its scope tabs, the search and filter
//! controls, the hero and the card grid, and the dossier sheet with its review, version, upload
//! and collaboration sections — plus the pure payload comparison the versions and upload panels
//! both read.
//! **Position:** the `/missions` route, in the mission hub.
//! **Signals & state:** none at this level; the page owns both fetches and every shared signal.
//! **Invariants:** one overlay at a time — the dossier sheet and the create dialog never stack.
//! The stored mission payload is moved, never cloned: it is the one value here that reaches
//! hundreds of megabytes.
#![allow(dead_code)]

mod card_grid;
mod dossier_body;
mod dossier_collaboration;
mod dossier_lifecycle;
mod dossier_sheet;
mod dossier_upload;
mod dossier_upload_panel;
mod dossier_versions;
mod featured_hero;
mod filter_bar;
mod header;
mod mission_diff;
mod page;
mod search_bar;

pub use page::MissionLibraryPage;

// The two test batteries span every shard of this page, so the names they reach for through
// `use super::{…}` are gathered here.
#[cfg(test)]
use card_grid::{
    author_avatar_img_src, bookmark_api_path, card_is_bookmarked, mission_art_url, PLACEHOLDER_ART,
};
#[cfg(test)]
use dossier_upload::{
    diff_summary_lines, next_semver, oversize_refusal, parse_uploaded_document,
    unwrap_export_envelope, upload_failure, UPLOAD_MAX_BYTES,
};
#[cfg(test)]
use dossier_versions::{census_line, version_census};
#[cfg(test)]
use featured_hero::{featured_briefing_text, FEATURED_BRIEFING_FALLBACK};
#[cfg(test)]
use mission_diff::{
    classify_row, diff_mission_payloads, CollectionDelta, MissionDiff, RowChange, DIFF_SAMPLE_CAP,
};

#[cfg(test)]
#[path = "tests/mission_library.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/mission_library_versions.rs"]
mod versions;
