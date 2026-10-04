//! The guards on the offline pack line.

use super::*;

const EVERY_STATE: [OfflineState; 7] = [
    OfflineState::Idle,
    OfflineState::Downloading,
    OfflineState::Ready,
    OfflineState::Incomplete,
    OfflineState::QuotaShort,
    OfflineState::Unsupported,
    OfflineState::Failed,
];

#[test]
fn every_offline_state_has_its_own_sentence() {
    let texts: std::collections::HashSet<String> = EVERY_STATE
        .into_iter()
        .map(|state| {
            offline_pack_text(
                OfflineStatus {
                    state,
                    progress_percent: 0,
                },
                OptionalFiles::Complete,
                &PackRefresh::Refreshed,
            )
        })
        .collect();
    assert_eq!(texts.len(), EVERY_STATE.len());
}

#[test]
fn the_download_sentence_carries_the_progress() {
    let text = offline_pack_text(
        OfflineStatus {
            state: OfflineState::Downloading,
            progress_percent: 42,
        },
        OptionalFiles::Unknown,
        &PackRefresh::Unknown,
    );
    assert!(text.ends_with("42%"), "{text}");
    assert!(
        offline_pack_text(
            OfflineStatus::IDLE,
            OptionalFiles::Unknown,
            &PackRefresh::Unknown
        )
        .contains("not started")
    );
}

#[test]
fn a_ready_pack_without_the_icon_font_says_icons_may_show_as_text() {
    let ready = OfflineStatus {
        state: OfflineState::Ready,
        progress_percent: 99,
    };
    let missing = offline_pack_text(ready, OptionalFiles::Missing, &PackRefresh::Refreshed);
    assert!(missing.starts_with("Offline copy ready"), "{missing}");
    assert!(missing.ends_with(ICON_FONT_MISSING_NOTICE), "{missing}");
    assert!(ICON_FONT_MISSING_NOTICE.contains("icon font is not cached"));
    for optional in [OptionalFiles::Complete, OptionalFiles::Unknown] {
        let text = offline_pack_text(ready, optional, &PackRefresh::Refreshed);
        assert!(!text.contains("icon font"), "{optional:?}: {text}");
    }
}

#[test]
fn a_ready_pack_that_kept_its_saved_copy_says_the_refresh_failed_with_the_copy_date() {
    let ready = OfflineStatus {
        state: OfflineState::Ready,
        progress_percent: 100,
    };
    let kept = PackRefresh::KeptSavedCopy {
        saved_on: Some("28 Sep 2026, 14:05 UTC".into()),
    };
    let text = offline_pack_text(ready, OptionalFiles::Complete, &kept);
    assert!(text.starts_with("Offline copy ready"), "{text}");
    assert!(
        text.ends_with("Refresh failed, using the saved copy from 28 Sep 2026, 14:05 UTC."),
        "{text}"
    );
    let with_font = offline_pack_text(ready, OptionalFiles::Missing, &kept);
    assert!(
        with_font.contains("saved copy from 28 Sep 2026"),
        "{with_font}"
    );
    assert!(with_font.ends_with(ICON_FONT_MISSING_NOTICE), "{with_font}");
    let undated = offline_pack_text(
        ready,
        OptionalFiles::Complete,
        &PackRefresh::KeptSavedCopy { saved_on: None },
    );
    assert!(
        undated.ends_with("using the saved copy (undated)."),
        "{undated}"
    );
    for refresh in [PackRefresh::Unknown, PackRefresh::Refreshed] {
        assert_eq!(refresh_notice(&refresh), None, "{refresh:?}");
        let text = offline_pack_text(ready, OptionalFiles::Complete, &refresh);
        assert!(!text.contains("Refresh failed"), "{refresh:?}: {text}");
    }
}

#[test]
fn a_failed_or_downloading_pack_never_claims_a_saved_copy() {
    let kept = PackRefresh::KeptSavedCopy {
        saved_on: Some("28 Sep 2026, 14:05 UTC".into()),
    };
    for state in [
        OfflineState::Idle,
        OfflineState::Downloading,
        OfflineState::QuotaShort,
        OfflineState::Unsupported,
        OfflineState::Failed,
    ] {
        let text = offline_pack_text(
            OfflineStatus {
                state,
                progress_percent: 10,
            },
            OptionalFiles::Complete,
            &kept,
        );
        assert!(!text.contains("saved copy"), "{state:?}: {text}");
    }
}
