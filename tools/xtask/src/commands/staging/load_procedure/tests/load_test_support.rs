//! Test support of the load procedure: the committed data, settings with five source addresses,
//! a stub workstation, a passing report, and the recorded host answers of a seeded staging host.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Result, anyhow};
use developer_tools::staging_verification::load_generation::LoadReport;
use developer_tools::staging_verification::load_generation::LoadRunPlan;
use developer_tools::staging_verification::load_generation::load_report::{
    AddressSummary, ClassSummary, RefreshSummary, TemplateSummary, UnexpectedErrors,
};
use developer_tools::staging_verification::load_generation::workload_plan::{
    PerAddressCeilings, RequestClass, WindowCeiling,
};

use super::committed_load_data::CommittedLoadData;
use super::workstation_load::WorkstationLoad;
use crate::commands::staging::procedure_runner::fake_clock::FakeClock;
use crate::commands::staging::procedure_runner::runner_support::{ScriptedHost, test_settings};
use crate::commands::staging::staging_settings::StagingSettings;

/// The simulated start of every recorded run.
pub(super) const START_UNIX_MS: u64 = 1_800_000_000_000;
/// The five source addresses of the test settings.
pub(super) const SOURCES: [&str; 5] = [
    "192.0.2.117",
    "192.0.2.240",
    "192.0.2.241",
    "192.0.2.242",
    "192.0.2.243",
];

/// The repository root of this checkout.
pub(super) fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The committed workload and population.
pub(super) fn committed() -> CommittedLoadData {
    CommittedLoadData::read(&repository_root()).expect("the committed load data decodes")
}

/// The test settings with five source addresses and a load target.
pub(super) fn load_settings() -> StagingSettings {
    let mut settings = test_settings();
    settings.load_source_addresses = SOURCES.iter().map(ToString::to_string).collect();
    settings.load_target_origin = Some("http://192.0.2.10:3080".into());
    settings
}

/// The parsed source addresses.
pub(super) fn source_ips() -> Vec<IpAddr> {
    SOURCES
        .iter()
        .map(|address| address.parse().unwrap())
        .collect()
}

/// A workstation that answers every keying refresh with one status and every member load with
/// one outcome, remembering the plans it was handed.
pub(super) struct StubWorkstation {
    pub keying_status: u16,
    pub outcome: Result<LoadReport, String>,
    pub plans: Mutex<Vec<LoadRunPlan>>,
}

impl StubWorkstation {
    pub(super) fn answering(outcome: Result<LoadReport, String>) -> Self {
        Self {
            keying_status: 401,
            outcome,
            plans: Mutex::new(Vec::new()),
        }
    }
}

impl WorkstationLoad for StubWorkstation {
    fn keying_refresh(&self, _origin: &str, _address: IpAddr) -> Result<u16> {
        Ok(self.keying_status)
    }

    fn member_load(&self, plan: &LoadRunPlan) -> Result<LoadReport> {
        self.plans.lock().unwrap().push(plan.clone());
        self.outcome.clone().map_err(|why| anyhow!(why))
    }
}

fn class(completed: u64, p95: f64) -> ClassSummary {
    ClassSummary {
        completed,
        expected: completed,
        p50_milliseconds: Some(p95 / 2.0),
        p95_milliseconds: Some(p95),
        max_milliseconds: Some(p95 * 2.0),
    }
}

/// A report that meets every acceptance threshold over `measured_seconds`.
pub(super) fn passing_report(measured_seconds: f64) -> LoadReport {
    let completed = (measured_seconds * 27.0) as u64;
    LoadReport {
        seed: 20_260_929,
        clients: 100,
        accounts: 1_100,
        target_requests_per_second: 27.0,
        period_milliseconds: 3_703.7,
        ramp_seconds: 60.0,
        measured_seconds,
        requests_sent: completed + 2_000,
        completed_requests: completed,
        achieved_requests_per_second: 27.0,
        skipped_slots: 0,
        unsent_slots: 0,
        guard_delayed_requests: 3,
        late_switches: 0,
        unexpected_errors: UnexpectedErrors::default(),
        json_reads: class(completed * 79 / 100, 120.0),
        json_writes: class(completed * 20 / 100, 240.0),
        sessions: class(completed / 100, 80.0),
        census_window_seconds: 10.0,
        census_windows: vec![100; (measured_seconds / 10.0) as usize],
        minimum_concurrent_clients: 100,
        member_accounts: 1_100,
        refreshes: RefreshSummary {
            attempted: 1_200,
            succeeded: 1_200,
            statuses: BTreeMap::from([(200, 1_200)]),
        },
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
        addresses: SOURCES
            .iter()
            .map(|address| AddressSummary {
                address: address.to_string(),
                clients: 20,
                requests: completed / 5,
                auth_requests: 240,
                busiest_window_requests: 8,
                busiest_window_auth_requests: 1,
            })
            .collect(),
        templates: vec![TemplateSummary {
            id: "events".into(),
            class: RequestClass::JsonRead,
            statuses: BTreeMap::from([(200, completed)]),
            unexpected: 0,
            transport_errors: 0,
            timeouts: 0,
        }],
    }
}

/// `count` fixture event rows as `LOAD_FIXTURE_EVENTS` returns them, each with `slots` slots.
pub(super) fn fixture_event_rows(count: u32, slots: u32, mission_title: &str) -> String {
    (1..=count)
        .map(|event| {
            let slot_ids: Vec<String> = (0..slots).map(|slot| format!("slot-{event}-{slot}")).collect();
            format!(
                "[Load fixture] {event:02}|event-{event}|em-{event}|mission-1|{mission_title}|{slots}|{}\n",
                slot_ids.join(",")
            )
        })
        .collect()
}

/// An account file of `accounts` synthetic accounts from the first reserved id, with fake
/// tokens.
pub(super) fn account_file_text(accounts: u64) -> String {
    let entries: Vec<String> = (0..accounts)
        .map(|k| {
            format!(
                r#"{{"discord_id":"{}","refresh_token":"not-a-token-{k}"}}"#,
                9_100_000_000_000_000_000u64 + k
            )
        })
        .collect();
    format!(r#"{{"accounts":[{}]}}"#, entries.join(","))
}

/// A game-runtime counter exposition with `requests` heartbeats, `fast` of them within 5 ms.
pub(super) fn exposition(requests: u64, fast: u64) -> String {
    let route = "/api/v1/game-runtime/sessions/{sessionId}/heartbeats";
    format!(
        "# TYPE tbd_http_requests_total counter\n\
         tbd_http_requests_total{{method=\"POST\",route=\"{route}\",status=\"200\"}} {requests}\n\
         tbd_http_requests_total{{method=\"GET\",route=\"/api/v1/events\",status=\"200\"}} 5000\n\
         tbd_http_request_duration_seconds_bucket{{method=\"POST\",route=\"{route}\",le=\"0.005\"}} {fast}\n\
         tbd_http_request_duration_seconds_bucket{{method=\"POST\",route=\"{route}\",le=\"0.01\"}} {requests}\n\
         tbd_http_request_duration_seconds_bucket{{method=\"POST\",route=\"{route}\",le=\"+Inf\"}} {requests}\n"
    )
}

/// Five fleet servers whose last heartbeat is `age_ms` old by the database's clock.
pub(super) fn census_rows(age_ms: u64) -> String {
    (1..=5)
        .map(|n| {
            format!(
                "TBD Staging {n}|server-{n}|{n}|{}|{START_UNIX_MS}\n",
                START_UNIX_MS - age_ms
            )
        })
        .collect()
}

/// A staging host holding the seeded population, fixture events, fresh strict buckets, a
/// heartbeating fleet and the API's counters, which grow after the first simulated second.
pub(super) fn seeded_host(clock: &FakeClock) -> ScriptedHost {
    let buckets: String = SOURCES
        .iter()
        .map(|address| format!("strict|{address}|{START_UNIX_MS}\n"))
        .collect();
    ScriptedHost::new(clock)
        .answer(
            "FROM users",
            0,
            0,
            "1100|1100|9100000000000000000|9100000000000001099\n",
        )
        .answer(
            "[Load fixture]",
            0,
            0,
            &fixture_event_rows(10, 128, "TBD Staging Everon"),
        )
        .answer("rate_limit_buckets", 0, 0, &buckets)
        .answer("server_runtime_sessions", 0, 0, &census_rows(5_000))
        .answer("/metrics", 0, 0, &exposition(100, 100))
        .answer("/metrics", START_UNIX_MS + 1_000, 0, &exposition(700, 670))
        .answer("FROM missions WHERE title", 0, 0, "mission-1\n")
}
