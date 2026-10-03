//! The workstation seam's process boundary: the plan and the report cross into and out of the
//! `staging-load` child as JSON that parses back byte-identically, and the child is the binary
//! developer_tools declares.

use staging_load_plan::workload_plan::FixtureEvent;
use staging_load_plan::{LoadRunPlan, decode_plan, decode_report, encode_plan, encode_report};

use super::load_test_support::{committed, passing_report, repository_root, source_ips};
use super::workstation_load::staging_load_build_arguments;

/// The committed workload bound to the test settings' five addresses and ten fixture events.
fn fixture_plan() -> LoadRunPlan {
    let data = committed();
    let slots = data.population.fixture_events.slots_per_event();
    LoadRunPlan {
        workload: data.workload,
        target_origin: "http://192.0.2.10:3080".into(),
        source_addresses: source_ips(),
        account_file: "/home/staging/tbd/run/load-accounts.json".into(),
        fixture_events: (1..=data.population.fixture_events.count)
            .map(|event| FixtureEvent {
                event_id: format!("event-{event}").into(),
                event_mission_id: format!("em-{event}").into(),
                mission_id: "mission-1".into(),
                slot_ids: (0..slots)
                    .map(|slot| format!("slot-{event}-{slot}").into())
                    .collect(),
            })
            .collect(),
    }
}

#[test]
fn the_seam_carries_the_committed_plan_into_the_child_byte_identically() {
    let plan = fixture_plan();
    plan.validate()
        .expect("the fixture plan passes the plan checks");
    let sent = encode_plan(&plan).expect("encodes");
    let received = decode_plan(&sent).expect("the child decodes it");
    assert_eq!(received, plan);
    assert_eq!(encode_plan(&received).expect("re-encodes"), sent);
}

#[test]
fn the_seam_carries_the_report_out_of_the_child_byte_identically() {
    let report = passing_report(1800.0);
    let printed = format!(
        "{}\n",
        encode_report(&report).expect("the child encodes it")
    );
    let read = decode_report(&printed).expect("decodes the child's line");
    assert_eq!(read, report);
    assert_eq!(
        format!("{}\n", encode_report(&read).expect("re-encodes")),
        printed
    );
}

#[test]
fn the_seam_builds_the_staging_load_binary_developer_tools_declares() {
    assert_eq!(
        staging_load_build_arguments(),
        [
            "build",
            "-q",
            "-p",
            "developer_tools",
            "--bin",
            "staging-load"
        ]
    );
    let manifest =
        std::fs::read_to_string(repository_root().join("tools/developer_tools/Cargo.toml"))
            .expect("the developer_tools manifest");
    let declared: toml::Value = manifest.parse::<toml::Table>().expect("toml").into();
    let binaries = declared["bin"].as_array().expect("[[bin]] tables");
    assert!(
        binaries
            .iter()
            .any(|binary| binary["name"].as_str() == Some("staging-load")
                && binary["path"].as_str() == Some("src/bin/staging_load.rs")),
        "developer_tools declares no staging-load binary"
    );
}
