use std::path::PathBuf;

use serde_json::{Value, json};

use super::*;
use crate::staging_verification::load_generation::sample_plans::fixture_events;

/// A file in the temporary directory, removed when dropped.
struct TemporaryFile(PathBuf);

impl TemporaryFile {
    fn with(name: &str, contents: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "tbd-account-rotation-{name}-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, contents).expect("temporary file");
        Self(path)
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn accounts(count: usize) -> Vec<AccountSecret> {
    (0..count)
        .map(|index| AccountSecret {
            discord_id: format!("91000000000000000{index:02}"),
            refresh_token: SecretToken(format!("refresh-{index}")),
        })
        .collect()
}

fn pair(access: &str, refresh: &str) -> RefreshedPair {
    RefreshedPair {
        access_token: SecretToken(access.to_owned()),
        refresh_token: SecretToken(refresh.to_owned()),
    }
}

#[test]
fn the_account_file_is_read_in_account_order() {
    let file = TemporaryFile::with(
        "ordered",
        &json!({ "accounts": [
            { "discord_id": "9100000000000000000", "refresh_token": "first-token" },
            { "discord_id": "9100000000000000001", "refresh_token": "second-token" },
        ] })
        .to_string(),
    );
    let read = read_account_file(&file.0, 2).expect("reads");
    assert_eq!(read[0].discord_id, "9100000000000000000");
    assert_eq!(read[1].refresh_token.expose(), "second-token");
}

#[test]
fn a_mismatched_or_malformed_account_file_is_refused_without_echoing_a_token() {
    let entry = |id: &str, token: &str| json!({ "discord_id": id, "refresh_token": token });
    let cases: [(Value, &str); 6] = [
        (
            json!({ "accounts": "refresh-SECRET-0" }),
            "Data error at line 1",
        ),
        (
            json!({ "accounts": [entry("9100000000000000000", "SECRET-1")], "extra": 1 }),
            "Data error",
        ),
        (
            json!({ "accounts": [entry("91000000000000000x0", "SECRET-2")] }),
            "not a decimal id",
        ),
        (
            json!({ "accounts": [entry("", "SECRET-3")] }),
            "not a decimal id",
        ),
        (
            json!({ "accounts": [entry("9100000000000000000", "")] }),
            "empty refresh_token",
        ),
        (
            json!({ "accounts": [entry("1", "SECRET-4"), entry("1", "SECRET-5")] }),
            "repeats discord id 1",
        ),
    ];
    for (index, (document, expected)) in cases.into_iter().enumerate() {
        let file = TemporaryFile::with(&format!("malformed-{index}"), &document.to_string());
        let wanted = document["accounts"].as_array().map_or(1, Vec::len);
        let error = format!(
            "{:#}",
            read_account_file(&file.0, wanted).expect_err("refused")
        );
        assert!(error.contains(expected), "{index}: {error}");
        assert!(!error.contains("SECRET"), "{index} echoes a token: {error}");
    }
    let file = TemporaryFile::with(
        "short",
        &json!({ "accounts": [entry("1", "SECRET-6")] }).to_string(),
    );
    let error = format!("{:#}", read_account_file(&file.0, 2).expect_err("refused"));
    assert!(
        error.contains("holds 1 accounts; the workload needs 2"),
        "{error}"
    );
}

#[test]
fn tokens_never_print() {
    let token = SecretToken("refresh-SECRET".to_owned());
    assert_eq!(format!("{token:?}"), "SecretToken(redacted)");
    let dealt = deal_accounts(accounts(2), &fixture_events(1, 2), 1, 1);
    let rendered = format!("{dealt:?}");
    assert!(
        !rendered.contains("refresh-0") && rendered.contains("redacted"),
        "{rendered}"
    );
}

#[test]
fn accounts_deal_by_client_stride_into_matching_rings_and_request_state() {
    let dealt = deal_accounts(accounts(6), &fixture_events(2, 3), 3, 4);
    let held: Vec<Vec<(usize, usize)>> = dealt
        .iter()
        .map(|share| {
            (0..2)
                .map(|position| {
                    (
                        share.ring.account(position).index,
                        share.members[position].index,
                    )
                })
                .collect()
        })
        .collect();
    assert_eq!(
        held,
        vec![
            vec![(0, 0), (3, 3)],
            vec![(1, 1), (4, 4)],
            vec![(2, 2), (5, 5)]
        ]
    );
    let fifth = &dealt[1].members[1];
    assert_eq!(fifth.binding.event_mission_id, "em-0");
    assert_eq!(fifth.binding.slot_id, "slot-0-2");
    assert_eq!(fifth.step_cursor(3), 0);
}

#[test]
fn a_ring_refreshes_in_order_switches_on_demand_wraps_and_never_returns_to_a_retired_account() {
    let mut ring = deal_accounts(accounts(3), &fixture_events(1, 3), 1, 1)
        .remove(0)
        .ring;
    assert_eq!((ring.current(), ring.next_candidate()), (None, Some(0)));
    let access = ring.take_refreshed(0, pair("access-0", "refresh-0-1"));
    assert_eq!(access.expose(), "access-0");
    assert_eq!(
        (ring.current(), ring.next_candidate()),
        (None, Some(1)),
        "a refresh alone switches nothing"
    );
    ring.switch_to(0);
    assert_eq!(ring.current(), Some(0));
    ring.retire(1);
    assert_eq!((ring.current(), ring.next_candidate()), (Some(0), Some(2)));
    ring.take_refreshed(2, pair("access-2", "refresh-2-1"));
    assert_eq!(
        (ring.current(), ring.next_candidate()),
        (Some(0), Some(0)),
        "a prefetched account waits for its switch, and the ring wraps past the retired one"
    );
    ring.switch_to(2);
    let again = refresh_request(ring.account(0));
    let body: Value = serde_json::from_slice(&again.body.expect("a body")).expect("JSON");
    assert_eq!(
        body,
        json!({ "refresh_token": "refresh-0-1" }),
        "the rotated token, once"
    );
    ring.retire(0);
    assert_eq!((ring.current(), ring.next_candidate()), (Some(2), Some(2)));
    ring.retire(2);
    assert_eq!((ring.current(), ring.next_candidate()), (None, None));
}

#[test]
fn the_refresh_follows_the_session_token_contract() {
    let dealt = deal_accounts(accounts(1), &fixture_events(1, 1), 1, 1);
    let request = refresh_request(dealt[0].ring.account(0));
    assert_eq!(request.method, HttpMethod::Post);
    assert_eq!(request.path, REFRESH_PATH);
    assert_eq!(request.expected_statuses, vec![200]);
    assert!(request.auth && !request.conditional);
    let complete = json!({
        "access_token": "access-1",
        "expires_at": "2099-01-01T00:00:00Z",
        "refresh_token": "refresh-1",
        "token_type": "Bearer",
    });
    let decoded = decode_refresh_answer(complete.to_string().as_bytes()).expect("decodes");
    assert_eq!(decoded.refresh_token.expose(), "refresh-1");
    let spoiled: [(&str, Value); 5] = [
        ("token_type", json!("MAC")),
        ("access_token", json!("")),
        ("refresh_token", json!(7)),
        ("expires_at", Value::Null),
        ("surplus", json!(true)),
    ];
    for (field, value) in spoiled {
        let mut answer = complete.clone();
        answer[field] = value;
        assert!(
            decode_refresh_answer(answer.to_string().as_bytes()).is_none(),
            "{field}"
        );
    }
    assert!(decode_refresh_answer(b"not json").is_none());
}
