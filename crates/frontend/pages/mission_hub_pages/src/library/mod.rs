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

mod card_grid;
mod dossier_body;
mod dossier_collaboration;
mod dossier_lifecycle;
pub mod dossier_sheet;
mod dossier_upload;
mod dossier_upload_panel;
mod dossier_versions;
mod featured_hero;
mod filter_bar;
mod header;
mod mission_diff;
pub mod page;
mod search_bar;

#[cfg(target_arch = "wasm32")]
pub use page::MissionLibraryPage;

// The two test batteries span every shard of this page, so the names they reach for through
// `use super::{…}` are gathered here.
#[cfg(test)]
use card_grid::{PLACEHOLDER_ART, card_is_bookmarked, mission_art_url};
#[cfg(test)]
use dossier_upload::{
    UPLOAD_MAX_BYTES, next_semver, oversize_refusal, parse_uploaded_document,
    unwrap_export_envelope, upload_failure,
};
#[cfg(test)]
use dossier_versions::{census_line, version_census};
#[cfg(test)]
use featured_hero::{FEATURED_BRIEFING_FALLBACK, featured_briefing_text};
#[cfg(test)]
use mission_diff::{
    CollectionDelta, DIFF_SAMPLE_CAP, MissionDiff, RowChange, classify_row, diff_mission_payloads,
};

#[cfg(test)]
#[path = "tests/mission_library.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/mission_library_versions.rs"]
mod versions;
