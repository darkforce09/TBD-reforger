use super::*;

fn milliseconds(count: u64) -> Duration {
    Duration::from_millis(count)
}

fn ascending(count: u64) -> Vec<Duration> {
    (1..=count).map(milliseconds).collect()
}

fn record(
    class: RequestClass,
    scheduled: u64,
    finished: u64,
    outcome: RequestOutcome,
) -> RequestRecord {
    RequestRecord {
        client: 0,
        address: 0,
        account: 0,
        class,
        template: Some(0),
        scheduled: milliseconds(scheduled),
        sent: milliseconds(scheduled),
        finished: milliseconds(finished),
        outcome,
        auth: false,
    }
}

#[test]
fn nearest_rank_takes_the_value_at_rank_ceiling_of_p_times_n() {
    assert_eq!(nearest_rank(&[], 95), None);
    assert_eq!(nearest_rank(&ascending(1), 95), Some(milliseconds(1)));
    assert_eq!(nearest_rank(&ascending(19), 95), Some(milliseconds(19)));
    assert_eq!(nearest_rank(&ascending(20), 95), Some(milliseconds(19)));
    assert_eq!(nearest_rank(&ascending(100), 95), Some(milliseconds(95)));
    assert_eq!(nearest_rank(&ascending(101), 95), Some(milliseconds(96)));
    assert_eq!(nearest_rank(&ascending(1000), 95), Some(milliseconds(950)));
    assert_eq!(nearest_rank(&ascending(4), 50), Some(milliseconds(2)));
    assert_eq!(nearest_rank(&ascending(5), 50), Some(milliseconds(3)));
}

#[test]
fn a_summary_sorts_its_sample_and_keeps_expected_answers_inside_the_window() {
    let ok = RequestOutcome::Expected { status: 200 };
    let mut records: Vec<RequestRecord> = (0..20u64)
        .rev()
        .map(|index| record(RequestClass::JsonRead, 1000, 1100 + 10 * index, ok))
        .collect();
    records.push(record(
        RequestClass::JsonRead,
        1000,
        9000,
        RequestOutcome::UnexpectedStatus { status: 500 },
    ));
    records.push(record(RequestClass::JsonRead, 0, 9500, ok));
    records.push(record(RequestClass::JsonWrite, 1000, 5000, ok));
    records.push(record(RequestClass::JsonRead, 100, 900, ok));
    let window = milliseconds(1000)..milliseconds(10_000);
    let reads = summarise(&records, RequestClass::JsonRead, &window);
    // Latencies 100..=290 ms in steps of 10 plus the 9500 ms one: 21 samples.
    assert_eq!(reads.samples, 21);
    assert_eq!(reads.p95, Some(milliseconds(290)));
    assert_eq!(reads.p50, Some(milliseconds(200)));
    assert_eq!(reads.max, Some(milliseconds(9500)));
    let writes = summarise(&records, RequestClass::JsonWrite, &window);
    assert_eq!((writes.samples, writes.p95), (1, Some(milliseconds(4000))));
    assert_eq!(
        summarise(&records, RequestClass::Session, &window),
        LatencySummary::default()
    );
}

#[test]
fn outcomes_classify_against_the_expected_statuses() {
    assert_eq!(
        RequestOutcome::classify(304, &[200, 304]),
        RequestOutcome::Expected { status: 304 }
    );
    assert_eq!(
        RequestOutcome::classify(304, &[200]),
        RequestOutcome::UnexpectedStatus { status: 304 }
    );
    assert!(!RequestOutcome::UnexpectedStatus { status: 500 }.is_expected());
    assert_eq!(
        RequestOutcome::UndecodableSessionAnswer { status: 200 }.status(),
        Some(200)
    );
    assert_eq!(RequestOutcome::Timeout.status(), None);
    assert_eq!(RequestOutcome::TransportError.status(), None);
    let late = record(RequestClass::JsonRead, 500, 400, RequestOutcome::Timeout);
    assert_eq!(late.latency(), Duration::ZERO);
}
