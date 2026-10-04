use super::*;

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
fn optional_files_attribute_values_are_absent_complete_and_missing() {
    assert_eq!(OFFLINE_OPTIONAL_ATTRIBUTE, "data-offline-optional");
    assert_eq!(OptionalFiles::Unknown.attribute_value(), None);
    assert_eq!(OptionalFiles::Complete.attribute_value(), Some("complete"));
    assert_eq!(OptionalFiles::Missing.attribute_value(), Some("missing"));
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
