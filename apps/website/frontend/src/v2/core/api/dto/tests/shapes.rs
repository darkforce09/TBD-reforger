//! Serialisation shape checks that need no captured fixture.

use super::*;
use crate::shell::nav_config::Role;

#[test]
fn paginated_shape() {
    let p: Paginated<i64> =
        serde_json::from_str(r#"{"data":[1,2,3],"total":3,"limit":20,"offset":0}"#).unwrap();
    assert_eq!(p.data, vec![1, 2, 3]);
    assert_eq!((p.total, p.limit, p.offset), (3, 20, 0));
}

#[test]
fn link_status_optionals() {
    let full: LinkStatus = serde_json::from_str(
        r#"{"linked":true,"arma_id":"a","arma_character":"Cpl","pending_code":true}"#,
    )
    .unwrap();
    assert!(full.linked && full.pending_code == Some(true) && full.arma_id.as_deref() == Some("a"));
    // The minimal shape (backend drops the empties)…
    let min: LinkStatus = serde_json::from_str(r#"{"linked":false}"#).unwrap();
    assert!(!min.linked && min.arma_id.is_none() && min.pending_code.is_none());
    // …and it re-serializes absent (skip_serializing_if), so it round-trips exactly.
    assert_eq!(serde_json::to_string(&min).unwrap(), r#"{"linked":false}"#);
}

#[test]
fn me_response_round_trips() {
    let json = r#"{"user":{"discord_id":"1","username":"u","discord_handle":"u#1","avatar_url":"","arma_id":null,"arma_character":"","role":"enlisted","is_banned":false,"total_deployments":0,"attendance_rate":0.0,"created_at":"t","updated_at":"t"},"arma_linked":true}"#;
    let me: MeResponse = serde_json::from_str(json).unwrap();
    assert!(me.arma_linked && me.user.role == Role::Enlisted);
    let back: MeResponse = serde_json::from_str(&serde_json::to_string(&me).unwrap()).unwrap();
    assert!(back == me, "MeResponse re-serialize → reparse is stable");
}

/// The live shape a filed leave request comes back as: midnight dates, a reason, and no reviewer
/// while it is still pending. Re-serialising must leave the reviewer key out rather than emit an
/// explicit null.
#[test]
fn leave_request_pending_round_trips_without_reviewed_by() {
    let json = r#"{"id":"44fa4c17-5bd5-4c6b-b02d-4ccd52af6910","discord_id":"000000000000000001","starts_on":"2026-09-01T00:00:00Z","ends_on":"2026-09-03T00:00:00Z","reason":"t265-probe","status":"pending","created_at":"2026-07-26T23:27:01.118063Z"}"#;
    let loa: LeaveRequest = serde_json::from_str(json).unwrap();
    assert_eq!(loa.status, "pending");
    assert!(loa.reviewed_by.is_none());
    assert_eq!(loa.starts_on, "2026-09-01T00:00:00Z");
    let back = serde_json::to_string(&loa).unwrap();
    assert!(
        !back.contains("reviewed_by"),
        "pending LOA must omit reviewed_by, got {back}"
    );
    let again: LeaveRequest = serde_json::from_str(&back).unwrap();
    assert_eq!(again, loa);
}

/// `{data:[LeaveRequest]}` — `GET /me/leave-requests` envelope the deployments page reads.
#[test]
fn leave_request_my_list_envelope() {
    let json = r#"{"data":[{"id":"44fa4c17-5bd5-4c6b-b02d-4ccd52af6910","discord_id":"000000000000000001","starts_on":"2026-09-01T00:00:00Z","ends_on":"2026-09-03T00:00:00Z","reason":"t265-probe","status":"pending","created_at":"2026-07-26T23:27:01.118063Z"}]}"#;
    let env: DataEnvelope<LeaveRequest> = serde_json::from_str(json).unwrap();
    assert_eq!(env.data.len(), 1);
    assert_eq!(env.data[0].reason, "t265-probe");
}

/// Create body is bare dates — the opposite of the response wire form.
#[test]
fn create_leave_input_serializes_bare_ymd() {
    let body = CreateLeaveInput {
        starts_on: "2026-08-01".into(),
        ends_on: "2026-08-05".into(),
        reason: "holiday".into(),
    };
    assert_eq!(
        serde_json::to_string(&body).unwrap(),
        r#"{"starts_on":"2026-08-01","ends_on":"2026-08-05","reason":"holiday"}"#
    );
}
