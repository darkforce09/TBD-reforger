//! Workloads, templates and fixture events the plan's tests and the load generator's tests build
//! plans from.
//!
//! - **Role:** a workload document in the committed shape, the committed workload at its size, and
//!   builders of templates, steps and fixture events.
//! - **Position:** compiled for this crate's tests and, behind the `test_fixtures` feature, for
//!   the load generator's tests, which enable it from their `[dev-dependencies]`.
//! - **Signals & state:** none; constants and pure builders.
//! - **Invariants:** every value here passes the plan's checks unless a test changes it.

use crate::identifiers::{EventId, EventMissionId, MissionId, SlotId, TemplateId};
use crate::workload_plan::{
    FixtureEvent, HttpMethod, PerAddressCeilings, RequestClass, RequestStep, RequestTemplate,
    WindowCeiling, WorkloadPlan,
};

/// A workload document in the committed shape, with every field and a representative mix.
pub const SAMPLE_WORKLOAD_JSON: &str = r#"{
  "seed": 20260929,
  "ramp_seconds": 60,
  "measured_seconds": 1800,
  "clients": 100,
  "source_address_count": 5,
  "requests_per_second": 27,
  "accounts_per_client": 11,
  "account_hold_seconds": 160,
  "jitter_fraction": 0.05,
  "request_timeout_seconds": 10,
  "census_window_seconds": 10,
  "per_address_ceilings": {
    "all_requests": { "max_requests": 8, "window_seconds": 1 },
    "auth_requests": { "max_requests": 1, "window_seconds": 2 }
  },
  "request_mix": [
    { "id": "events", "class": "json_read", "weight": 10,
      "steps": [ { "method": "GET", "path": "/api/v1/events", "expected_statuses": [200, 304] } ] },
    { "id": "me", "class": "json_read", "weight": 5,
      "steps": [ { "method": "GET", "path": "/api/v1/me", "expected_statuses": [200] } ] },
    { "id": "mission_bookmark", "class": "json_write", "weight": 2,
      "steps": [
        { "method": "POST", "path": "/api/v1/missions/{mission_id}/bookmark", "expected_statuses": [200, 204] },
        { "method": "DELETE", "path": "/api/v1/missions/{mission_id}/bookmark", "expected_statuses": [200, 204] }
      ] },
    { "id": "own_slot", "class": "json_write", "weight": 2,
      "steps": [
        { "method": "POST", "path": "/api/v1/event-missions/{event_mission_id}/register",
          "body": { "slot_id": "{slot_id}" }, "expected_statuses": [200] },
        { "method": "DELETE", "path": "/api/v1/event-missions/{event_mission_id}/register", "expected_statuses": [200] }
      ] },
    { "id": "fire_mission", "class": "json_write", "weight": 1,
      "steps": [ { "method": "POST", "path": "/api/v1/fire-missions",
                   "body": { "event_id": "{event_id}", "target_grid": "0{account_index}" },
                   "expected_statuses": [201] } ] }
  ]
}"#;

/// The committed shape at its committed size: 100 clients, 5 addresses, 27 requests a second.
pub fn sample_workload() -> WorkloadPlan {
    WorkloadPlan {
        seed: 7,
        ramp_seconds: 60.0,
        measured_seconds: 1800.0,
        clients: 100,
        source_address_count: 5,
        requests_per_second: 27.0,
        accounts_per_client: 11,
        account_hold_seconds: 160.0,
        jitter_fraction: 0.05,
        request_timeout_seconds: 10.0,
        census_window_seconds: 10.0,
        per_address_ceilings: PerAddressCeilings {
            all_requests: WindowCeiling {
                max_requests: 8,
                window_seconds: 1.0,
            },
            auth_requests: WindowCeiling {
                max_requests: 1,
                window_seconds: 2.0,
            },
        },
        request_mix: vec![read_template("events", "/api/v1/events", &[200])],
    }
}

/// A one-step JSON read.
pub fn read_template(id: impl Into<TemplateId>, path: &str, statuses: &[u16]) -> RequestTemplate {
    RequestTemplate {
        id: id.into(),
        class: RequestClass::JsonRead,
        weight: 1,
        steps: vec![step(HttpMethod::Get, path, None, statuses)],
    }
}

/// One step of a template.
pub fn step(
    method: HttpMethod,
    path: &str,
    body: Option<serde_json::Value>,
    statuses: &[u16],
) -> RequestStep {
    RequestStep {
        method,
        path: path.to_owned(),
        body,
        expected_statuses: statuses.to_vec(),
    }
}

/// `count` fixture events named `event-<e>`, `em-<e>` and `mission-<e>`, each with `slots` slots
/// named `slot-<e>-<s>`.
pub fn fixture_events(count: usize, slots: usize) -> Vec<FixtureEvent> {
    (0..count)
        .map(|event| FixtureEvent {
            event_id: EventId::new(format!("event-{event}")),
            event_mission_id: EventMissionId::new(format!("em-{event}")),
            mission_id: MissionId::new(format!("mission-{event}")),
            slot_ids: (0..slots)
                .map(|slot| SlotId::new(format!("slot-{event}-{slot}")))
                .collect(),
        })
        .collect()
}
