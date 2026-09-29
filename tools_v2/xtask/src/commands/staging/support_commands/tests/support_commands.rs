//! The read-only commands: preflight never sends a change and names every unmet check, status
//! compares each resting item with its expected value, capacity renders unaccounted values as
//! `-`, and fingerprints print only SHA-256 digests.
use super::fingerprints;
use super::host_capacity;
use super::preflight::{self, PreflightCheck};
use super::status;
use crate::commands::staging::discord_procedure::DiscordProcedure;
use crate::commands::staging::fleet_procedure::FleetProcedure;
use crate::commands::staging::load_procedure::LoadProcedure;
use crate::commands::staging::procedure_runner::fake_clock::FakeClock;
use crate::commands::staging::procedure_runner::procedure::StagingProcedure;
use crate::commands::staging::procedure_runner::runner_support::{
    ScriptedHost, scratch_folder, test_settings,
};
use crate::commands::staging::remote_observers::remote_command::{CommandPurpose, RemoteCommand};

#[test]
fn staging_preflight_stays_read_only() {
    let settings = test_settings();
    let mut checks = preflight::harness_checks(&settings, &scratch_folder("preflight-root"));
    checks.extend(FleetProcedure.preflight_checks(&settings));
    checks.extend(LoadProcedure::default().preflight_checks(&settings));
    checks.extend(DiscordProcedure.preflight_checks(&settings));
    for check in &checks {
        if let preflight::PreflightProbe::Host { command, .. } = &check.probe {
            assert_eq!(command.purpose, CommandPurpose::Read, "{}", check.name);
        }
    }
    // A list holding one changing command is refused before any check runs.
    let clock = FakeClock::starting_at(0);
    let mut host = ScriptedHost::new(&clock).answer("", 0, 0, "");
    let mut output = Vec::new();
    let changing = vec![
        PreflightCheck::host("reads", RemoteCommand::read("probe", "true".into()), |_| {
            Ok("ok".into())
        }),
        PreflightCheck::host(
            "changes",
            RemoteCommand::change("probe", "rm -f x".into()),
            |_| Ok("ok".into()),
        ),
    ];
    let refusal = preflight::run(&changing, &mut host, &mut output).unwrap_err();
    assert!(
        format!("{refusal:#}").contains("\"changes\" would change the host"),
        "{refusal:#}"
    );
    assert!(host.calls.is_empty() && output.is_empty());
}

#[test]
fn staging_preflight_reports_met_and_unmet_checks() {
    let settings = test_settings();
    let clock = FakeClock::starting_at(0);
    let mut host = ScriptedHost::new(&clock)
        .answer("current_database()", 0, 0, "tbd_reforger|on\n")
        .answer("/healthz", 0, 0, "502")
        .answer(
            "systemctl",
            0,
            0,
            "Id=tbd-reforger@1.service\nActiveState=active\n",
        )
        .answer("staging-fixtures", 0, 0, "present a\nmissing b\n");
    let checks: Vec<PreflightCheck> =
        preflight::harness_checks(&settings, &scratch_folder("preflight-report"))
            .into_iter()
            .filter(|check| matches!(check.probe, preflight::PreflightProbe::Host { .. }))
            .collect();
    let mut output = Vec::new();
    let code = preflight::run(&checks, &mut host, &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(code, 1, "{text}");
    assert!(
        text.contains("met    database read-only session: tbd_reforger|on"),
        "{text}"
    );
    assert!(
        text.contains("UNMET  API health: /healthz answered \"502\""),
        "{text}"
    );
    assert!(
        text.contains("UNMET  fleet units: not active: tbd-reforger@2.service"),
        "{text}"
    );
    assert!(text.contains("UNMET  host tools: missing b"), "{text}");
    assert!(text.ends_with("preflight: 1 met, 3 unmet\n"), "{text}");
    assert!(
        host.calls
            .iter()
            .all(|call| call.purpose == CommandPurpose::Read)
    );
}

#[test]
fn staging_status_compares_each_resting_item_with_its_expected_value() {
    let settings = test_settings();
    let clock = FakeClock::starting_at(0);
    let units: String = (1..=5)
        .flat_map(|n| {
            [
                format!("tbd-reforger@{n}.service"),
                format!("fleet-host-agent@{n}.service"),
            ]
        })
        .chain(["acknowledgement-dropping-relay@5.service".to_string()])
        .map(|unit| format!("Id={unit}\nActiveState=active\n\n"))
        .collect();
    let mut host = ScriptedHost::new(&clock)
        .answer("synthetic_accounts", 0, 0, "synthetic_accounts|0\nload_fixture_events|10\nactive_servers|5\nlive_missions_with_artifacts|2\nfleet_scenarios|2\nballistics_catalogs|1\n")
        .answer("staging-discord-outage.conf", 0, 0, "absent\n")
        .answer("control.sock", 0, 0, "{\"arming\":\"disarmed\",\"last_drop\":null}\n")
        .answer("--property=", 0, 0, &units);
    let items = status::items(&settings, &mut host);
    let text = status::render(&items);
    assert_eq!(
        status::exit_code(&items),
        1,
        "ten fixture events left behind: {text}"
    );
    let fixture_events = items
        .iter()
        .find(|item| item.name == "[Load fixture] events")
        .unwrap();
    assert_eq!(
        (
            fixture_events.observed.as_str(),
            fixture_events.expected.as_str()
        ),
        ("10", "0")
    );
    for name in [
        "synthetic accounts",
        "API outage drop-in",
        "relay of instance 5",
        "tbd-reforger@3.service",
    ] {
        assert!(
            items.iter().any(|item| item.name == name && item.holds()),
            "{name}: {text}"
        );
    }
    assert!(text.starts_with("resting state:\n"), "{text}");
    assert!(text.contains("setup content:\n"), "{text}");
    assert!(
        host.calls
            .iter()
            .all(|call| call.purpose == CommandPurpose::Read)
    );
}

#[test]
fn staging_status_reports_unreadable_items_as_off_their_resting_value() {
    let settings = test_settings();
    let clock = FakeClock::starting_at(0);
    let mut host = ScriptedHost::new(&clock).answer("", 0, 255, "");
    let items = status::items(&settings, &mut host);
    assert_eq!(status::exit_code(&items), 1);
    assert!(
        items
            .iter()
            .filter(|item| item.resting)
            .all(|item| item.observed.starts_with("unreadable"))
    );
}

#[test]
fn staging_capacity_renders_load_memory_and_units() {
    let units = vec![
        "tbd-reforger@1.service".to_string(),
        "tbd-reforger@2.service".to_string(),
    ];
    let command = host_capacity::command(&units);
    assert_eq!(command.purpose, CommandPurpose::Read);
    assert!(command.stdin.as_deref().unwrap().contains("'--property=Id,ActiveState,SubState,MainPID,ExecMainStartTimestampMonotonic,MemoryCurrent,CPUUsageNSec' 'tbd-reforger@1.service'"));
    let text = host_capacity::render(
        "loadavg=0.52 0.61 0.70\nmem_total_kib=32000000\nmem_available_kib=20000000\n\n\
         Id=tbd-reforger@1.service\nActiveState=active\nMemoryCurrent=2147483648\nCPUUsageNSec=1500000000000\n\n\
         Id=tbd-reforger@2.service\nActiveState=active\nMemoryCurrent=[not set]\n",
        &units,
    );
    assert!(
        text.contains("load average (1/5/15 min): 0.52 0.61 0.70"),
        "{text}"
    );
    assert!(
        text.contains("memory: 20000000 kB available of 32000000 kB"),
        "{text}"
    );
    assert!(text.contains("2048.0") && text.contains("1500.0"), "{text}");
    let second = text
        .lines()
        .find(|line| line.contains("tbd-reforger@2.service"))
        .unwrap();
    assert!(second.trim_end().ends_with('-'), "{second}");
}

#[test]
fn staging_fingerprints_print_only_sha256_digests() {
    let source = "a".repeat(64);
    let configuration = "0123456789abcdef".repeat(4);
    assert_eq!(
        fingerprints::render(&source, &configuration).unwrap(),
        format!("source_sha256={source}\nconfiguration_sha256={configuration}\n")
    );
    assert!(fingerprints::render(&"A".repeat(64), &configuration).is_err());
    assert!(fingerprints::render(&source, "abc").is_err());
}
