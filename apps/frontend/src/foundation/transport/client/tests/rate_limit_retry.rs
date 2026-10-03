//! The rate-limit retry: which answers wait and retry, how long they wait, and how many sends one
//! call makes.

use super::*;
use futures::executor::block_on;
use std::cell::RefCell;

/// Runs the loop over `statuses` (one per send, each with its `Retry-After`), returning the
/// final status, the number of sends and the waits slept.
fn run(statuses: &[(u16, Option<&str>)]) -> (Result<u16, String>, usize, Vec<u32>) {
    let sent = RefCell::new(0usize);
    let slept = RefCell::new(Vec::new());
    let result = block_on(send_with_rate_limit_retry(
        || {
            let index = *sent.borrow();
            *sent.borrow_mut() += 1;
            let answer = statuses[index.min(statuses.len() - 1)];
            async move { Ok::<_, String>(answer) }
        },
        |answer: &(u16, Option<&str>)| (answer.0, answer.1.map(str::to_string)),
        |seconds| {
            slept.borrow_mut().push(seconds);
            async {}
        },
    ))
    .map(|answer| answer.0);
    (result, sent.into_inner(), slept.into_inner())
}

#[test]
fn rate_limit_retry_waits_out_a_429_and_returns_the_next_answer() {
    let (result, sends, waits) = run(&[(429, Some("5")), (200, None)]);
    assert_eq!(result, Ok(200));
    assert_eq!(sends, 2);
    assert_eq!(waits, vec![5]);
}

#[test]
fn rate_limit_retry_stops_after_three_sends_and_returns_the_last_429() {
    let (result, sends, waits) = run(&[(429, None), (429, Some("1")), (429, Some("9"))]);
    assert_eq!(result, Ok(429));
    assert_eq!(sends, RATE_LIMIT_ATTEMPTS as usize);
    assert_eq!(waits, vec![DEFAULT_RETRY_AFTER_S, 1]);
}

#[test]
fn rate_limit_retry_never_retries_another_status() {
    for status in [200, 304, 400, 404, 500, 503] {
        let (result, sends, waits) = run(&[(status, Some("1"))]);
        assert_eq!(result, Ok(status));
        assert_eq!(sends, 1, "status {status}");
        assert!(waits.is_empty(), "status {status}");
    }
}

#[test]
fn rate_limit_retry_reads_retry_after_with_a_default_and_a_ceiling() {
    assert_eq!(rate_limit_wait_s(429, 0, Some("7")), Some(7));
    assert_eq!(rate_limit_wait_s(429, 0, Some(" 7 ")), Some(7));
    assert_eq!(rate_limit_wait_s(429, 1, None), Some(DEFAULT_RETRY_AFTER_S));
    assert_eq!(
        rate_limit_wait_s(429, 0, Some("Wed, 21 Oct 2026 07:28:00 GMT")),
        Some(DEFAULT_RETRY_AFTER_S)
    );
    assert_eq!(
        rate_limit_wait_s(429, 0, Some("3600")),
        Some(MAX_RETRY_AFTER_S)
    );
    assert_eq!(
        rate_limit_wait_s(429, RATE_LIMIT_ATTEMPTS - 1, Some("1")),
        None
    );
}

#[test]
fn rate_limit_retry_ends_at_a_send_failure() {
    let sends = RefCell::new(0usize);
    let result = block_on(send_with_rate_limit_retry(
        || {
            *sends.borrow_mut() += 1;
            async { Err::<u16, String>("offline".into()) }
        },
        |status: &u16| (*status, None),
        |_| async {},
    ));
    assert_eq!(result, Err("offline".to_string()));
    assert_eq!(sends.into_inner(), 1);
}
