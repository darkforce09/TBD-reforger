//! The review workspace writes nothing: every write path of the editor consults review mode, and
//! the review boot arms none of the draft machinery.
//!
//! The write paths are browser-only, so what can be run natively is run — the predicate, the
//! writer role and the refusal sentence — and the wiring is pinned on the scrubbed source, where a
//! call parked in dead code does not count.

use super::*;
use crate::v2::core::api::dto::ReviewWorkspace;
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
use crate::v2::core::test_support::fixtures::golden;

fn reviewed() -> ReviewedVersion {
    let workspace: ReviewWorkspace = serde_json::from_str(golden!(
        "GET__missions__00000000-0000-4000-c000-000000000004__artifacts__00000000-0000-4000-f000-000000000004__workspace.json"
    ))
    .unwrap();
    ReviewedVersion::from_workspace(&workspace)
}

macro_rules! editor_source {
    ($path:literal) => {
        live_code(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/",
            $path
        )))
    };
}

/// Opening a review withholds every write and names the version; closing it restores authoring.
#[test]
fn an_open_review_withholds_writes_until_it_closes() {
    close();
    assert!(writes_mission() && !is_open() && reviewed_now().is_none());
    open(reviewed());
    assert!(is_open() && !writes_mission());
    assert_eq!(reviewed_now().map(|r| r.semver), Some("0.4.0".to_string()));
    assert!(
        !crate::v2::apps::editor::shell::tab_lock::may_write(),
        "the writer role reads as read-only while a review is open"
    );
    assert_eq!(
        saves_nothing_message(),
        "The review workspace of artifact bcfb3b1c4109, version 0.4.0, saves nothing — open the \
         mission in the Mission Creator to change it."
    );
    assert!(reviewed_for("00000000-0000-4000-c000-000000000004").is_some());
    assert!(
        reviewed_for("00000000-0000-4000-c000-000000000001").is_none(),
        "a review of one mission is not another mission's to show"
    );
    close();
    assert!(writes_mission());
    assert!(crate::v2::apps::editor::shell::tab_lock::may_write());
    assert_eq!(
        saves_nothing_message(),
        "The review workspace saves nothing."
    );
}

fn reviewed_now() -> Option<ReviewedVersion> {
    super::reviewed()
}

/// The writer role, the draft scheduling and the unload prompt all consult review mode.
#[test]
fn the_draft_writer_and_the_unload_prompt_consult_review_mode() {
    let tab_lock = editor_source!("shell/tab_lock.rs");
    assert!(only_body(&tab_lock, "pub fn may_write()").contains("review_mode::writes_mission()"));
    let history = editor_source!("bridge/document_host/history.rs");
    let edit = only_body(&history, "fn after_doc_change(");
    let guard = edit
        .find("review_mode::writes_mission()")
        .expect("the edit tail consults review mode");
    let arm = edit
        .find("schedule_edit_persist(")
        .expect("the edit tail arms the draft writer");
    assert!(
        guard < arm,
        "the draft writer is armed only past the review check"
    );
    assert!(only_body(&history, "pub fn register_unload_guard()")
        .contains("review_mode::writes_mission()"));
}

/// The version save refuses before it compiles or sends anything.
#[test]
fn the_version_save_refuses_in_a_review() {
    let saving = editor_source!("shell/document_commands/imp/mission_saving.rs");
    let save = only_body(&saving, "pub fn save_now(");
    let refuse = save
        .find("review_mode::writes_mission()")
        .expect("the save consults review mode");
    assert!(refuse < save.find("compile_payload(").expect("the save compiles"));
    assert!(save.contains("review_mode::saves_nothing_message()"));
}

/// Neither mission-row mirror writes the row from a review.
#[test]
fn the_row_mirrors_write_nothing_in_a_review() {
    let strip = editor_source!("ui/docks/top_strip/row_mirror.rs");
    let commit = only_body(&strip, "pub(super) fn commit(");
    assert!(
        commit
            .find("review_mode::writes_mission()")
            .expect("commit consults review mode")
            < commit.find(".arm(").expect("commit arms the mirror")
    );
    let settings = editor_source!("ui/modals/settings_modal/mission_row_mirror.rs");
    for writer in [
        "pub(super) fn set_game_mode(",
        "pub(super) fn set_presentation(",
    ] {
        let body = only_body(&settings, writer);
        assert!(
            body.find("review_mode::writes_mission()")
                .expect("the writer consults review mode")
                < body.find("api_patch").expect("the writer patches the row"),
            "{writer} must refuse before it patches"
        );
    }
    let load = only_body(&settings, "pub(super) fn load(");
    assert!(
        load.find("review_mode::writes_mission()")
            .expect("load consults review mode")
            < load.find("api_get").expect("load reads the row"),
        "a review shows the reviewed row, not today's"
    );
}

/// The boot restores a review from the reviewed version and arms none of the draft machinery.
#[test]
fn the_review_boot_restores_the_reviewed_version_and_arms_nothing() {
    let boot = editor_source!("mission_editor/canvas_mount/boot_tasks.rs");
    let start = only_body(&boot, "pub(super) fn start(");
    let review = start
        .find("review_mode::reviewed_for(&mission_id)")
        .expect("the boot asks for an open review of its own mission");
    assert!(
        review
            < start
                .find("review_restore::start(")
                .expect("the review restore")
    );
    assert!(start.contains("restore_authored_document("));
    let authored = only_body(&boot, "fn restore_authored_document(");
    assert!(authored.contains("yrs_persist::register_mission_persist("));
    assert!(authored.contains("yrs_persist::register_tab_sync("));
    let restore = editor_source!("mission_editor/canvas_mount/review_restore.rs");
    let body = only_body(&restore, "pub(super) fn start(");
    assert!(body.contains("adopt_payload("));
    assert!(body.contains("Adopt::Init"));
    assert!(body.contains("set_reviewed_row_meta("));
    for armed in [
        "yrs_persist",
        "register_tab_sync",
        "register_flush_on_hide",
        "mark_ready",
        "hydrate_from_server",
        "save_state_debounced",
        "load_state",
    ] {
        assert!(
            !restore.contains(armed),
            "the review restore must not reach `{armed}`"
        );
    }
}

/// The peer-tab banner stays silent in a review, which carries its own.
#[test]
fn the_peer_tab_banner_is_silent_in_a_review() {
    let tab_lock = editor_source!("shell/tab_lock.rs");
    let banner = only_body(&tab_lock, "pub fn TabLockBanner()");
    assert!(banner.contains("review_mode::writes_mission()"));
}
