//! Captured-response round trips for server rows, and the telemetry frame decoder.

use super::*;
use crate::v2::core::api::dto::servers::is_power_of_ten;

// ── `{data}` envelopes ──
/// Typed as the page reads it. An untyped envelope round-trips any payload, so the test passes
/// while the real type cannot deserialise the very golden it is pinned against — which is how a
/// float-versus-integer mismatch shipped and survived for a month. Typed, this fails loudly on the
/// fractional sample.
#[test]
fn servers_envelope() {
    assert_golden::<DataEnvelope<ServerRowDto>>(golden!("GET__servers.json"), &[]);
}

/// One **live** `GET /servers/:id/status/stream` frame, captured byte-exact off a running Axum
/// stack (`curl -sN .../status/stream`) whose `server_statuses` row reproduces the
/// `GET__servers.json` golden. Includes the `data: ` prefix and the `\n\n` terminator the
/// `sse.rs` splitter keys on, so the fixture is the wire and not a paraphrase of it.
///
/// The rest of the fixture corpus is request bodies, so without this one nothing pinned the type a
/// live consumer deserialises on every single frame.
///
/// It is embedded from the corpus rather than written out here because the DOM oracle serves the
/// same file to the browser as `text/event-stream`: one file means the bytes this test pins and the
/// bytes a rendered page receives cannot drift apart.
pub(crate) const LIVE_SSE_FRAME: &str =
    golden!("GET__servers__00000000-0000-4000-d000-000000000001__status__stream.sse.txt");

/// The captured live frame must deserialize, and must carry the tenth the `numeric(5,1)`
/// column really holds — rounding it away would be a second, quieter version of this bug.
#[test]
fn live_sse_frame_deserializes_with_its_fractional_fps() {
    let payload = LIVE_SSE_FRAME
        .trim()
        .strip_prefix("data:")
        .expect("captured frame is a data: frame")
        .trim();
    let dto: ServerStatusDto = serde_json::from_str(payload)
        .unwrap_or_else(|e| panic!("R-api: live SSE frame does not deserialize: {e}"));
    assert_eq!(dto.server_fps, 58.7, "the wire tenth must survive the DTO");
    assert_eq!(dto.player_count, 47);
    assert_eq!(dto.max_players, 64);
    assert_eq!(dto.uptime_seconds, 19842);
    assert_eq!(dto.ingame_time.as_deref(), Some("06:42"));
    assert_eq!(dto.ingame_weather.as_deref(), Some("overcast"));
    // The frame is also a golden: it must re-serialize canonically byte-equal.
    assert_eq!(canon(payload), canon(&serde_json::to_string(&dto).unwrap()));
}

// ── frame decoding ──
//
// These live beside the round-trip harness because the module that uses them is browser-only and
// therefore never compiled by the native test run, which is why the decoder itself lives here.

/// The captured live frame, through the real decoder. This once returned a rejection — every live
/// frame did — and the read loop dropped it without a word.
#[test]
fn a_live_frame_decodes_into_a_status() {
    match decode_server_status_frame(LIVE_SSE_FRAME) {
        SseFrame::Status(dto) => {
            assert_eq!(dto.server_fps, 58.7);
            assert_eq!(dto.player_count, 47);
            assert_eq!(dto.max_players, 64);
            assert!(dto.is_online);
        }
        other => panic!("live frame must decode into a status, got {other:?}"),
    }
}

/// The `i64` regression, pinned: a fractional `server_fps` must never be why a frame is dropped.
#[test]
fn a_fractional_fps_is_not_a_reason_to_reject_a_frame() {
    for fps in ["58.7", "0.0", "29.4", "60", "100.0", "19.9"] {
        let frame = format!(
            "data: {{\"server_id\":\"s\",\"is_online\":true,\"player_count\":1,\
             \"max_players\":2,\"server_fps\":{fps},\"uptime_seconds\":3,\
             \"updated_at\":\"t\"}}\n\n"
        );
        assert!(
            matches!(decode_server_status_frame(&frame), SseFrame::Status(_)),
            "server_fps={fps} must decode"
        );
    }
}

/// A malformed payload must come back carrying its reason. A bare `None` here is exactly what
/// made this class of defect invisible.
#[test]
fn a_bad_payload_is_rejected_with_its_reason_not_silently_dropped() {
    match decode_server_status_frame("data: {\"server_id\":\"s\",\"is_online\":\"yes\"}\n\n") {
        SseFrame::Rejected { error, payload } => {
            assert!(error.contains("invalid type"), "unexpected error: {error}");
            assert!(payload.contains("server_id"), "payload must be reported");
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

/// Keepalives and non-`data:` lines are NOT rejections — auditing them would drown the real
/// signal, which is the failure mode the audit exists to avoid.
#[test]
fn non_data_frames_are_not_audited_as_rejections() {
    for f in [": keepalive\n\n", "event: ping\n\n", "\n\n", "id: 7\n\n"] {
        assert_eq!(
            decode_server_status_frame(f),
            SseFrame::NotData,
            "frame {f:?}"
        );
    }
}

#[test]
fn the_warn_ladder_is_first_then_powers_of_ten() {
    for n in [10u64, 100, 1000, 10_000] {
        assert!(is_power_of_ten(n), "{n} should be on the ladder");
    }
    for n in [0u64, 1, 2, 9, 11, 99, 101, 1001] {
        assert!(!is_power_of_ten(n), "{n} should not be on the ladder");
    }
}

/// The audit message names the field and the two types to reconcile. A line that said only "parse
/// failed" would have cost exactly as much time as no line at all.
#[test]
fn the_audit_message_names_the_offending_field_and_both_structs() {
    let SseFrame::Rejected { error, payload } = decode_server_status_frame(
        "data: {\"server_id\":\"s\",\"is_online\":true,\"player_count\":1,\
         \"max_players\":2,\"server_fps\":\"nope\",\"uptime_seconds\":3,\"updated_at\":\"t\"}\n\n",
    ) else {
        panic!("expected Rejected");
    };
    let msg = audit_rejected_frame("test", &error, &payload);
    assert!(msg.contains("server_fps"), "must name the field: {msg}");
    assert!(msg.contains("ServerStatusDto") && msg.contains("ServerStatus"));
    assert!(msg.contains("REJECTED and dropped"));
}
