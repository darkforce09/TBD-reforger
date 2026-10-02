use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use super::*;
use crate::staging_verification::load_generation::sample_plans::{
    SAMPLE_WORKLOAD_JSON, fixture_events, sample_workload,
};

fn sample_run() -> LoadRunPlan {
    LoadRunPlan {
        workload: sample_workload(),
        target_origin: "http://192.0.2.10:3080".to_owned(),
        source_addresses: (2..7)
            .map(|last| IpAddr::V4(Ipv4Addr::new(127, 0, 0, last)))
            .collect(),
        account_file: PathBuf::from("accounts.json"),
        fixture_events: fixture_events(10, 128),
    }
}

fn refusal(plan: &LoadRunPlan) -> String {
    format!(
        "{:#}",
        plan.validate().expect_err("the plan must be refused")
    )
}

#[test]
fn the_committed_shape_decodes_and_validates() {
    let plan = WorkloadPlan::from_json_str(SAMPLE_WORKLOAD_JSON).expect("decodes");
    plan.validate().expect("validates");
    assert_eq!(plan.clients, 100);
    assert_eq!(
        plan.per_address_ceilings.all_requests.requests_per_second(),
        8.0
    );
    assert_eq!(
        plan.per_address_ceilings
            .auth_requests
            .requests_per_second(),
        0.5
    );
    assert_eq!(plan.request_mix.len(), 5);
    let settings = plan.checked_settings().expect("settings");
    assert_eq!(settings.accounts, 1100);
    let period = settings.period.as_secs_f64();
    assert!((period - 100.0 / 27.0).abs() < 1e-9, "{period}");
}

#[test]
fn unknown_fields_are_refused_at_every_level() {
    let document: Value = serde_json::from_str(SAMPLE_WORKLOAD_JSON).expect("sample");
    let pointers = [
        "",
        "/per_address_ceilings",
        "/per_address_ceilings/auth_requests",
        "/request_mix/0",
        "/request_mix/0/steps/0",
    ];
    for pointer in pointers {
        let mut changed = document.clone();
        changed
            .pointer_mut(pointer)
            .and_then(Value::as_object_mut)
            .expect("an object")
            .insert("surplus".to_owned(), Value::Bool(true));
        let error = WorkloadPlan::from_json_str(&changed.to_string()).expect_err(pointer);
        assert!(
            format!("{error:#}").contains("surplus"),
            "{pointer}: {error:#}"
        );
    }
}

/// A change that puts one number of the workload out of range.
type Spoil = fn(&mut WorkloadPlan);

#[test]
fn numbers_out_of_range_are_refused() {
    let cases: [(&str, Spoil); 10] = [
        ("clients", |plan| plan.clients = 0),
        ("source_address_count", |plan| {
            plan.source_address_count = 101
        }),
        ("accounts_per_client", |plan| plan.accounts_per_client = 0),
        ("requests_per_second", |plan| plan.requests_per_second = 0.0),
        ("jitter_fraction", |plan| plan.jitter_fraction = 0.5),
        ("measured_seconds", |plan| plan.measured_seconds = 0.0),
        ("ramp_seconds", |plan| plan.ramp_seconds = -1.0),
        ("census_window_seconds", |plan| {
            plan.census_window_seconds = 1801.0
        }),
        ("max_requests", |plan| {
            plan.per_address_ceilings.auth_requests.max_requests = 0
        }),
        ("window_seconds", |plan| {
            plan.per_address_ceilings.all_requests.window_seconds = f64::INFINITY
        }),
    ];
    for (field, spoil) in cases {
        let mut plan = sample_workload();
        spoil(&mut plan);
        let error = format!("{:#}", plan.validate().expect_err(field));
        assert!(error.contains(field), "{field}: {error}");
    }
}

#[test]
fn a_ramp_too_short_for_the_sign_ins_is_refused_naming_the_minimum() {
    let committed = sample_workload();
    // 100 clients over 5 addresses under one refresh per 2 s: 100 / (5 × 0.5 × 0.8) = 50 s.
    assert!((committed.minimum_ramp_seconds() - 50.0).abs() < 1e-9);
    for ramp in [0.0, 10.0, 49.999] {
        let mut short = sample_workload();
        short.ramp_seconds = ramp;
        let error = format!("{:#}", short.validate().expect_err("a short ramp"));
        assert!(
            error.contains("ramp_seconds") && error.contains("at least 50 s"),
            "{ramp}: {error}"
        );
    }
    let mut minimum = sample_workload();
    minimum.ramp_seconds = minimum.minimum_ramp_seconds();
    assert_eq!(
        minimum.checked_settings().expect("the minimum ramp").ramp,
        Duration::from_secs(50)
    );
    let mut uneven = sample_workload();
    uneven.clients = 7;
    uneven.source_address_count = 3;
    uneven.ramp_seconds = 5.0;
    let error = format!("{:#}", uneven.validate().expect_err("a short ramp"));
    assert!(error.contains("at least 5.834 s"), "{error}");
    uneven.ramp_seconds = 5.834;
    uneven.validate().expect("the named minimum is accepted");
}

#[test]
fn the_target_must_be_a_plain_http_origin() {
    for (origin, expected) in [
        ("https://192.0.2.10:3080", "plain http"),
        ("http://192.0.2.10:3080/api", "without a path"),
        ("http://192.0.2.10:3080/?probe=1", "without a path"),
        ("http://user:secret@192.0.2.10:3080", "no credentials"),
        ("192.0.2.10:3080", "not a URL"),
    ] {
        let mut plan = sample_run();
        plan.target_origin = origin.to_owned();
        let error = refusal(&plan);
        assert!(error.contains(expected), "{origin}: {error}");
        assert!(
            !error.contains("secret"),
            "an origin with credentials is never echoed: {error}"
        );
    }
    let mut plan = sample_run();
    plan.target_origin = "http://192.0.2.10:3080/".to_owned();
    assert_eq!(
        plan.normalized_origin().expect("an origin"),
        "http://192.0.2.10:3080"
    );
}

#[test]
fn source_addresses_must_match_the_count_and_be_distinct_unicast_addresses() {
    let mut short = sample_run();
    short.source_addresses.pop();
    assert!(refusal(&short).contains("names 4 source addresses"));
    let mut repeated = sample_run();
    repeated.source_addresses[4] = repeated.source_addresses[0];
    assert!(refusal(&repeated).contains("named twice"));
    let mut unspecified = sample_run();
    unspecified.source_addresses[1] = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    assert!(refusal(&unspecified).contains("not a unicast address"));
}

#[test]
fn fixture_events_must_hold_a_safe_id_and_a_slot_for_every_account() {
    let mut empty = sample_run();
    empty.fixture_events.clear();
    assert!(refusal(&empty).contains("no fixture event"));
    let mut crowded = sample_run();
    crowded.fixture_events[3].slot_ids.truncate(109);
    assert!(refusal(&crowded).contains("fixture event 3 has 109 slots; 110 accounts"));
    let mut unsafe_id = sample_run();
    unsafe_id.fixture_events[0].event_mission_id = "../missions".to_owned();
    assert!(refusal(&unsafe_id).contains("event_mission_id"));
    let mut unsafe_slot = sample_run();
    unsafe_slot.fixture_events[1].slot_ids[5] = "slot 5".to_owned();
    assert!(refusal(&unsafe_slot).contains("slot id \"slot 5\""));
    sample_run().validate().expect("the sample run is valid");
}
