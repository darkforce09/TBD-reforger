use super::*;

const BOUNDS: PublicationBounds = PublicationBounds {
    tail: 40,
    retained_after: 10,
};

/// Without `Last-Event-ID` the stream opens at the tail with no reset.
///
/// RED: start a cursorless stream at 0 — the ready cursor is not the tail.
#[test]
fn audit_delivery_opening_without_cursor_starts_at_the_tail() {
    let (ready, reset, cursor) = opening(None, BOUNDS);
    assert_eq!(
        ready,
        AuditStreamReady {
            resume_after: 40,
            retained_after: 10
        }
    );
    assert_eq!(reset, None);
    assert_eq!(cursor, 40);
}

/// A cursor inside the retained history replays from itself; the floor itself is replayable.
///
/// RED: compare with `<=` against the floor — a cursor equal to the floor resets.
#[test]
fn audit_delivery_opening_inside_retained_history_replays_from_the_cursor() {
    for requested in [10, 25, 40] {
        let (ready, reset, cursor) = opening(Some(requested), BOUNDS);
        assert_eq!(ready.resume_after, requested);
        assert_eq!(ready.retained_after, 10);
        assert_eq!(reset, None, "cursor {requested} is replayable");
        assert_eq!(cursor, requested);
    }
}

/// A cursor beyond the tail is announced in `ready`, then reset to the tail as `cursor_ahead`.
///
/// RED: drop the tail comparison — the stream waits past sequences it never delivers.
#[test]
fn audit_delivery_opening_ahead_of_the_tail_resets_to_the_tail() {
    let (ready, reset, cursor) = opening(Some(41), BOUNDS);
    assert_eq!(ready.resume_after, 41);
    assert_eq!(
        reset,
        Some(AuditStreamReset {
            reason: AuditStreamResetReason::CursorAhead,
            resume_after: 40,
            retained_after: 10
        })
    );
    assert_eq!(cursor, 40);
}

/// A cursor below the floor resets to the tail as `history_unavailable`.
///
/// RED: drop the floor comparison — the stream replays across sequences that may be gone.
#[test]
fn audit_delivery_opening_below_the_floor_resets_to_the_tail() {
    for requested in [0, 9] {
        let (ready, reset, cursor) = opening(Some(requested), BOUNDS);
        assert_eq!(ready.resume_after, requested);
        assert_eq!(
            reset,
            Some(AuditStreamReset {
                reason: AuditStreamResetReason::HistoryUnavailable,
                resume_after: 40,
                retained_after: 10
            })
        );
        assert_eq!(cursor, 40);
    }
}

/// A live cursor the floor has passed is no longer replayable; one at or above it still is.
///
/// RED: check the floor only at open — a raised floor never resets a running stream.
#[test]
fn audit_delivery_reset_reason_follows_a_raised_floor() {
    let raised = PublicationBounds {
        tail: 60,
        retained_after: 45,
    };
    assert_eq!(
        reset_reason(44, raised),
        Some(AuditStreamResetReason::HistoryUnavailable)
    );
    assert_eq!(reset_reason(45, raised), None);
    assert_eq!(reset_reason(60, raised), None);
    assert_eq!(
        reset_reason(61, raised),
        Some(AuditStreamResetReason::CursorAhead)
    );
    assert_eq!(
        reset_to_tail(AuditStreamResetReason::HistoryUnavailable, raised),
        AuditStreamReset {
            reason: AuditStreamResetReason::HistoryUnavailable,
            resume_after: 60,
            retained_after: 45
        }
    );
}

/// An empty publication history opens at 0 and replays nothing missing.
#[test]
fn audit_delivery_opening_on_an_empty_history_starts_at_zero() {
    let empty = PublicationBounds {
        tail: 0,
        retained_after: 0,
    };
    assert_eq!(opening(None, empty).1, None);
    assert_eq!(opening(Some(0), empty).2, 0);
    assert_eq!(
        opening(Some(1), empty).1.map(|reset| reset.reason),
        Some(AuditStreamResetReason::CursorAhead)
    );
}
