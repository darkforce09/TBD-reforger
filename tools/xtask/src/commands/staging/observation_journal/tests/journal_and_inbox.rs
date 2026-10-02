//! The journal keeps one line and one content-addressed artifact per observation and never
//! reuses a run's file; the inbox accepts an entry only inside its window and never one that
//! carries a credential.
use serde_json::{Value, json};

use super::browser_inbox::{BrowserInbox, InboxRead, InboxWindow};
use super::journal::{ARTIFACT_FOLDER, JOURNAL_FILE, JournalEntry, ObservationJournal, digest};
use crate::commands::staging::procedure_runner::runner_support::scratch_folder;

fn entry<'a>(step: &'a str, artifact: &'a [u8]) -> JournalEntry<'a> {
    JournalEntry {
        step,
        observer: "database",
        observed_unix_ms: 1_800_000_000_000,
        summary: "row found",
        verdict: "satisfied",
        artifact,
    }
}

#[test]
fn staging_journal_archives_every_observation_by_its_digest() {
    let folder = scratch_folder("journal");
    let mut journal = ObservationJournal::create(&folder).unwrap();
    let first = journal
        .archive(&entry("w1_stop.server1", b"stopped\n"))
        .unwrap();
    let again = journal
        .archive(&entry("w1_stop.server2", b"stopped\n"))
        .unwrap();
    let other = journal.archive(&entry("w1_stop.request", b"")).unwrap();
    assert_eq!(first, digest(b"stopped\n"));
    assert_eq!(first, again, "identical bytes share one artifact");
    assert_eq!(
        other,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    let stored = std::fs::read(folder.join(ARTIFACT_FOLDER).join(format!("{first}.txt"))).unwrap();
    assert_eq!(
        digest(&stored),
        first,
        "an artifact holds exactly the bytes its name digests"
    );
    let lines: Vec<Value> = std::fs::read_to_string(folder.join(JOURNAL_FILE))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 3, "one line per observation");
    assert_eq!(lines[1]["sequence"], 2);
    assert_eq!(lines[1]["step"], "w1_stop.server2");
    assert_eq!(lines[1]["sha256"], json!(first));
    assert_eq!(
        lines[1]["artifact"],
        json!(format!("artifacts/{first}.txt"))
    );
    assert!(
        ObservationJournal::create(&folder).is_err(),
        "a run folder is never reused"
    );
}

fn inbox_with(step: &str, contents: &str) -> BrowserInbox {
    let folder = scratch_folder("inbox");
    std::fs::write(folder.join(format!("{step}.json")), contents).unwrap();
    BrowserInbox::new(&folder)
}

const WINDOW: InboxWindow = InboxWindow {
    opens_unix_ms: 1_000,
    closes_unix_ms: 2_000,
};

#[test]
fn staging_inbox_accepts_an_entry_only_inside_its_step_window() {
    let absent = BrowserInbox::new(&scratch_folder("inbox-empty"));
    assert_eq!(absent.read("w4", WINDOW).unwrap(), InboxRead::Absent);
    for (captured, inside) in [(999, false), (1_000, true), (2_000, true), (2_001, false)] {
        let raw = json!({ "captured_at_unix_ms": captured, "output": "Membership data is stale" })
            .to_string();
        let read = inbox_with("w4", &raw).read("w4", WINDOW).unwrap();
        if inside {
            assert_eq!(
                read,
                InboxRead::Accepted {
                    captured_at_unix_ms: captured,
                    text: "Membership data is stale".into(),
                    raw: raw.clone(),
                }
            );
        } else {
            assert_eq!(
                read,
                InboxRead::Outside {
                    captured_at_unix_ms: captured
                }
            );
        }
    }
    let structured = json!({ "captured_at_unix_ms": 1_500, "output": { "status": 200, "body": { "role": "admin" } } });
    let InboxRead::Accepted { text, .. } = inbox_with("w5", &structured.to_string())
        .read("w5", WINDOW)
        .unwrap()
    else {
        panic!("a JSON tool output is accepted as JSON text");
    };
    assert_eq!(
        serde_json::from_str::<Value>(&text).unwrap()["body"]["role"],
        "admin"
    );
}

#[test]
fn staging_inbox_refuses_credentials_and_malformed_entries() {
    for output in [
        json!({ "headers": { "Authorization": "Bearer abc" } }),
        json!({ "body": { "access_token": "abc" } }),
        json!({ "body": { "refresh_token": "abc" } }),
        json!({ "headers": { "set-cookie": "tbd_session=abc" } }),
        json!({ "token": "abc" }),
    ] {
        let raw = json!({ "captured_at_unix_ms": 1_500, "output": output }).to_string();
        let InboxRead::Refused(why) = inbox_with("w5", &raw).read("w5", WINDOW).unwrap() else {
            panic!("{raw} must be refused");
        };
        assert!(why.contains("never reads a credential"), "{why}");
    }
    for raw in [
        "not json",
        "{\"captured_at_unix_ms\": 1500}",
        "{\"captured_at_unix_ms\": 1500, \"output\": 1, \"extra\": 2}",
    ] {
        assert!(
            matches!(
                inbox_with("w5", raw).read("w5", WINDOW).unwrap(),
                InboxRead::Refused(_)
            ),
            "{raw}"
        );
    }
}
