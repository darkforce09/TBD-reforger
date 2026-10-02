//! The committed load data, its agreement with the host tool, and the pure judges at their
//! acceptance edges and one step past them.

use std::collections::BTreeMap;

use developer_tools::staging_verification::load_generation::load_report::{
    RefreshSummary, UnexpectedErrors,
};
use regex::Regex;

use super::committed_load_data::{
    CommittedLoadData, FIRST_RESERVED_DISCORD_ID, POPULATION_FILE, WORKLOAD_FILE, workload_digest,
};
use super::game_operations::{delta, p95_bound, sample};
use super::load_preconditions::{
    KeyingAnswer, account_file_shape, cidr_covers, keying_answers_hold, strict_buckets_hold,
    trusted_proxies_hold,
};
use super::load_report_judges::{
    LoadThresholds, concurrency, member_accounts, p95_json_reads, p95_json_writes, refresh_pacing,
    sustained_rate, zero_unexpected_errors,
};
use super::load_test_support::{
    account_file_text, committed, exposition, passing_report, repository_root, source_ips,
};
use crate::commands::staging::procedure_runner::runner_support::scratch_folder;

fn read(path: &str) -> Vec<u8> {
    std::fs::read(repository_root().join(path)).unwrap()
}

fn constant(source: &str, name: &str) -> String {
    let pattern = Regex::new(&format!(r"const {name}: [^=]+= ([^;]+);")).unwrap();
    pattern
        .captures(source)
        .unwrap_or_else(|| panic!("{name} is not a constant"))
        .get(1)
        .unwrap()
        .as_str()
        .trim()
        .to_string()
}

#[test]
fn staging_load_data_decodes_agrees_and_digests_both_files_length_framed() {
    let data = committed();
    assert_eq!(
        data.population.accounts,
        data.workload.clients * data.workload.accounts_per_client
    );
    assert_eq!(data.population.accounts, 1_100);
    assert_eq!(
        (data.workload.clients, data.workload.source_address_count),
        (100, 5)
    );
    assert_eq!(
        (data.workload.ramp_seconds, data.workload.measured_seconds),
        (60.0, 1_800.0)
    );
    assert_eq!(data.workload.requests_per_second, 27.0);
    let (workload, population) = (read(WORKLOAD_FILE), read(POPULATION_FILE));
    assert_eq!(
        data.workload_sha256,
        workload_digest(&workload, &population)
    );
    assert_eq!(data.workload_sha256.len(), 64);
    assert_ne!(
        workload_digest(&workload, &population),
        workload_digest(&population, &workload)
    );
    let mut joined = workload.clone();
    joined.extend_from_slice(&population);
    assert_ne!(workload_digest(&joined, b""), data.workload_sha256);
}

#[test]
fn staging_load_population_states_the_host_tool_fixture_plan() {
    let root = repository_root().join("apps/api/src/bin/staging_fixtures");
    let plan = std::fs::read_to_string(root.join("load_fixture_events/fixture_plan.rs")).unwrap();
    let reserved = std::fs::read_to_string(root.join("reserved_accounts.rs")).unwrap();
    let data = committed();
    let events = &data.population.fixture_events;
    assert_eq!(
        constant(&plan, "FIXTURE_TITLE_PREFIX"),
        format!("{:?}", events.title_prefix)
    );
    assert_eq!(
        constant(&plan, "FIXTURE_EVENT_COUNT"),
        events.count.to_string()
    );
    assert_eq!(
        constant(&plan, "FIXTURE_FACTIONS").matches('"').count() / 2,
        events.factions as usize
    );
    assert_eq!(
        constant(&plan, "FIXTURE_SQUADS_PER_FACTION"),
        events.squads_per_faction.to_string()
    );
    assert_eq!(
        constant(&plan, "FIXTURE_SLOTS_PER_SQUAD"),
        events.slots_per_squad.to_string()
    );
    assert_eq!(
        constant(&plan, "FIXTURE_START_OFFSET_DAYS"),
        events.start_offset_days.to_string()
    );
    assert_eq!(
        constant(&plan, "LOAD_POPULATION_ACCOUNTS").replace('_', ""),
        data.population.accounts.to_string()
    );
    let first = constant(&reserved, "FIRST_RESERVED_DISCORD_ID").replace('_', "");
    assert_eq!(first, data.population.id_base);
    assert_eq!(first, FIRST_RESERVED_DISCORD_ID.to_string());
    assert_eq!(events.slots_per_event(), 128);
    assert_eq!(events.title(1), "[Load fixture] 01");
}

#[test]
fn staging_load_workload_stays_on_member_routes_eighty_twenty() {
    let data = committed();
    let mut weights: BTreeMap<String, u32> = BTreeMap::new();
    for template in &data.workload.request_mix {
        let class = serde_json::to_value(template.class)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        *weights.entry(class).or_default() += template.weight;
        for step in &template.steps {
            assert!(step.path.starts_with("/api/v1/"), "{}", step.path);
            for game in ["/game-runtime/", "/fleet-executor/", "/ingest/", "/auth/"] {
                assert!(!step.path.contains(game), "{} reaches {game}", step.path);
            }
        }
    }
    assert_eq!(weights["json_read"], 80);
    assert_eq!(weights["json_write"], 20);
    let fire = data
        .workload
        .request_mix
        .iter()
        .find(|template| template.id == "fire_mission_save")
        .unwrap();
    let body = fire.steps[0].body.as_ref().unwrap();
    assert_eq!(
        (
            body["catalog_id"].as_str(),
            body["catalog_version"].as_u64()
        ),
        (Some("vanilla_mortars"), Some(1))
    );
    assert_eq!(
        body["client_solution"]["solver_revision"],
        "game-ballistics-2"
    );
    assert_eq!(fire.steps[0].expected_statuses, vec![201]);
}

#[test]
fn staging_load_data_refuses_a_population_the_workload_cannot_carry() {
    let workload = read(WORKLOAD_FILE);
    let population = String::from_utf8(read(POPULATION_FILE)).unwrap();
    let short = population.replace("\"accounts\": 1100", "\"accounts\": 1000");
    assert!(
        CommittedLoadData::from_documents(&workload, short.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("1000 accounts")
    );
    let outside = population.replace("9100000000000000000", "9100000000000099000");
    assert!(
        CommittedLoadData::from_documents(&workload, outside.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("reserved range")
    );
    let unknown = population.replace("\"accounts\": 1100", "\"accounts\": 1100, \"extra\": 1");
    assert!(CommittedLoadData::from_documents(&workload, unknown.as_bytes()).is_err());
}

#[test]
fn staging_load_game_operations_delta_counts_by_status_and_bounds_p95_from_buckets() {
    let before = sample(&exposition(100, 100));
    let after = sample(&exposition(700, 670));
    let change = delta(&before, &after);
    let runtime = &change.families[0];
    assert_eq!(
        (
            runtime.family.as_str(),
            runtime.requests,
            runtime.server_errors
        ),
        ("game_runtime", 600, 0)
    );
    assert_eq!(runtime.p95_bound_seconds, Some(0.005));
    let slow = delta(&before, &sample(&exposition(700, 669)));
    assert_eq!(slow.families[0].p95_bound_seconds, Some(0.01));
    assert_eq!(change.families[1].requests, 0);
    assert!(!after.requests.contains_key("events"));
    let restarted = delta(&sample(&exposition(700, 670)), &before);
    assert_eq!(restarted.families[0].requests, 0);
    assert_eq!(p95_bound(&BTreeMap::from([("+Inf".to_string(), 10)])), None);
}

#[test]
fn staging_load_report_judges_hold_the_acceptance_thresholds_at_their_bounds() {
    let limits = LoadThresholds::ACCEPTANCE;
    let mut report = passing_report(1_800.0);
    report.json_reads.p95_milliseconds = Some(500.0);
    report.json_writes.p95_milliseconds = Some(1_000.0);
    report.completed_requests = 36_000;
    assert!(p95_json_reads(&report, &limits).is_ok() && p95_json_writes(&report, &limits).is_ok());
    assert!(sustained_rate(&report, &limits).is_ok());
    report.completed_requests = 35_999;
    assert!(sustained_rate(&report, &limits).is_err());
    report.json_writes.p95_milliseconds = None;
    assert!(p95_json_writes(&report, &limits).is_err());
    let short = passing_report(60.0);
    assert!(sustained_rate(&short, &limits).is_err());
    assert!(sustained_rate(&short, &LoadThresholds::rehearsal(60.0, 1_100)).is_ok());
    report.refreshes.statuses.insert(429, 1);
    assert!(refresh_pacing(&report, &limits).is_err());
    let mut crowded = passing_report(1_800.0);
    crowded.addresses[2].busiest_window_auth_requests = 2;
    assert!(
        refresh_pacing(&crowded, &limits)
            .unwrap_err()
            .contains("auth 2/1")
    );
}

#[test]
fn staging_load_judge_boundary_zero_unexpected_errors_fails_one_error_of_any_kind() {
    /// Counts one unexpected outcome under a single kind.
    type CountOne = fn(&mut UnexpectedErrors);
    let limits = LoadThresholds::ACCEPTANCE;
    let clean = passing_report(1_800.0);
    assert!(zero_unexpected_errors(&clean, &limits).is_ok());
    let kinds: [(CountOne, &str); 4] = [
        (
            |errors| errors.unexpected_statuses = 1,
            "1 unexpected (1 statuses, 0 transport errors, 0 timeouts, 0 undecodable refreshes)",
        ),
        (
            |errors| errors.transport_errors = 1,
            "1 unexpected (0 statuses, 1 transport errors, 0 timeouts, 0 undecodable refreshes)",
        ),
        (
            |errors| errors.timeouts = 1,
            "1 unexpected (0 statuses, 0 transport errors, 1 timeouts, 0 undecodable refreshes)",
        ),
        (
            |errors| errors.undecodable_session_answers = 1,
            "1 unexpected (0 statuses, 0 transport errors, 0 timeouts, 1 undecodable refreshes)",
        ),
    ];
    for (count_one, tally) in kinds {
        let mut report = clean.clone();
        count_one(&mut report.unexpected_errors);
        report.unexpected_errors.total = 1;
        let why = zero_unexpected_errors(&report, &limits).unwrap_err();
        assert!(why.starts_with(tally), "{why}");
    }
    let mut silent = clean;
    silent.requests_sent = 0;
    assert!(zero_unexpected_errors(&silent, &limits).is_err());
}

#[test]
fn staging_load_judge_boundary_p95_reads_and_writes_fail_a_thousandth_past_their_ceilings() {
    let limits = LoadThresholds::ACCEPTANCE;
    let mut report = passing_report(1_800.0);
    report.json_reads.p95_milliseconds = Some(500.0);
    report.json_writes.p95_milliseconds = Some(1_000.0);
    assert!(p95_json_reads(&report, &limits).is_ok());
    assert!(p95_json_writes(&report, &limits).is_ok());
    report.json_reads.p95_milliseconds = Some(500.001);
    report.json_writes.p95_milliseconds = Some(1_000.001);
    let reads = p95_json_reads(&report, &limits).unwrap_err();
    assert!(reads.ends_with("needs at most 500 ms"), "{reads}");
    let writes = p95_json_writes(&report, &limits).unwrap_err();
    assert!(writes.ends_with("needs at most 1000 ms"), "{writes}");
    report.json_reads.p95_milliseconds = Some(1.0);
    report.json_reads.expected = 0;
    assert!(p95_json_reads(&report, &limits).is_err());
}

#[test]
fn staging_load_judge_boundary_sustained_rate_needs_twenty_a_second_over_the_whole_window() {
    let limits = LoadThresholds::ACCEPTANCE;
    let mut report = passing_report(1_800.0);
    report.completed_requests = 36_000;
    assert!(sustained_rate(&report, &limits).is_ok());
    report.completed_requests = 35_999;
    let short = sustained_rate(&report, &limits).unwrap_err();
    assert!(
        short.ends_with("needs 1800 measured s and 36000 completed requests"),
        "{short}"
    );
    // A thirty-second of a second past 1,800 s needs 36,000.625 requests, so 36,001 whole ones.
    report.measured_seconds = 1_800.0 + 1.0 / 32.0;
    report.completed_requests = 36_001;
    assert!(sustained_rate(&report, &limits).is_ok());
    report.completed_requests = 36_000;
    let rounded = sustained_rate(&report, &limits).unwrap_err();
    assert!(
        rounded.ends_with("and 36001 completed requests"),
        "{rounded}"
    );
    // A window a thirty-second of a second short of 1,800 s fails whatever it completed.
    report.measured_seconds = 1_800.0 - 1.0 / 32.0;
    report.completed_requests = 48_600;
    assert!(sustained_rate(&report, &limits).is_err());
}

#[test]
fn staging_load_judge_boundary_concurrency_needs_one_hundred_clients_in_every_window() {
    let limits = LoadThresholds::ACCEPTANCE;
    let report = passing_report(1_800.0);
    assert_eq!(
        (
            report.census_windows.len(),
            report.minimum_concurrent_clients
        ),
        (180, 100)
    );
    assert!(concurrency(&report, &limits).is_ok());
    let mut thin = report.clone();
    thin.census_windows[179] = 99;
    thin.minimum_concurrent_clients = 99;
    assert_eq!(
        concurrency(&thin, &limits).unwrap_err(),
        "at least 99 distinct clients in each of 180 census windows of 10 s; needs 100"
    );
    let mut unwindowed = report;
    unwindowed.census_windows.clear();
    assert!(concurrency(&unwindowed, &limits).is_err());
}

#[test]
fn staging_load_judge_boundary_member_accounts_needs_one_thousand() {
    let limits = LoadThresholds::ACCEPTANCE;
    let mut report = passing_report(1_800.0);
    report.member_accounts = 1_000;
    assert!(member_accounts(&report, &limits).is_ok());
    report.member_accounts = 999;
    assert_eq!(
        member_accounts(&report, &limits).unwrap_err(),
        "999 member accounts of 1100 (1200 of 1200 refreshes succeeded); needs 1000"
    );
}

#[test]
fn staging_load_judge_boundary_refresh_pacing_holds_both_ceilings_and_every_refresh() {
    let limits = LoadThresholds::ACCEPTANCE;
    let report = passing_report(1_800.0);
    let ceilings = &report.per_address_ceilings;
    assert_eq!(ceilings, &committed().workload.per_address_ceilings);
    assert_eq!(
        (
            ceilings.all_requests.max_requests,
            ceilings.all_requests.window_seconds
        ),
        (8, 1.0)
    );
    assert_eq!(
        (
            ceilings.auth_requests.max_requests,
            ceilings.auth_requests.window_seconds
        ),
        (1, 2.0)
    );
    assert!(report.addresses.iter().all(|address| {
        (
            address.busiest_window_requests,
            address.busiest_window_auth_requests,
        ) == (8, 1)
    }));
    assert!(refresh_pacing(&report, &limits).is_ok());
    let mut busy = report.clone();
    busy.addresses[4].busiest_window_requests = 9;
    let why = refresh_pacing(&busy, &limits).unwrap_err();
    assert!(why.contains("192.0.2.243 9/8 auth 1/1"), "{why}");
    let mut crowded = report.clone();
    crowded.addresses[0].busiest_window_auth_requests = 2;
    let why = refresh_pacing(&crowded, &limits).unwrap_err();
    assert!(why.contains("192.0.2.117 8/8 auth 2/1"), "{why}");
    let mut refused = report.clone();
    refused.refreshes.succeeded = 1_199;
    refused.refreshes.statuses = BTreeMap::from([(200, 1_199), (401, 1)]);
    let why = refresh_pacing(&refused, &limits).unwrap_err();
    assert!(why.starts_with("1199 of 1200 refreshes succeeded"), "{why}");
    let mut idle = report;
    idle.refreshes = RefreshSummary::default();
    assert!(refresh_pacing(&idle, &limits).is_err());
}

#[test]
fn staging_load_preconditions_judge_proxies_keying_buckets_and_the_token_file() {
    let sources = source_ips();
    assert!(cidr_covers("127.0.0.0/8", &"127.0.0.1".parse().unwrap()));
    assert!(!cidr_covers(
        "192.168.0.0/24",
        &"192.168.1.5".parse().unwrap()
    ));
    assert!(cidr_covers("::1", &"::1".parse().unwrap()));
    assert!(!cidr_covers(
        "not-an-address",
        &"127.0.0.1".parse().unwrap()
    ));
    assert!(trusted_proxies_hold("127.0.0.1", &sources).is_ok());
    assert!(
        trusted_proxies_hold("10.0.0.1", &sources)
            .unwrap_err()
            .contains("does not cover")
    );
    assert!(
        trusted_proxies_hold("127.0.0.1, 192.0.2.0/24", &sources)
            .unwrap_err()
            .contains("192.0.2.117")
    );
    let answers: Vec<KeyingAnswer> = sources
        .iter()
        .map(|address| KeyingAnswer {
            address: address.to_string(),
            status: Some(401),
            error: None,
        })
        .collect();
    assert!(keying_answers_hold(&answers, &sources).is_ok());
    assert!(keying_answers_hold(&answers[..4], &sources).is_err());
    let buckets: String = sources
        .iter()
        .map(|address| format!("strict|{address}|1000000\n"))
        .collect();
    assert!(strict_buckets_hold(&buckets, &sources, 1_000_000).is_ok());
    assert!(strict_buckets_hold(&buckets, &sources, 1_000_000 + 300_001).is_err());
    let collapsed = "strict|127.0.0.1|1000000\n";
    assert!(
        strict_buckets_hold(collapsed, &sources, 0)
            .unwrap_err()
            .contains("192.0.2.243")
    );
    let file = scratch_folder("token-shape").join("load_tokens");
    std::fs::write(&file, account_file_text(3)).unwrap();
    let shape = account_file_shape(&file).unwrap();
    assert_eq!((shape.accounts, shape.consecutive), (3, true));
    assert_eq!(shape.last_discord_id, "9100000000000000002");
    assert!(!format!("{shape:?}").contains("not-a-token"));
}
