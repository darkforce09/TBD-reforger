//! The review workspace's words, held against the captured workspace, and the wiring that makes the
//! editor it mounts read-only.

use super::banner::{findings_summary, review_workspace_title, NOTHING_IS_SAVED};
use super::page::workspace_failure_sentence;
use crate::v2::apps::editor::shell::review_mode::{self, ReviewedVersion};
use crate::v2::core::api::dto::ReviewWorkspace;
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
use crate::v2::core::test_support::fixtures::golden;

const WORKSPACE: &str = golden!(
    "GET__missions__00000000-0000-4000-c000-000000000004__artifacts__00000000-0000-4000-f000-000000000004__workspace.json"
);

/// The banner names the artifact by its short digest and the version it compiled from.
#[test]
fn the_banner_names_the_artifact_and_its_version() {
    let workspace: ReviewWorkspace = serde_json::from_str(WORKSPACE).unwrap();
    assert_eq!(
        review_workspace_title(
            &workspace.artifact.artifact_digest,
            &workspace.version.semver
        ),
        "Review workspace of artifact bcfb3b1c4109, version 0.4.0"
    );
    assert!(NOTHING_IS_SAVED.contains("nothing here is saved"));
    assert_eq!(findings_summary(0), "The compile reported no findings");
    assert_eq!(findings_summary(1), "The compile reported 1 finding");
    assert_eq!(findings_summary(3), "The compile reported 3 findings");
}

/// A refused read says who may open a workspace, or that there is nothing to open.
#[test]
fn a_refused_workspace_says_why() {
    assert!(workspace_failure_sentence(403, None).contains("author and administrators"));
    assert!(workspace_failure_sentence(404, Some("mission not found")).contains("no such artifact"));
    assert!(workspace_failure_sentence(401, None).contains("sign in again"));
    assert_eq!(
        workspace_failure_sentence(
            500,
            Some("the reviewed version no longer matches its artifact")
        ),
        "The reviewed version no longer matches its artifact"
    );
    assert_eq!(
        workspace_failure_sentence(0, None),
        "The review workspace could not be opened"
    );
}

/// The reviewed version the editor opens on is exactly the captured workspace's: its artifact, its
/// version and its authored payload, stamped with the row fields the artifact's compile read.
#[test]
fn the_editor_opens_on_the_reviewed_version() {
    let workspace: ReviewWorkspace = serde_json::from_str(WORKSPACE).unwrap();
    let reviewed = ReviewedVersion::from_workspace(&workspace);
    assert_eq!(reviewed.mission_id, "00000000-0000-4000-c000-000000000004");
    assert_eq!(
        reviewed.artifact_digest,
        "bcfb3b1c4109fe03ac0292372a3fdfb86052d44979c2f2638e7e419e7f234577"
    );
    assert_eq!(reviewed.semver, "0.4.0");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&reviewed.payload_json).unwrap(),
        workspace.version.json_payload
    );
    let row = reviewed.row_meta();
    assert_eq!(row.title, "Operation Cold Anvil");
    assert_eq!(row.terrain, "everon");
    assert_eq!(row.time_of_day, "03:15:00");
    assert_eq!(row.weather, "heavy_rain");
    assert!(row.briefing.is_empty(), "the blurb is not a compile input");
}

/// The route opens review mode before it mounts the editor and closes it when it goes away, so the
/// editor's boot always finds the reviewed version and the next Mission Creator authors again.
#[test]
fn the_route_opens_review_mode_around_the_editor() {
    let src = live_code(include_str!("../page.rs"));
    let inner = only_body(&src, "fn ReviewWorkspaceInner()");
    assert!(inner.contains("on_cleanup(review_mode::close)"));
    let open = inner
        .find("review_mode::open(ReviewedVersion::from_workspace(")
        .expect("the route opens review mode");
    let mount = inner
        .find("<MissionEditorPage")
        .expect("the route mounts the editor");
    assert!(
        open < mount,
        "review mode must be open before the editor mounts"
    );
    assert!(inner.contains("load_review_workspace(store, &mission_id, &artifact_id)"));
}

/// While a review is open no mount writes the mission; closing it restores authoring.
#[test]
fn review_mode_withholds_every_write_while_open() {
    let workspace: ReviewWorkspace = serde_json::from_str(WORKSPACE).unwrap();
    review_mode::close();
    assert!(review_mode::writes_mission());
    review_mode::open(ReviewedVersion::from_workspace(&workspace));
    assert!(review_mode::is_open());
    assert!(!review_mode::writes_mission());
    assert!(!crate::v2::apps::editor::shell::tab_lock::may_write());
    assert!(review_mode::saves_nothing_message().contains("version 0.4.0, saves nothing"));
    review_mode::close();
    assert!(review_mode::writes_mission());
    assert!(crate::v2::apps::editor::shell::tab_lock::may_write());
}
