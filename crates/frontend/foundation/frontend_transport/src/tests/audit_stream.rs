//! The audit stream's pure half: the step decoding, the resume cursor, the backoff and the connect
//! verdict; plus a pin on the browser half's cursor and abort wiring.

use super::*;
use crate::sse_frames::SseParser;
use frontend_api_dtos::administration::{AuditSeverity, AuditStreamResetReason};
use frontend_api_dtos::identifiers::ServerSentEventId;
use frontend_test_support::class_r_scrub::{live_code, only_body};

/// One audit line's JSON with the given line id.
fn row_json(id: i64) -> String {
    format!(
        r#"{{"id":{id},"severity":"info","action":"user.warn","message":"line {id}","created_at":"2026-07-01T00:00:00Z"}}"#
    )
}

/// Feed `stream` through a parser and the tracker, collecting every step that is not `Nothing`.
fn steps(tracker: &mut AuditStreamTracker, stream: &str) -> Vec<AuditStreamStep> {
    let mut parser = SseParser::new();
    parser
        .feed(stream.as_bytes())
        .into_iter()
        .map(|item| tracker.observe(item))
        .filter(|step| *step != AuditStreamStep::Nothing)
        .collect()
}

/// The cursor the tracker's next connection would send, read without starting that connection.
fn resume_cursor(tracker: &AuditStreamTracker) -> Option<i64> {
    tracker
        .clone()
        .begin_connection()
        .and_then(|id| id.parse().ok())
}

/// A ready frame with the given cursor and floor.
fn ready_frame(resume_after: i64, retained_after: i64) -> String {
    format!(
        "event: ready\nid: {resume_after}\ndata: {{\"resume_after\":{resume_after},\"retained_after\":{retained_after}}}\n\n"
    )
}

#[test]
fn audit_stream_path_is_the_admin_stream_route() {
    assert_eq!(AUDIT_STREAM_PATH, "/admin/audit-logs/stream");
}

#[test]
fn audit_stream_connect_verdicts() {
    assert_eq!(connect_verdict(200, false), ConnectVerdict::Stream);
    assert_eq!(connect_verdict(204, true), ConnectVerdict::Stream);
    assert_eq!(connect_verdict(401, false), ConnectVerdict::Revalidate);
    assert_eq!(
        connect_verdict(401, true),
        ConnectVerdict::Stop(OfflineReason::SignedOut)
    );
    for revalidated in [false, true] {
        assert_eq!(
            connect_verdict(403, revalidated),
            ConnectVerdict::Stop(OfflineReason::Forbidden)
        );
        assert_eq!(
            connect_verdict(400, revalidated),
            ConnectVerdict::ForgetCursorAndBackOff
        );
    }
    for status in [0, 404, 429, 500, 502, 503] {
        assert_eq!(connect_verdict(status, false), ConnectVerdict::BackOff);
    }
}

#[test]
fn audit_stream_parses_only_non_negative_cursors() {
    assert_eq!(parse_cursor(&ServerSentEventId::new("10")), Some(10));
    assert_eq!(parse_cursor(&ServerSentEventId::new("0")), Some(0));
    assert_eq!(parse_cursor(&ServerSentEventId::new("")), None);
    assert_eq!(parse_cursor(&ServerSentEventId::new("-1")), None);
    assert_eq!(parse_cursor(&ServerSentEventId::new("ten")), None);
}

#[test]
fn audit_stream_backoff_doubles_from_one_second_to_the_ceiling() {
    let nominal: Vec<u32> = (0..8).map(|n| reconnect_delay_ms(1_000, n, 0.0)).collect();
    assert_eq!(
        nominal,
        vec![1_000, 2_000, 4_000, 8_000, 16_000, 30_000, 30_000, 30_000]
    );
    assert_eq!(reconnect_delay_ms(1_000, u32::MAX, 0.0), 30_000);
}

#[test]
fn audit_stream_first_wait_is_held_between_one_second_and_the_ceiling() {
    assert_eq!(reconnect_delay_ms(0, 0, 0.0), 1_000);
    assert_eq!(reconnect_delay_ms(200, 1, 0.0), 2_000);
    assert_eq!(reconnect_delay_ms(5_000, 0, 0.0), 5_000);
    assert_eq!(reconnect_delay_ms(5_000, 2, 0.0), 20_000);
    assert_eq!(reconnect_delay_ms(u32::MAX, 0, 0.0), 30_000);
}

#[test]
fn audit_stream_a_server_retry_hint_sets_the_first_wait() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let _ = steps(&mut tracker, "retry: 5000\n");
    assert_eq!(tracker.next_delay_ms(0.0), 5_000);
    assert_eq!(tracker.next_delay_ms(0.0), 10_000);
    let _ = steps(&mut tracker, "retry: 99999999999\n");
    assert_eq!(tracker.next_delay_ms(0.0), 30_000);
}

#[test]
fn audit_stream_jitter_adds_at_most_a_quarter() {
    assert_eq!(reconnect_delay_ms(1_000, 0, 0.5), 1_125);
    assert!(reconnect_delay_ms(1_000, 0, 0.999_999) < 1_250);
    assert!(reconnect_delay_ms(1_000, 9, 0.999_999) < 37_500);
    // Out-of-range draws clamp rather than shrink or overshoot the window.
    assert_eq!(reconnect_delay_ms(1_000, 0, -3.0), 1_000);
    assert_eq!(reconnect_delay_ms(1_000, 0, 7.0), 1_250);
}

#[test]
fn audit_stream_first_connection_sends_no_cursor_and_its_ready_requires_history() {
    let mut tracker = AuditStreamTracker::new();
    assert_eq!(tracker.begin_connection(), None);
    let got = steps(&mut tracker, &ready_frame(10, 0));
    assert_eq!(
        got,
        vec![AuditStreamStep::Ready {
            ready: AuditStreamReady {
                resume_after: 10,
                retained_after: 0
            },
            history_required: true,
        }]
    );
    assert_eq!(resume_cursor(&tracker), Some(10));
}

#[test]
fn audit_stream_rows_advance_the_cursor_by_publication_sequence_not_line_id() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let stream = format!(
        "{}id: 11\ndata: {}\n\nid: 12\ndata: {}\n\n",
        ready_frame(10, 0),
        row_json(3),
        row_json(900)
    );
    let got = steps(&mut tracker, &stream);
    assert_eq!(got.len(), 3);
    let ids: Vec<i64> = got
        .iter()
        .filter_map(|step| match step {
            AuditStreamStep::Row(entry) => Some(entry.id.get()),
            _ => None,
        })
        .collect();
    assert_eq!(ids, vec![3, 900]);
    assert_eq!(resume_cursor(&tracker), Some(12));
    let AuditStreamStep::Row(first) = &got[1] else {
        panic!("second step must be a row");
    };
    assert_eq!(first.severity, AuditSeverity::Info);
    assert_eq!(first.message, "line 3");
}

#[test]
fn audit_stream_reconnect_resumes_after_the_last_id_and_needs_no_history() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let _ = steps(
        &mut tracker,
        &format!("{}id: 11\ndata: {}\n\n", ready_frame(10, 0), row_json(5)),
    );
    assert_eq!(tracker.begin_connection().as_deref(), Some("11"));
    let got = steps(&mut tracker, &ready_frame(11, 0));
    assert_eq!(
        got,
        vec![AuditStreamStep::Ready {
            ready: AuditStreamReady {
                resume_after: 11,
                retained_after: 0
            },
            history_required: false,
        }]
    );
}

#[test]
fn audit_stream_reset_moves_the_cursor_to_the_tail_even_backwards() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let _ = steps(
        &mut tracker,
        &format!("{}id: 50\ndata: {}\n\n", ready_frame(40, 0), row_json(7)),
    );
    tracker.begin_connection();
    let stream = format!(
        "{}event: reset\nid: 20\ndata: {{\"reason\":\"cursor_ahead\",\"resume_after\":20,\"retained_after\":0}}\n\n",
        ready_frame(50, 0)
    );
    let got = steps(&mut tracker, &stream);
    assert_eq!(
        got[1],
        AuditStreamStep::Reset(AuditStreamReset {
            reason: AuditStreamResetReason::CursorAhead,
            resume_after: 20,
            retained_after: 0,
        })
    );
    assert_eq!(resume_cursor(&tracker), Some(20));
    assert_eq!(tracker.begin_connection().as_deref(), Some("20"));
}

#[test]
fn audit_stream_history_unavailable_reset_is_decoded() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let got = steps(
        &mut tracker,
        "event: reset\nid: 90\ndata: {\"reason\":\"history_unavailable\",\"resume_after\":90,\"retained_after\":60}\n\n",
    );
    assert_eq!(
        got,
        vec![AuditStreamStep::Reset(AuditStreamReset {
            reason: AuditStreamResetReason::HistoryUnavailable,
            resume_after: 90,
            retained_after: 60,
        })]
    );
    assert_eq!(resume_cursor(&tracker), Some(90));
}

#[test]
fn audit_stream_control_events_without_an_id_fall_back_to_their_payload_cursor() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let _ = steps(
        &mut tracker,
        "event: ready\ndata: {\"resume_after\":33,\"retained_after\":0}\n\n",
    );
    assert_eq!(resume_cursor(&tracker), Some(33));
}

#[test]
fn audit_stream_an_unreadable_row_is_rejected_and_skipped_by_the_cursor() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let got = steps(
        &mut tracker,
        &format!("{}id: 11\ndata: {{\"id\":\"x\"}}\n\n", ready_frame(10, 0)),
    );
    assert!(matches!(
        &got[1],
        AuditStreamStep::Rejected { event, .. } if event == "message"
    ));
    assert_eq!(resume_cursor(&tracker), Some(11));
}

#[test]
fn audit_stream_an_unreadable_ready_leaves_the_cursor_and_the_connection_unready() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let got = steps(&mut tracker, "event: ready\nid: 10\ndata: nope\n\n:\n\n");
    assert!(matches!(
        &got[..],
        [AuditStreamStep::Rejected { event, .. }] if event == "ready"
    ));
    assert_eq!(resume_cursor(&tracker), None);
    // No `ready`, so the keep-alive after it does not count the connection healthy.
    assert_eq!(tracker.next_delay_ms(0.0), 1_000);
    assert_eq!(tracker.next_delay_ms(0.0), 2_000);
}

#[test]
fn audit_stream_authorization_expired_is_surfaced_and_keeps_the_cursor() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let got = steps(
        &mut tracker,
        &format!(
            "{}event: authorization_expired\ndata: session permissions changed; reconnect\n\n",
            ready_frame(10, 0)
        ),
    );
    assert_eq!(got[1], AuditStreamStep::AuthorizationExpired);
    assert_eq!(resume_cursor(&tracker), Some(10));
}

#[test]
fn audit_stream_ignores_keep_alives_retry_hints_and_unknown_events() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let got = steps(
        &mut tracker,
        ":\n\nretry: 5000\nevent: telemetry\ndata: {}\n\n",
    );
    assert!(got.is_empty());
}

#[test]
fn audit_stream_backoff_grows_until_a_connection_proves_healthy() {
    let mut tracker = AuditStreamTracker::new();
    // A server that accepts and closes at once, again and again, is backed off.
    for expected in [1_000, 2_000, 4_000] {
        tracker.begin_connection();
        let _ = steps(&mut tracker, &ready_frame(10, 0));
        assert_eq!(tracker.next_delay_ms(0.0), expected);
    }
    // A keep-alive before `ready` proves nothing.
    tracker.begin_connection();
    let _ = steps(&mut tracker, &format!(":\n\n{}", ready_frame(10, 0)));
    assert_eq!(tracker.next_delay_ms(0.0), 8_000);
    // A keep-alive after `ready` shows the connection was held open: the wait starts over.
    tracker.begin_connection();
    let _ = steps(&mut tracker, &format!("{}:\n\n", ready_frame(10, 0)));
    assert_eq!(tracker.next_delay_ms(0.0), 1_000);
    assert_eq!(tracker.next_delay_ms(0.0), 2_000);
}

#[test]
fn audit_stream_a_row_after_ready_also_resets_the_backoff() {
    let mut tracker = AuditStreamTracker::new();
    for _ in 0..4 {
        let _ = tracker.next_delay_ms(0.0);
    }
    tracker.begin_connection();
    let _ = steps(
        &mut tracker,
        &format!("{}id: 11\ndata: {}\n\n", ready_frame(10, 0), row_json(1)),
    );
    assert_eq!(tracker.next_delay_ms(0.0), 1_000);
}

#[test]
fn audit_stream_forgetting_the_cursor_starts_fresh_and_requires_history_again() {
    let mut tracker = AuditStreamTracker::new();
    tracker.begin_connection();
    let _ = steps(&mut tracker, &ready_frame(10, 0));
    tracker.forget_cursor();
    assert_eq!(tracker.begin_connection(), None);
    let got = steps(&mut tracker, &ready_frame(12, 0));
    assert!(matches!(
        got[..],
        [AuditStreamStep::Ready {
            history_required: true,
            ..
        }]
    ));
}

#[test]
fn audit_stream_a_chunked_stream_yields_the_same_steps_as_a_whole_one() {
    let stream = format!(
        "{}:\n\nid: 11\r\ndata: {}\r\n\r\nevent: reset\nid: 30\ndata: {{\"reason\":\"history_unavailable\",\"resume_after\":30,\"retained_after\":25}}\n\n",
        ready_frame(10, 0),
        row_json(4)
    );
    let mut whole_tracker = AuditStreamTracker::new();
    whole_tracker.begin_connection();
    let whole = steps(&mut whole_tracker, &stream);
    assert_eq!(whole.len(), 3);
    for cut in (0..stream.len()).step_by(7) {
        let mut tracker = AuditStreamTracker::new();
        tracker.begin_connection();
        let mut parser = SseParser::new();
        let bytes = stream.as_bytes();
        let mut got: Vec<AuditStreamStep> = Vec::new();
        for part in [&bytes[..cut], &bytes[cut..]] {
            got.extend(
                parser
                    .feed(part)
                    .into_iter()
                    .map(|item| tracker.observe(item))
                    .filter(|step| *step != AuditStreamStep::Nothing),
            );
        }
        assert_eq!(got, whole, "split at {cut}");
        assert_eq!(resume_cursor(&tracker), Some(30));
    }
}

/// The browser half cannot run natively, so its wiring is pinned on the scrubbed source: every
/// connection sends the tracker's cursor, carries its own abort signal, parks its controller in
/// the page-owned handle, and the handle's abort stops the loop and the connection. No global
/// slot holds the handle.
#[test]
fn audit_stream_transport_resumes_from_the_tracker_and_aborts_through_the_handle() {
    let code = live_code(include_str!("../audit_stream/transport.rs"));
    let run = only_body(&code, "async fn run_stream(");
    assert!(run.contains("let last_event_id = tracker.begin_connection();"));
    assert!(run.contains("connect(&handle, &token, last_event_id.as_deref())"));
    assert!(run.contains("tracker.next_delay_ms("));
    assert!(run.contains("revalidate_session(store)"));
    let request = only_body(&code, "fn stream_request(");
    assert!(request.contains("init.set_signal(Some(&controller.signal()))"));
    assert!(request.contains("headers.set("));
    let connect = only_body(&code, "async fn connect(");
    assert!(connect.contains("stream_request(token, last_event_id)"));
    assert!(connect.contains("*handle.controller.borrow_mut() = Some(controller)"));
    let abort = only_body(&code, "pub fn abort(&self)");
    assert!(abort.contains("self.stopped.set(true)"));
    assert!(abort.contains("self.abort_connection()"));
    assert!(
        !code.contains("thread_local!"),
        "the handle belongs to the page; no global slot may hold it"
    );
}

/// The audit stream's `view!` markup, if any, braces every attribute value that would otherwise end
/// its tag early: the same scan the audit log page runs over its own folder, here over the stream
/// module and its folder, located from this file.
#[test]
fn view_attributes_in_the_audit_stream_are_braced_where_they_must_be() {
    use frontend_test_support::repository_root::source_file_folder;
    use frontend_test_support::view_attribute_guard::assert_view_attributes_are_well_formed;
    let tests = source_file_folder(env!("CARGO_MANIFEST_DIR"), file!());
    let transport = tests
        .parent()
        .expect("the tests folder sits inside the transport folder");
    assert_view_attributes_are_well_formed(
        env!("CARGO_MANIFEST_DIR"),
        &[
            transport.join("audit_stream.rs"),
            transport.join("audit_stream"),
        ],
    );
}
