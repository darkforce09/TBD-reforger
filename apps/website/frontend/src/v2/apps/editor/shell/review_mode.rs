//! Whether this editor mount is the read-only review workspace of one artifact, and what it shows.
//!
//! **Role:** holds the reviewed version the review workspace route opens the editor on — the
//! mission, the artifact and its digest, the version number, the authored payload and the row
//! fields the artifact compiled from — and answers the question every write path asks: may this
//! mount write the mission?
//! **Position:** opened by the review workspace route before the editor mounts and closed when the
//! route goes away; read by the boot, the draft writer's scheduling, the writer role, the unload
//! guard, the version save and the two mission-row mirrors.
//! **Signals & state:** one thread-local cell. The paths that read it run from timers, channel
//! callbacks and detached tasks with no reactive owner, which is why it is a cell rather than a
//! context — the reason the tab lock keeps its role in a cell too.
//! **Invariants:** while a review is open the editor writes nothing: no draft record, no
//! warm-session marker, no writer election, no version, no mission-row change. Edits stay in the
//! tab's memory and are discarded with it. The reviewed payload is serialised once, when the review
//! opens, and shared rather than copied.

use std::cell::RefCell;
use std::rc::Rc;

use crate::v2::core::api::dto::{ArtifactMetadata, ReviewWorkspace};
use website_map_engine::editing::persist::server_adoption::RowMeta;

/// The version a review workspace shows, and the artifact it compiled into.
#[derive(Clone)]
pub struct ReviewedVersion {
    /// The mission the version belongs to.
    pub mission_id: String,
    /// The artifact's identity digest, lowercase hex.
    pub artifact_digest: String,
    /// The version number the artifact compiled from.
    pub semver: String,
    /// The authored editor payload of that version, serialised once.
    pub payload_json: Rc<String>,
    /// The mission fields the artifact's compile read.
    pub metadata: ArtifactMetadata,
}

impl ReviewedVersion {
    /// The reviewed version a workspace answer describes.
    pub fn from_workspace(workspace: &ReviewWorkspace) -> Self {
        Self {
            mission_id: workspace.artifact.mission_id.clone(),
            artifact_digest: workspace.artifact.artifact_digest.clone(),
            semver: workspace.version.semver.clone(),
            payload_json: Rc::new(workspace.version.json_payload.to_string()),
            metadata: workspace.artifact.metadata.clone(),
        }
    }

    /// The row fields the document is stamped with: the ones the artifact's compile read, rather
    /// than whatever the mission row says today. The library blurb is not part of a compile, so it
    /// is left as the payload has it.
    pub fn row_meta(&self) -> RowMeta {
        RowMeta {
            title: self.metadata.title.clone(),
            terrain: self.metadata.terrain.clone(),
            time_of_day: self.metadata.time_of_day.clone(),
            weather: self.metadata.weather.clone(),
            briefing: String::new(),
        }
    }
}

thread_local! {
    /// The open review, when this tab's editor is a review workspace.
    static REVIEWED: RefCell<Option<ReviewedVersion>> = const { RefCell::new(None) };
}

/// Open a review: the next editor mount shows `reviewed` and writes nothing.
pub fn open(reviewed: ReviewedVersion) {
    REVIEWED.with(|cell| *cell.borrow_mut() = Some(reviewed));
}

/// Close the review, so the next editor mount authors again.
pub fn close() {
    REVIEWED.with(|cell| *cell.borrow_mut() = None);
}

/// The open review, if there is one.
#[must_use]
pub fn reviewed() -> Option<ReviewedVersion> {
    REVIEWED.with(|cell| cell.borrow().clone())
}

/// The open review of `mission_id`: the version an editor mount of that mission shows instead of
/// its draft. A review of any other mission is not this mount's to show.
#[must_use]
pub fn reviewed_for(mission_id: &str) -> Option<ReviewedVersion> {
    reviewed().filter(|reviewed| reviewed.mission_id == mission_id)
}

/// Whether this editor mount is a review workspace.
#[must_use]
pub fn is_open() -> bool {
    REVIEWED.with(|cell| cell.borrow().is_some())
}

/// Whether this editor mount may write the mission — its draft record, its row or a new version.
/// False for exactly as long as a review is open.
#[must_use]
pub fn writes_mission() -> bool {
    !is_open()
}

/// What a write attempted in the review workspace is told: which artifact and version it shows,
/// and where the mission is changed instead.
#[must_use]
pub fn saves_nothing_message() -> String {
    match reviewed() {
        Some(reviewed) => {
            format!(
            "The review workspace of artifact {}, version {}, saves nothing — open the mission in \
             the Mission Creator to change it.",
            reviewed.artifact_digest.chars().take(12).collect::<String>(),
            reviewed.semver
        )
        }
        None => "The review workspace saves nothing.".to_string(),
    }
}

#[cfg(test)]
#[path = "tests/review_mode/read_only_review.rs"]
mod tests;
