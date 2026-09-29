use super::*;
use crate::staging_verification::load_generation::latency_recording::RequestOutcome;

const OK: RequestOutcome = RequestOutcome::Expected { status: 200 };
const FAILED: RequestOutcome = RequestOutcome::UnexpectedStatus { status: 503 };

fn record(
    client: u32,
    account: usize,
    class: RequestClass,
    finished: u64,
    outcome: RequestOutcome,
) -> RequestRecord {
    RequestRecord {
        client,
        address: 0,
        account,
        class,
        template: None,
        scheduled: Duration::from_millis(finished),
        sent: Duration::from_millis(finished),
        finished: Duration::from_millis(finished),
        outcome,
        auth: class == RequestClass::Session,
    }
}

#[test]
fn each_whole_window_counts_the_distinct_clients_with_an_expected_answer() {
    let read = RequestClass::JsonRead;
    let records = vec![
        record(0, 0, read, 1000, OK),
        record(0, 0, read, 1500, OK),
        record(0, 0, read, 2999, OK),
        record(0, 0, read, 3000, OK),
        record(1, 1, read, 1999, OK),
        record(1, 1, RequestClass::Session, 2000, OK),
        record(2, 2, read, 3100, FAILED),
        record(3, 3, read, 4200, OK),
        record(4, 4, read, 500, OK),
    ];
    let measured = Duration::from_millis(1000)..Duration::from_millis(4500);
    let counted = census(&records, &measured, Duration::from_secs(1));
    // Three whole windows; the partial one from 4000 ms is left out, as is the answer before
    // the window and the failed one.
    assert_eq!(counted.windows, vec![2, 2, 1]);
    assert_eq!(counted.minimum_concurrent_clients, 1);
}

#[test]
fn a_window_too_short_for_one_census_window_counts_zero() {
    let records = vec![record(0, 0, RequestClass::JsonRead, 1200, OK)];
    let measured = Duration::from_millis(1000)..Duration::from_millis(1500);
    let counted = census(&records, &measured, Duration::from_secs(1));
    assert_eq!(
        counted,
        Census {
            windows: vec![],
            minimum_concurrent_clients: 0
        }
    );
}

#[test]
fn a_member_account_refreshed_and_received_an_expected_json_answer() {
    let session = RequestClass::Session;
    let records = vec![
        record(0, 1, session, 100, OK),
        record(0, 1, RequestClass::JsonRead, 200, OK),
        record(0, 2, session, 300, OK),
        record(0, 2, RequestClass::JsonRead, 400, FAILED),
        record(1, 3, session, 100, FAILED),
        record(1, 3, RequestClass::JsonRead, 200, OK),
        record(1, 4, session, 300, OK),
        record(1, 4, RequestClass::JsonWrite, 400, OK),
        record(1, 5, session, 500, OK),
    ];
    assert_eq!(member_accounts(&records), 2);
}
