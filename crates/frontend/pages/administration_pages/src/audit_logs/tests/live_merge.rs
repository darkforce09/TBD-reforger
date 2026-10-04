//! The audit board: dedupe across the live stream and the history, overlap, ordering, the history
//! continuation, and restarts.

use super::*;
use serde_json::json;

/// One typed audit line with the given id and message.
fn line(id: i64, message: &str) -> AuditLogEntry {
    serde_json::from_value(json!({
        "id": id,
        "severity": "warn",
        "action": "user.warn",
        "message": message,
        "created_at": "2026-07-01T00:00:00Z"
    }))
    .expect("a valid audit line")
}

/// A history page of the given ids and cursor.
fn page_of(ids: &[i64], next_cursor: Option<i64>) -> CursorList<AuditLogEntry> {
    CursorList {
        data: ids.iter().map(|id| line(*id, "history")).collect(),
        next_cursor: next_cursor.map(|c| json!(c)),
    }
}

/// The board's ids, in display order.
fn ids(board: &AuditBoard) -> Vec<i64> {
    board.rows().iter().map(|l| l.id.get()).collect()
}

#[test]
fn audit_board_starts_empty_at_epoch_zero() {
    let board = AuditBoard::new();
    assert!(board.is_empty());
    assert_eq!(board.epoch(), 0);
    assert_eq!(board.live_count(), 0);
    assert_eq!(board.continuation(), None);
}

#[test]
fn audit_board_orders_rows_newest_id_first_whatever_the_arrival_order() {
    let mut board = AuditBoard::new();
    board.insert_live(line(7, "live"));
    board.insert_live(line(12, "live"));
    board.merge_history(0, page_of(&[10, 9, 3], None));
    board.insert_live(line(5, "live, allocated early and published late"));
    assert_eq!(ids(&board), vec![12, 10, 9, 7, 5, 3]);
}

#[test]
fn audit_board_dedupes_a_live_row_repeated_by_the_stream() {
    let mut board = AuditBoard::new();
    assert!(board.insert_live(line(4, "first")));
    assert!(!board.insert_live(line(4, "replayed after a reconnect")));
    assert_eq!(ids(&board), vec![4]);
    assert_eq!(board.live_count(), 1);
    assert_eq!(board.get(4).map(|l| l.message.as_str()), Some("first"));
}

#[test]
fn audit_board_dedupes_history_that_overlaps_live_rows() {
    let mut board = AuditBoard::new();
    // Lines committed between `ready` and the history answer arrive both ways.
    board.insert_live(line(11, "live"));
    board.insert_live(line(10, "live"));
    assert!(board.merge_history(0, page_of(&[11, 10, 9, 8], Some(8))));
    assert_eq!(ids(&board), vec![11, 10, 9, 8]);
    assert_eq!(board.live_count(), 2, "the overlap stays counted as live");
    assert_eq!(board.get(11).map(|l| l.message.as_str()), Some("live"));
}

#[test]
fn audit_board_dedupes_a_live_row_the_history_already_holds() {
    let mut board = AuditBoard::new();
    board.merge_history(0, page_of(&[3, 2, 1], None));
    assert!(!board.insert_live(line(2, "live copy")));
    assert_eq!(ids(&board), vec![3, 2, 1]);
    assert_eq!(board.live_count(), 0);
}

#[test]
fn audit_board_dedupes_overlapping_history_pages() {
    let mut board = AuditBoard::new();
    board.merge_history(0, page_of(&[20, 19, 18], Some(18)));
    board.merge_history(0, page_of(&[18, 17, 16], Some(16)));
    assert_eq!(ids(&board), vec![20, 19, 18, 17, 16]);
}

#[test]
fn audit_board_continues_below_the_smallest_history_id_not_a_live_id() {
    let mut board = AuditBoard::new();
    board.merge_history(0, page_of(&[50, 49, 48], Some(48)));
    // A late-published line with an old id must not drag the keyset below lines never loaded.
    board.insert_live(line(3, "live"));
    assert_eq!(board.continuation(), Some(48));
    board.merge_history(0, page_of(&[47, 46], Some(46)));
    assert_eq!(board.continuation(), Some(46));
}

#[test]
fn audit_board_has_no_continuation_before_a_history_page_or_after_the_last() {
    let mut board = AuditBoard::new();
    board.insert_live(line(9, "live"));
    assert_eq!(
        board.continuation(),
        None,
        "live rows alone never open the keyset"
    );
    board.merge_history(0, page_of(&[8, 7], None));
    assert_eq!(board.continuation(), None);
    // A cursor on an empty page cannot move the keyset, so it ends the history too.
    let mut empty = AuditBoard::new();
    empty.merge_history(0, page_of(&[], Some(5)));
    assert_eq!(empty.continuation(), None);
    // A non-numeric cursor is the end rather than a guess.
    let mut odd = AuditBoard::new();
    odd.merge_history(
        0,
        CursorList {
            data: vec![line(2, "history")],
            next_cursor: Some(json!("2")),
        },
    );
    assert_eq!(odd.continuation(), None);
}

#[test]
fn audit_board_restart_empties_everything_and_moves_the_epoch() {
    let mut board = AuditBoard::new();
    board.insert_live(line(30, "live"));
    board.merge_history(0, page_of(&[29, 28], Some(28)));
    let epoch = board.restart();
    assert_eq!(epoch, 1);
    assert_eq!(board.epoch(), 1);
    assert!(board.is_empty());
    assert_eq!(board.live_count(), 0);
    assert_eq!(board.continuation(), None);
}

#[test]
fn audit_board_drops_a_history_page_from_before_a_restart() {
    let mut board = AuditBoard::new();
    let stale = board.epoch();
    let fresh = board.restart();
    assert!(!board.merge_history(stale, page_of(&[5, 4], Some(4))));
    assert!(
        board.is_empty(),
        "a page of the replaced history must not land"
    );
    assert_eq!(board.continuation(), None);
    assert!(board.merge_history(fresh, page_of(&[6, 5], None)));
    assert_eq!(ids(&board), vec![6, 5]);
}

#[test]
fn audit_board_reset_then_reload_keeps_live_rows_that_arrived_after_the_reset() {
    let mut board = AuditBoard::new();
    board.merge_history(0, page_of(&[10, 9], None));
    let epoch = board.restart();
    board.insert_live(line(12, "after the reset"));
    assert!(board.merge_history(epoch, page_of(&[12, 11, 10], Some(10))));
    assert_eq!(ids(&board), vec![12, 11, 10]);
    assert_eq!(board.live_count(), 1);
    assert_eq!(board.continuation(), Some(10));
}
