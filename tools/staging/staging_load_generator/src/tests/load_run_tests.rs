use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::json;

use super::run;
use crate::stub_api_server::{QUICK_ROUTE, StubApi, StubHit, StubOptions};
use staging_load_plan::LoadReport;
use staging_load_plan::sample_plans::{fixture_events, read_template, step};
use staging_load_plan::workload_plan::{
    HttpMethod, LoadRunPlan, PerAddressCeilings, RequestClass, RequestTemplate, WindowCeiling,
    WorkloadPlan,
};

/// Slack for arrival instants against send instants: the stub stamps a request when its handler
/// runs, a little after the client let it go.
const ARRIVAL_SLACK: Duration = Duration::from_millis(100);

/// The shape of one short run against the stub.
#[derive(Clone)]
struct Shape {
    clients: u32,
    addresses: u32,
    accounts_per_client: u32,
    requests_per_second: f64,
    ramp: f64,
    measured: f64,
    hold: f64,
    timeout: f64,
    census: f64,
    all_requests: (u32, f64),
    auth_requests: (u32, f64),
    events: usize,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            clients: 2,
            addresses: 1,
            accounts_per_client: 1,
            requests_per_second: 20.0,
            ramp: 0.2,
            measured: 1.5,
            hold: 60.0,
            timeout: 2.0,
            census: 0.5,
            all_requests: (40, 1.0),
            auth_requests: (20, 1.0),
            events: 2,
        }
    }
}

/// An account file in the temporary directory, removed when dropped.
struct AccountFile(PathBuf);

impl Drop for AccountFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Accounts `0..count` with generation-0 refresh tokens, as the stub expects them.
fn account_file(name: &str, count: u32) -> AccountFile {
    let path = std::env::temp_dir().join(format!(
        "tbd-load-generation-{name}-{}.json",
        std::process::id()
    ));
    let accounts: Vec<_> = (0..count)
        .map(|account| {
            json!({
                "discord_id": (9_100_000_000_000_000_000u64 + u64::from(account)).to_string(),
                "refresh_token": format!("refresh-{account}-0"),
            })
        })
        .collect();
    std::fs::write(&path, json!({ "accounts": accounts }).to_string()).expect("account file");
    AccountFile(path)
}

/// The loopback source addresses 127.0.0.2 onwards.
fn loopback_sources(count: u32) -> Vec<IpAddr> {
    (0..count)
        .map(|index| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2 + index as u8)))
        .collect()
}

fn plan(
    origin: &str,
    shape: &Shape,
    mix: Vec<RequestTemplate>,
    accounts: &AccountFile,
) -> LoadRunPlan {
    let account_count = shape.clients * shape.accounts_per_client;
    let slots = (account_count as usize).div_ceil(shape.events);
    LoadRunPlan {
        workload: WorkloadPlan {
            seed: 11,
            ramp_seconds: shape.ramp,
            measured_seconds: shape.measured,
            clients: shape.clients,
            source_address_count: shape.addresses,
            requests_per_second: shape.requests_per_second,
            accounts_per_client: shape.accounts_per_client,
            account_hold_seconds: shape.hold,
            jitter_fraction: 0.05,
            request_timeout_seconds: shape.timeout,
            census_window_seconds: shape.census,
            per_address_ceilings: PerAddressCeilings {
                all_requests: WindowCeiling {
                    max_requests: shape.all_requests.0,
                    window_seconds: shape.all_requests.1,
                },
                auth_requests: WindowCeiling {
                    max_requests: shape.auth_requests.0,
                    window_seconds: shape.auth_requests.1,
                },
            },
            request_mix: mix,
        },
        target_origin: origin.to_owned(),
        source_addresses: loopback_sources(shape.addresses),
        account_file: accounts.0.clone(),
        fixture_events: fixture_events(shape.events, slots),
    }
}

fn run_against(stub: &StubApi, name: &str, shape: &Shape, mix: Vec<RequestTemplate>) -> LoadReport {
    let accounts = account_file(name, shape.clients * shape.accounts_per_client);
    run(&plan(&stub.origin, shape, mix, &accounts)).expect("the load run")
}

fn write_template(
    id: &str,
    weight: u32,
    steps: Vec<staging_load_plan::workload_plan::RequestStep>,
) -> RequestTemplate {
    RequestTemplate {
        id: id.into(),
        class: RequestClass::JsonWrite,
        weight,
        steps,
    }
}

fn weighted(mut template: RequestTemplate, weight: u32) -> RequestTemplate {
    template.weight = weight;
    template
}

/// Arrival instants at the stub from `peer` of the hits `keep` selects, ascending.
fn arrivals(hits: &[StubHit], peer: IpAddr, keep: impl Fn(&StubHit) -> bool) -> Vec<Instant> {
    let mut instants: Vec<Instant> = hits
        .iter()
        .filter(|hit| hit.peer == peer && keep(hit))
        .map(|hit| hit.at)
        .collect();
    instants.sort_unstable();
    instants
}

/// No `max + 1` arrivals fall inside `window`, less the arrival slack.
fn assert_ceiling(instants: &[Instant], max: usize, window: Duration, what: &str) {
    for run in instants.windows(max + 1) {
        let span = run[max] - run[0];
        assert!(
            span >= window - ARRIVAL_SLACK,
            "{what}: {} arrivals within {span:?}",
            max + 1
        );
    }
}

fn template<'a>(
    report: &'a LoadReport,
    id: &str,
) -> &'a staging_load_plan::load_report::TemplateSummary {
    report
        .templates
        .iter()
        .find(|template| template.id == id)
        .expect("template in the report")
}

/// The hits `keep` selects among those of `client`, whose accounts are `client mod clients`, in
/// arrival order.
fn client_hits(
    hits: &[StubHit],
    client: usize,
    clients: usize,
    keep: impl Fn(&StubHit) -> bool,
) -> Vec<StubHit> {
    let mut selected: Vec<StubHit> = hits
        .iter()
        .filter(|hit| hit.account.map(|account| account % clients) == Some(client) && keep(hit))
        .cloned()
        .collect();
    selected.sort_by_key(|hit| hit.at);
    selected
}

/// Every member request of `client` carries a current token of the account whose switch last
/// passed: `turns[m]` from `m` holds after the client's sign-in, give or take the arrival slack.
fn assert_accounts_follow_the_switches(
    hits: &[StubHit],
    client: usize,
    clients: usize,
    hold: Duration,
    turns: &[usize],
) {
    let sign_in = client_hits(hits, client, clients, StubHit::is_refresh)[0].at;
    let turn = |since: Duration| {
        usize::try_from(since.as_nanos() / hold.as_nanos())
            .unwrap_or(usize::MAX)
            .min(turns.len() - 1)
    };
    for hit in client_hits(hits, client, clients, |hit| !hit.is_refresh()) {
        assert!(!hit.stale_token, "{hit:?}");
        let since = hit.at.saturating_duration_since(sign_in);
        let possible = turn(since.saturating_sub(ARRIVAL_SLACK))..=turn(since + ARRIVAL_SLACK);
        assert!(
            possible
                .clone()
                .any(|turn| hit.account == Some(turns[turn])),
            "client {client}: {hit:?} {since:?} after the sign-in, expected one of {:?}",
            possible.map(|turn| turns[turn]).collect::<Vec<_>>()
        );
    }
}

/// Once `client` sends its first member request, no two follow each other further apart than
/// `most`: no switch leaves it without an account.
fn assert_no_gap(hits: &[StubHit], client: usize, clients: usize, most: Duration) {
    let members = client_hits(hits, client, clients, |hit| !hit.is_refresh());
    assert!(
        members.len() >= 2,
        "client {client} sent too little to judge"
    );
    for pair in members.windows(2) {
        let gap = pair[1].at - pair[0].at;
        assert!(gap <= most, "client {client}: {gap:?} between {pair:?}");
    }
}

#[test]
fn every_source_address_stays_under_its_request_and_auth_ceilings() {
    let stub = StubApi::start(2, None);
    let shape = Shape {
        clients: 6,
        addresses: 2,
        accounts_per_client: 2,
        requests_per_second: 60.0,
        ramp: 1.5,
        measured: 2.0,
        hold: 0.5,
        all_requests: (5, 0.5),
        auth_requests: (1, 0.4),
        ..Shape::default()
    };
    let mix = vec![read_template("events", "/api/v1/events", &[200])];
    let report = run_against(&stub, "ceilings", &shape, mix);
    let hits = stub.hits();
    for peer in loopback_sources(2) {
        let all = arrivals(&hits, peer, |_| true);
        let auth = arrivals(&hits, peer, StubHit::is_refresh);
        assert!(
            all.len() >= 10 && auth.len() >= 2,
            "{peer}: too little traffic to judge"
        );
        assert_ceiling(
            &all,
            5,
            Duration::from_millis(500),
            &format!("{peer} requests"),
        );
        assert_ceiling(
            &auth,
            1,
            Duration::from_millis(400),
            &format!("{peer} refreshes"),
        );
    }
    for address in &report.addresses {
        assert_eq!(address.clients, 3);
        assert!(address.busiest_window_requests <= 5, "{address:?}");
        assert!(address.busiest_window_auth_requests <= 1, "{address:?}");
    }
    assert!(
        report.guard_delayed_requests > 0,
        "the demand never reached a ceiling"
    );
    assert_eq!(
        report.unexpected_errors.total, 0,
        "{:?}",
        report.unexpected_errors
    );
}

#[test]
fn clients_rotate_accounts_every_hold_and_present_each_refresh_token_once() {
    let stub = StubApi::start(2, None);
    let shape = Shape {
        clients: 4,
        addresses: 2,
        accounts_per_client: 3,
        requests_per_second: 40.0,
        ramp: 0.4,
        measured: 3.35,
        hold: 0.8,
        auth_requests: (1, 0.1),
        ..Shape::default()
    };
    // Clients sign in 100 ms apart and switch every 800 ms, so switch 4 falls between 3.2 s and
    // 3.5 s and switch 5 after 4.0 s: the window closing at 3.75 s leaves slack on both sides.
    let accounts = account_file("rotation", 12);
    let before = std::fs::read(&accounts.0).expect("account file");
    let mix = vec![read_template("events", "/api/v1/events", &[200])];
    let report = run(&plan(&stub.origin, &shape, mix, &accounts)).expect("the load run");
    assert_eq!(std::fs::read(&accounts.0).expect("account file"), before);
    assert_eq!(stub.refresh_replays(), 0);
    assert_eq!(
        report.unexpected_errors.total, 0,
        "{:?}",
        report.unexpected_errors
    );
    // Five refreshes per client: the sign-in, then its three accounts' successors in turn, the
    // first two again with the refresh tokens their first refresh handed over.
    assert_eq!(
        (report.refreshes.attempted, report.refreshes.succeeded),
        (20, 20)
    );
    assert_eq!(report.late_switches, 0);
    for client in 0..4 {
        for (turn, generation) in [(0, 2), (1, 2), (2, 1)] {
            assert_eq!(
                stub.generation(client + 4 * turn),
                generation,
                "account {}",
                client + 4 * turn
            );
        }
    }
    assert_eq!(report.member_accounts, 12);
    let hits = stub.hits();
    let hold = Duration::from_millis(800);
    for client in 0..4 {
        let refreshes = client_hits(&hits, client, 4, StubHit::is_refresh);
        assert_eq!(refreshes.len(), 5);
        // The sign-in, then each prefetch half a hold before its switch.
        for (number, refresh) in (1u32..).zip(&refreshes[1..]) {
            let prefetch = refreshes[0].at + hold * number - hold / 2;
            assert!(
                refresh.at.max(prefetch) - refresh.at.min(prefetch) <= ARRIVAL_SLACK,
                "client {client}, prefetch {number}: {:?} from the sign-in",
                refresh.at - refreshes[0].at
            );
        }
        let turns = [client, client + 4, client + 8, client, client + 4];
        assert_accounts_follow_the_switches(&hits, client, 4, hold, &turns);
    }
}

#[test]
fn latency_runs_from_the_scheduled_instant_through_a_backlog() {
    let stub = StubApi::start(1, None);
    let shape = Shape {
        clients: 1,
        requests_per_second: 10.0,
        ramp: 0.1,
        measured: 2.0,
        ..Shape::default()
    };
    let report = run_against(
        &stub,
        "backlog",
        &shape,
        vec![read_template("slow", "/api/v1/slow", &[200])],
    );
    let reads = &report.json_reads;
    // Slots come every 100 ms and each answer takes 200 ms, so the client falls further behind
    // with every request: from the send each latency would stay near 200 ms.
    assert!(reads.expected >= 5, "{reads:?}");
    assert!(
        reads.max_milliseconds.expect("a sample") >= 600.0,
        "{reads:?}"
    );
    assert!(
        reads.p95_milliseconds.expect("a sample") >= 600.0,
        "{reads:?}"
    );
    assert!(
        report.unsent_slots > 0,
        "the backlog never cleared, so slots stay unsent"
    );
}

#[test]
fn latency_runs_to_the_end_of_the_body() {
    let stub = StubApi::start(1, None);
    let shape = Shape {
        clients: 1,
        requests_per_second: 2.0,
        ramp: 0.1,
        measured: 2.2,
        ..Shape::default()
    };
    let mix = vec![read_template("streamed", "/api/v1/streamed", &[200])];
    let report = run_against(&stub, "body_end", &shape, mix);
    let reads = &report.json_reads;
    // The headers come at once and the body ends three 100 ms pauses later.
    assert!(reads.expected >= 3, "{reads:?}");
    assert!(
        reads.p50_milliseconds.expect("a sample") >= 300.0,
        "{reads:?}"
    );
    assert_eq!(report.unexpected_errors.total, 0);
}

#[test]
fn a_304_is_expected_only_where_the_template_declares_it() {
    let stub = StubApi::start(1, None);
    let mix = vec![
        read_template("conditional_read", "/api/v1/conditional", &[200, 304]),
        read_template("plain_read", "/api/v1/events", &[200]),
    ];
    let report = run_against(&stub, "not_modified", &Shape::default(), mix);
    assert_eq!(
        report.unexpected_errors.total, 0,
        "{:?}",
        report.unexpected_errors
    );
    let conditional = template(&report, "conditional_read");
    assert_eq!(
        conditional.statuses.get(&200),
        Some(&2),
        "one full answer per account"
    );
    assert!(
        conditional
            .statuses
            .get(&304)
            .is_some_and(|&count| count > 0),
        "{conditional:?}"
    );
    assert_eq!(report.json_reads.expected, report.json_reads.completed);
    let hits = stub.hits();
    for account in 0..2 {
        let tagged: Vec<bool> = hits
            .iter()
            .filter(|hit| hit.path == "/api/v1/conditional" && hit.account == Some(account))
            .map(|hit| hit.if_none_match)
            .collect();
        assert_eq!(tagged.first(), Some(&false));
        assert!(
            tagged[1..].iter().all(|&tag| tag),
            "account {account}: {tagged:?}"
        );
    }
    assert!(
        hits.iter()
            .filter(|hit| hit.path == "/api/v1/events")
            .all(|hit| !hit.if_none_match)
    );
}

#[test]
fn unexpected_statuses_transport_errors_and_timeouts_are_told_apart() {
    let stub = StubApi::start(1, None);
    let shape = Shape {
        clients: 3,
        requests_per_second: 15.0,
        ramp: 0.2,
        measured: 1.6,
        timeout: 0.3,
        ..Shape::default()
    };
    let mix = vec![
        read_template("fails", "/api/v1/fail", &[200]),
        read_template("hangs", "/api/v1/hang", &[200]),
        read_template("aborts", "/api/v1/abort", &[200]),
        read_template("not_modified", "/api/v1/not-modified", &[200]),
    ];
    let report = run_against(&stub, "classification", &shape, mix);
    let hits = stub.hits();
    let arrived = |path: &str| hits.iter().filter(|hit| hit.path == path).count() as u64;
    let fails = template(&report, "fails");
    assert!(
        fails.unexpected >= 1 && fails.statuses.get(&500) == Some(&fails.unexpected),
        "{fails:?}"
    );
    assert_eq!((fails.transport_errors, fails.timeouts), (0, 0));
    assert_eq!(fails.unexpected, arrived("/api/v1/fail"));
    let hangs = template(&report, "hangs");
    assert!(
        hangs.timeouts >= 1 && hangs.timeouts == hangs.unexpected,
        "{hangs:?}"
    );
    assert!(
        hangs.statuses.is_empty() && hangs.transport_errors == 0,
        "{hangs:?}"
    );
    assert_eq!(hangs.timeouts, arrived("/api/v1/hang"));
    let aborts = template(&report, "aborts");
    assert!(
        aborts.transport_errors >= 1 && aborts.transport_errors == aborts.unexpected,
        "{aborts:?}"
    );
    assert!(
        aborts.statuses.is_empty() && aborts.timeouts == 0,
        "{aborts:?}"
    );
    assert_eq!(aborts.transport_errors, arrived("/api/v1/abort"));
    let not_modified = template(&report, "not_modified");
    assert!(not_modified.unexpected >= 1, "{not_modified:?}");
    assert_eq!(
        not_modified.statuses.get(&304),
        Some(&not_modified.unexpected)
    );
    let errors = &report.unexpected_errors;
    assert_eq!(
        errors.unexpected_statuses,
        fails.unexpected + not_modified.unexpected
    );
    assert_eq!(
        (errors.transport_errors, errors.timeouts),
        (aborts.transport_errors, hangs.timeouts)
    );
    assert_eq!(
        errors.total,
        errors.unexpected_statuses + errors.transport_errors + errors.timeouts
    );
    assert_eq!(report.json_reads.expected, 0);
}

#[test]
fn a_refused_connection_is_a_transport_error_and_leaves_the_client_without_an_account() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .expect("a free port")
        .port();
    let shape = Shape {
        clients: 1,
        accounts_per_client: 2,
        requests_per_second: 10.0,
        ramp: 0.1,
        measured: 1.2,
        hold: 0.5,
        ..Shape::default()
    };
    let accounts = account_file("refused", 2);
    let mix = vec![read_template("events", "/api/v1/events", &[200])];
    let report = run(&plan(
        &format!("http://127.0.0.1:{port}"),
        &shape,
        mix,
        &accounts,
    ))
    .expect("run");
    // The sign-in tries both accounts once, one after the other, and retires each: the client
    // never holds a token, so every slot from its sign-in on is skipped.
    assert_eq!(
        (report.refreshes.attempted, report.refreshes.succeeded),
        (2, 0)
    );
    assert_eq!(report.unexpected_errors.transport_errors, 2);
    assert_eq!(report.requests_sent, 2);
    assert!(report.skipped_slots > 0);
    assert_eq!(report.member_accounts, 0);
}

#[test]
fn the_census_counts_only_clients_with_expected_answers() {
    let failing = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3));
    let stub = StubApi::start(1, Some(failing));
    let shape = Shape {
        clients: 4,
        addresses: 2,
        requests_per_second: 40.0,
        ramp: 0.4,
        measured: 2.0,
        census: 0.5,
        ..Shape::default()
    };
    let report = run_against(
        &stub,
        "census",
        &shape,
        vec![read_template("events", "/api/v1/events", &[200])],
    );
    // Clients 1 and 3 send from 127.0.0.3, whose reads all fail; their sign-ins end in the ramp.
    assert_eq!(report.census_windows, vec![2, 2, 2, 2]);
    assert_eq!(report.minimum_concurrent_clients, 2);
    assert_eq!(report.member_accounts, 2);
    assert!(report.unexpected_errors.unexpected_statuses > 0);
}

#[test]
fn writes_stay_valid_for_each_account() {
    let stub = StubApi::start(2, None);
    let shape = Shape {
        clients: 3,
        accounts_per_client: 2,
        requests_per_second: 30.0,
        ramp: 0.3,
        measured: 2.0,
        hold: 0.8,
        ..Shape::default()
    };
    let register = "/api/v1/event-missions/{event_mission_id}/register";
    let bookmark = "/api/v1/missions/{mission_id}/bookmark";
    let mix = vec![
        write_template(
            "own_slot",
            2,
            vec![
                step(
                    HttpMethod::Post,
                    register,
                    Some(json!({ "slot_id": "{slot_id}" })),
                    &[200],
                ),
                step(HttpMethod::Delete, register, None, &[200]),
            ],
        ),
        write_template(
            "bookmark",
            2,
            vec![
                step(HttpMethod::Post, bookmark, None, &[204]),
                step(HttpMethod::Delete, bookmark, None, &[204]),
            ],
        ),
        write_template(
            "fire_mission",
            1,
            vec![step(
                HttpMethod::Post,
                "/api/v1/fire-missions",
                Some(json!({ "event_id": "{event_id}", "target_grid": "0{account_index}" })),
                &[201],
            )],
        ),
        weighted(read_template("events", "/api/v1/events", &[200]), 1),
    ];
    let report = run_against(&stub, "writes", &shape, mix);
    assert_eq!(stub.invalid_writes(), Vec::<String>::new());
    assert_eq!(
        report.unexpected_errors.total, 0,
        "{:?}",
        report.unexpected_errors
    );
    for id in ["own_slot", "bookmark", "fire_mission"] {
        let summary = template(&report, id);
        assert!(summary.statuses.values().sum::<u64>() >= 2, "{summary:?}");
    }
    assert!(report.json_writes.expected > 0);
}

#[test]
fn member_latency_never_includes_the_sign_in_while_refreshes_are_throttled() {
    let stub = StubApi::start_with(StubOptions {
        events: 10,
        failing_peer: None,
        refresh_delay: Duration::from_millis(500),
    });
    // One refresh per 100 ms and address: 100 clients over five addresses need the 2.5 s ramp,
    // the guard's margin still holds back sign-ins spaced 125 ms apart, and the stub answers each
    // 500 ms late, so the last clients receive their first token inside the measured window.
    let shape = Shape {
        clients: 100,
        addresses: 5,
        requests_per_second: 200.0,
        ramp: 2.5,
        measured: 1.5,
        all_requests: (400, 1.0),
        auth_requests: (1, 0.1),
        events: 10,
        ..Shape::default()
    };
    let mix = vec![read_template("quick", QUICK_ROUTE, &[200])];
    let report = run_against(&stub, "sign_in_latency", &shape, mix);
    assert_eq!(
        (report.refreshes.attempted, report.refreshes.succeeded),
        (100, 100)
    );
    assert!(
        report.guard_delayed_requests > 0,
        "the auth ceiling never held a sign-in back"
    );
    assert_eq!(
        report.unexpected_errors.total, 0,
        "{:?}",
        report.unexpected_errors
    );
    let reads = &report.json_reads;
    assert!(reads.expected >= 200, "{reads:?}");
    assert!(
        reads.p95_milliseconds.expect("a sample") < 100.0,
        "a member latency carries the harness's own sign-in pacing: {reads:?}"
    );
    assert!(
        report.skipped_slots > 0,
        "no slot fell between a sign-in and its token"
    );
}

#[test]
fn a_ramp_too_short_for_the_sign_ins_is_refused_before_any_request_leaves() {
    let stub = StubApi::start(1, None);
    // Four clients on one address under 20 refreshes a second need 4 / (20 × 0.8) = 0.25 s.
    let shape = Shape {
        clients: 4,
        ramp: 0.1,
        ..Shape::default()
    };
    let accounts = account_file("short_ramp", 4);
    let mix = vec![read_template("events", "/api/v1/events", &[200])];
    let error = run(&plan(&stub.origin, &shape, mix, &accounts)).expect_err("a short ramp");
    assert!(
        format!("{error:#}").contains("at least 0.25 s"),
        "{error:#}"
    );
    assert!(stub.hits().is_empty());
}

#[test]
fn prefetched_switches_leave_no_gap_and_keep_member_latency_flat() {
    let stub = StubApi::start_with(StubOptions {
        events: 10,
        failing_peer: None,
        refresh_delay: Duration::from_millis(300),
    });
    // Ten clients switch every second; each refresh takes 300 ms and starts half a hold (500 ms)
    // before its switch, so every switch finds its token ready and no member request waits on one.
    // The third switch, by 3.225 s, leaves its account two periods of the window closing at 3.45 s.
    let shape = Shape {
        clients: 10,
        addresses: 5,
        accounts_per_client: 4,
        requests_per_second: 100.0,
        ramp: 0.25,
        measured: 3.2,
        hold: 1.0,
        auth_requests: (1, 0.1),
        events: 10,
        ..Shape::default()
    };
    let mix = vec![read_template("quick", QUICK_ROUTE, &[200])];
    let report = run_against(&stub, "prefetch", &shape, mix);
    assert_eq!(
        (report.refreshes.attempted, report.refreshes.succeeded),
        (40, 40)
    );
    assert_eq!(report.late_switches, 0);
    assert_eq!(
        report.unexpected_errors.total, 0,
        "{:?}",
        report.unexpected_errors
    );
    assert_eq!(report.member_accounts, 40);
    let reads = &report.json_reads;
    assert!(reads.expected >= 200, "{reads:?}");
    assert!(
        reads.p95_milliseconds.expect("a sample") < 100.0,
        "a switch shows in the member latency: {reads:?}"
    );
    let hits = stub.hits();
    for client in 0..10 {
        let turns = [client, client + 10, client + 20, client + 30];
        assert_accounts_follow_the_switches(&hits, client, 10, Duration::from_secs(1), &turns);
        assert_no_gap(&hits, client, 10, Duration::from_millis(250));
    }
}

#[test]
fn a_prefetch_that_misses_its_switch_keeps_the_current_account_and_counts_the_switch_late() {
    let refresh_delay = Duration::from_millis(450);
    let stub = StubApi::start_with(StubOptions {
        events: 5,
        failing_peer: None,
        refresh_delay,
    });
    // Switches every 600 ms, prefetches 300 ms ahead, refreshes 450 ms long: each of the two
    // switches per client inside the window is 150 ms late.
    let shape = Shape {
        clients: 5,
        addresses: 5,
        accounts_per_client: 3,
        requests_per_second: 50.0,
        ramp: 0.125,
        measured: 1.6,
        hold: 0.6,
        auth_requests: (1, 0.1),
        events: 5,
        ..Shape::default()
    };
    let mix = vec![read_template("quick", QUICK_ROUTE, &[200])];
    let report = run_against(&stub, "late_switch", &shape, mix);
    assert_eq!(report.late_switches, 10);
    assert_eq!(
        (report.refreshes.attempted, report.refreshes.succeeded),
        (15, 15)
    );
    assert_eq!(
        report.unexpected_errors.total, 0,
        "{:?}",
        report.unexpected_errors
    );
    let hits = stub.hits();
    let hold = Duration::from_millis(600);
    for client in 0..5 {
        let refreshes = client_hits(&hits, client, 5, StubHit::is_refresh);
        assert_eq!(refreshes.len(), 3);
        let members = client_hits(&hits, client, 5, |hit| !hit.is_refresh());
        for (number, turn) in (1u32..).zip([client + 5, client + 10]) {
            let switch = refreshes[0].at + hold * number;
            let arrived = refreshes[number as usize].at + refresh_delay;
            let first_new = members
                .iter()
                .find(|hit| hit.account == Some(turn))
                .expect("the new account sends");
            assert!(
                first_new.at + Duration::from_millis(10) >= arrived,
                "client {client}: account {turn} sent before its token arrived"
            );
            let previous = if number == 1 { client } else { client + 5 };
            let last_old = members
                .iter()
                .rev()
                .find(|hit| hit.account == Some(previous))
                .expect("the old account sent");
            assert!(
                last_old.at > switch,
                "client {client}: account {previous} stopped at its switch instead of its successor's token"
            );
        }
        assert_no_gap(&hits, client, 5, Duration::from_millis(250));
    }
}
