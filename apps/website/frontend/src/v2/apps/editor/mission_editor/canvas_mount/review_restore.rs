//! Restores a review workspace's document: exactly the version its artifact compiled from.
//!
//! The reviewed payload replaces the fresh document as initialisation, so it is not an undo step
//! and the document opens clean; the row fields stamped over it are the ones the artifact's compile
//! read. Nothing is read from the local draft store and nothing is armed to write to it — no draft
//! writer, no flush on hide, no warm-session marker, no writer election — which is what makes the
//! workspace read-only while the editing tools still work on the tab's copy.

use super::*;
use crate::v2::apps::editor::shell::review_mode::ReviewedVersion;
use leptos::task::spawn_local;
use std::cell::Cell;
use std::rc::Rc;
use website_map_engine::editing::persist::server_adoption::{adopt_payload, Adopt};

/// What the review restore works with.
pub(super) struct ReviewRestore {
    pub doc: mission_doc::DocHandle,
    pub reviewed: ReviewedVersion,
    pub current_semver: RwSignal<Option<String>>,
    pub boot: RwSignal<BootPhase>,
    pub report: boot_progress::ProgressFn,
    pub restore_settled: Rc<Cell<bool>>,
    pub engine_mounted: Rc<Cell<bool>>,
    pub world_ready: Rc<Cell<bool>>,
}

/// Adopt the reviewed version, then join the engine's side of the boot handshake.
///
/// The adopt runs before the task's first suspension point, so it lands before the engine task
/// reads the document's terrain to boot the world assets.
pub(super) fn start(restore: ReviewRestore) {
    let ReviewRestore {
        doc,
        reviewed,
        current_semver,
        boot,
        report,
        restore_settled,
        engine_mounted,
        world_ready,
    } = restore;
    spawn_local(async move {
        adopt_payload(
            &doc,
            &reviewed.payload_json,
            &reviewed.row_meta(),
            Adopt::Init,
            &mission_history::after_local_edit,
        );
        mission_history::set_dirty(false);
        current_semver.set(Some(reviewed.semver.clone()));
        crate::v2::apps::editor::shell::document_commands::set_reviewed_row_meta(
            &reviewed.metadata,
        );
        validation_panel::clear_compile_findings();
        report(boot_progress::BootEvent::Finish(
            boot_progress::BootSeg::Mission,
        ));
        restore_settled.set(true);
        if engine_mounted.get() {
            mission_history::rebind_engine_from_doc();
        }
        if world_ready.get() {
            hand_over(boot);
        } else {
            boot.update(|b| *b = b.clone().advance(BootPhase::LoadingMap));
        }
    });
}
