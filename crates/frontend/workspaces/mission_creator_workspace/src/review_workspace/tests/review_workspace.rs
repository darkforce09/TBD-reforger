//! The review workspace opens the editor on exactly the reviewed version, and review mode withholds
//! every write of the mission while it is open.

use frontend_api_dtos::ReviewWorkspace;
use frontend_test_support::fixtures::golden;
use mission_creator_session::tab_lock::{self, SaveDecision, Stamp, TabRole};
use mission_creator_state::review_mode::{self, ReviewedVersion};

fn workspace() -> ReviewWorkspace {
    serde_json::from_str(golden!(
        "GET__missions__00000000-0000-4000-c000-000000000004__artifacts__00000000-0000-4000-f000-000000000004__workspace.json"
    ))
    .unwrap()
}

/// The reviewed version the editor opens on is exactly the captured workspace's: its artifact, its
/// version and its authored payload, stamped with the row fields the artifact's compile read.
#[test]
fn the_editor_opens_on_the_reviewed_version() {
    let workspace = workspace();
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

/// An elected writer tab whose own stamp is on the record writes its draft straight through; the
/// moment a review of the mission opens, the same tab writes nothing — the gate the draft writer
/// consults before it encodes refuses, so no draft record is written — and closing the review
/// restores the write.
#[test]
fn an_open_review_withholds_the_draft_write_an_elected_writer_would_make() {
    review_mode::close();
    let me = "tab-review-test";
    let own_record = Stamp {
        tab: me.to_string(),
        at: 1.0,
    };
    let draft_write_lands = || {
        tab_lock::may_write()
            && tab_lock::decide_save(tab_lock::role(), Some(&own_record), me) != SaveDecision::Defer
    };

    assert_eq!(tab_lock::role(), TabRole::Writer);
    assert!(
        draft_write_lands(),
        "an authoring writer tab saves its draft"
    );

    let reviewed = ReviewedVersion::from_workspace(&workspace());
    let mission_id = mission_model::ids::MissionId::new(reviewed.mission_id.as_str());
    review_mode::open(reviewed);
    assert!(
        review_mode::reviewed_for(&mission_id).is_some(),
        "the editor mount of the reviewed mission shows the reviewed version"
    );
    assert_eq!(
        tab_lock::role(),
        TabRole::Writer,
        "the election is unchanged"
    );
    assert!(!review_mode::writes_mission());
    assert!(
        !draft_write_lands(),
        "a review writes no draft record, whatever the writer election says"
    );

    review_mode::close();
    assert!(draft_write_lands(), "closing the review restores authoring");
}
