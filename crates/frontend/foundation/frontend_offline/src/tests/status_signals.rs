use super::*;
use crate::pack_status::{OfflineState, OfflineStatus, OptionalFiles, PackRefresh};
use leptos::prelude::GetUntracked;

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
