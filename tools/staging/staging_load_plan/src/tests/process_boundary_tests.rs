use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::Duration;

use super::*;
use crate::client_outcome::ClientOutcome;
use crate::latency_recording::{RequestOutcome, RequestRecord};
use crate::load_report::assemble;
use crate::sample_plans::{SAMPLE_WORKLOAD_JSON, fixture_events};
use crate::workload_plan::{RequestClass, WorkloadPlan};

/// The committed-shape workload, with its bodies, bound to a run.
fn sample_plan() -> LoadRunPlan {
    LoadRunPlan {
        workload: WorkloadPlan::from_json_str(SAMPLE_WORKLOAD_JSON).expect("sample workload"),
        target_origin: "http://192.0.2.10:3080".to_owned(),
        source_addresses: (2..7)
            .map(|last| IpAddr::V4(Ipv4Addr::new(127, 0, 0, last)))
            .collect(),
        account_file: PathBuf::from("/run/staging/accounts.json"),
        fixture_events: fixture_events(10, 110),
    }
}

/// One exchange of `class` finished `finished_seconds` after the run start.
fn record(class: RequestClass, template: Option<usize>, finished_seconds: f64) -> RequestRecord {
    let finished = Duration::from_secs_f64(finished_seconds);
    RequestRecord {
        client: 3,
        address: 1,
        account: 14,
        class,
        template,
        scheduled: finished - Duration::from_millis(42),
        sent: finished - Duration::from_millis(40),
        finished,
        outcome: RequestOutcome::Expected { status: 200 },
        auth: template.is_none(),
    }
}

/// A report folded from a refresh, a read and a write inside the measured window.
fn sample_report(plan: &LoadRunPlan) -> LoadReport {
    let settings = plan.checked_settings().expect("settings");
    let outcome = ClientOutcome {
        address: 1,
        records: vec![
            record(RequestClass::Session, None, 61.5),
            record(RequestClass::JsonRead, Some(0), 62.25),
            record(RequestClass::JsonWrite, Some(2), 63.125),
        ],
        skipped_slots: 2,
        unsent_slots: 1,
        guard_delayed: 4,
        late_switches: 0,
    };
    assemble(plan, &settings, &[outcome])
}

#[test]
fn a_plan_crosses_the_process_boundary_byte_identically() {
    let plan = sample_plan();
    let encoded = encode_plan(&plan).expect("encodes");
    let decoded = decode_plan(&encoded).expect("decodes");
    assert_eq!(decoded, plan);
    assert_eq!(encode_plan(&decoded).expect("re-encodes"), encoded);
}

#[test]
fn a_report_crosses_the_process_boundary_byte_identically() {
    let report = sample_report(&sample_plan());
    let encoded = encode_report(&report).expect("encodes");
    let decoded = decode_report(&format!("{encoded}\n")).expect("decodes with a final newline");
    assert_eq!(decoded, report);
    assert_eq!(encode_report(&decoded).expect("re-encodes"), encoded);
}

#[test]
fn an_unknown_field_is_refused_on_either_side() {
    let mut plan: serde_json::Value =
        serde_json::from_str(&encode_plan(&sample_plan()).expect("encodes")).expect("json");
    plan["surplus"] = serde_json::json!(1);
    let error = decode_plan(&plan.to_string()).expect_err("refused");
    assert!(
        format!("{error}").starts_with("decoding the load plan: unknown field `surplus`"),
        "{error}"
    );
    let error = decode_report("{\"seed\": 1}").expect_err("refused");
    assert!(
        format!("{error}").starts_with("decoding the load report: "),
        "{error}"
    );
}
