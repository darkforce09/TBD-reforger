use super::*;

const BOUNDS: PublicationBounds = PublicationBounds {
    tail: 40,
    retained_after: 10,
};

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
