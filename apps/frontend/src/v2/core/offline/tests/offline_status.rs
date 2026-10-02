use super::*;
use leptos::prelude::GetUntracked;

#[test]
fn offline_state_attribute_values_are_the_seven_documented_words() {
    let values: Vec<&str> = [
        OfflineState::Idle,
        OfflineState::Downloading,
        OfflineState::Ready,
        OfflineState::Incomplete,
        OfflineState::QuotaShort,
        OfflineState::Unsupported,
        OfflineState::Failed,
    ]
    .into_iter()
    .map(OfflineState::attribute_value)
    .collect();
    assert_eq!(
        values,
        [
            "idle",
            "downloading",
            "ready",
            "incomplete",
            "quota-short",
            "unsupported",
            "failed"
        ]
    );
    assert_eq!(OFFLINE_STATE_ATTRIBUTE, "data-offline-state");
    assert_eq!(OFFLINE_PROGRESS_ATTRIBUTE, "data-offline-progress");
}

#[test]
fn offline_status_starts_idle_and_follows_every_publish() {
    let status = offline_status();
    assert_eq!(status.get_untracked(), OfflineStatus::IDLE);
    let downloading = OfflineStatus {
        state: OfflineState::Downloading,
        progress_percent: 42,
    };
    publish_status(downloading);
    assert_eq!(status.get_untracked(), downloading);
    assert_eq!(offline_status().get_untracked(), downloading);
}

#[test]
fn optional_files_attribute_values_are_absent_complete_and_missing() {
    assert_eq!(OFFLINE_OPTIONAL_ATTRIBUTE, "data-offline-optional");
    assert_eq!(OptionalFiles::Unknown.attribute_value(), None);
    assert_eq!(OptionalFiles::Complete.attribute_value(), Some("complete"));
    assert_eq!(OptionalFiles::Missing.attribute_value(), Some("missing"));
}

#[test]
fn optional_files_start_unknown_and_follow_every_publish() {
    let optional = offline_optional_files();
    assert_eq!(optional.get_untracked(), OptionalFiles::Unknown);
    publish_optional_files(OptionalFiles::Missing);
    assert_eq!(optional.get_untracked(), OptionalFiles::Missing);
    publish_optional_files(OptionalFiles::Complete);
    assert_eq!(
        offline_optional_files().get_untracked(),
        OptionalFiles::Complete
    );
}

#[test]
fn pack_refresh_attribute_values_are_absent_refreshed_and_kept_saved_copy() {
    assert_eq!(OFFLINE_REFRESH_ATTRIBUTE, "data-offline-refresh");
    assert_eq!(PackRefresh::Unknown.attribute_value(), None);
    assert_eq!(PackRefresh::Refreshed.attribute_value(), Some("refreshed"));
    for saved_on in [None, Some("28 Sep 2026, 14:05 UTC".to_string())] {
        assert_eq!(
            PackRefresh::KeptSavedCopy { saved_on }.attribute_value(),
            Some("kept-saved-copy")
        );
    }
}

#[test]
fn pack_refresh_starts_unknown_and_follows_every_publish() {
    let refresh = offline_pack_refresh();
    assert_eq!(refresh.get_untracked(), PackRefresh::Unknown);
    let kept = PackRefresh::KeptSavedCopy {
        saved_on: Some("28 Sep 2026, 14:05 UTC".into()),
    };
    publish_pack_refresh(kept.clone());
    assert_eq!(refresh.get_untracked(), kept);
    publish_pack_refresh(PackRefresh::Refreshed);
    assert_eq!(
        offline_pack_refresh().get_untracked(),
        PackRefresh::Refreshed
    );
}
