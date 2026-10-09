//! The live status: the badge for every stream and history state, the history fallback, and the
//! load-state transitions.

use super::*;

/// Every stream state the badge can be asked about.
const STREAMS: [AuditStreamState; 5] = [
    AuditStreamState::Connecting,
    AuditStreamState::Live,
    AuditStreamState::Reconnecting,
    AuditStreamState::Offline(OfflineReason::SignedOut),
    AuditStreamState::Offline(OfflineReason::Forbidden),
];

/// Every history state.
const HISTORIES: [HistoryLoad; 5] = [
    HistoryLoad::Waiting,
    HistoryLoad::Loading,
    HistoryLoad::Reloading,
    HistoryLoad::Loaded,
    HistoryLoad::Failed,
];

#[test]
fn audit_status_offline_outranks_everything_and_reloading_outranks_the_connection() {
    for history in HISTORIES {
        for reason in [OfflineReason::SignedOut, OfflineReason::Forbidden] {
            assert_eq!(
                status_badge(AuditStreamState::Offline(reason), history).0,
                "Offline"
            );
        }
    }
    for stream in &STREAMS[..3] {
        assert_eq!(status_badge(*stream, HistoryLoad::Reloading).0, "Reloading");
    }
}

#[test]
fn audit_status_history_falls_back_only_when_the_stream_fails_before_ready() {
    assert!(history_fallback_due(
        HistoryLoad::Waiting,
        AuditStreamState::Reconnecting
    ));
    assert!(history_fallback_due(
        HistoryLoad::Waiting,
        AuditStreamState::Offline(OfflineReason::Forbidden)
    ));
    assert!(!history_fallback_due(
        HistoryLoad::Waiting,
        AuditStreamState::Live
    ));
    assert!(!history_fallback_due(
        HistoryLoad::Waiting,
        AuditStreamState::Connecting
    ));
    for history in &HISTORIES[1..] {
        for stream in STREAMS {
            assert!(
                !history_fallback_due(*history, stream),
                "a history already requested never falls back again ({history:?}, {stream:?})"
            );
        }
    }
}

#[test]
fn audit_status_first_load_is_loading_and_later_ones_are_reloading() {
    assert_eq!(
        history_load_start(HistoryLoad::Waiting),
        HistoryLoad::Loading
    );
    assert_eq!(
        history_load_start(HistoryLoad::Loading),
        HistoryLoad::Loading
    );
    for later in [
        HistoryLoad::Reloading,
        HistoryLoad::Loaded,
        HistoryLoad::Failed,
    ] {
        assert_eq!(history_load_start(later), HistoryLoad::Reloading);
    }
}
