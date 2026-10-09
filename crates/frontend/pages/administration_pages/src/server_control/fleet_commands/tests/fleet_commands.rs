//! The fleet command console: how a followed command's outcome is read and announced, the checks a
//! request passes and how a console reply reads.
//!
//! The follow loop is browser-only, so the decision it makes — what a receipt's state means, and
//! which announcement it earns — lives in [`announce_receipt`], which runs here natively against a
//! recording announcer for every state the backend sends.

use super::command_wording::*;
use frontend_api_dtos::{
    ConsoleCommandOutcome, FleetCommandList, FleetCommandReceipt, FleetCommandRequest,
};
use frontend_test_support::fixtures::golden;
use serde_json::json;
use std::cell::RefCell;

fn captured() -> Vec<FleetCommandReceipt> {
    serde_json::from_str::<FleetCommandList>(golden!(
        "GET__servers__00000000-0000-4000-d000-000000000001__commands.json"
    ))
    .unwrap()
    .items
}

/// The captured receipt in `state`.
fn in_state(state: &str) -> FleetCommandReceipt {
    captured()
        .into_iter()
        .find(|r| r.state == state)
        .unwrap_or_else(|| panic!("no captured {state} receipt"))
}

/// The captured failed broadcast, moved into `state`.
fn moved_to(state: &str) -> FleetCommandReceipt {
    let mut receipt = in_state("failed");
    receipt.state = state.into();
    receipt.failure_reason = None;
    receipt
}

/// The captured console command, as the API accepted it.
fn captured_console_command() -> FleetCommandReceipt {
    serde_json::from_str(golden!(
        "POST__servers__00000000-0000-4000-d000-000000000001__commands-console-command.json"
    ))
    .unwrap()
}

/// The captured console command, succeeded with `outcome` as what the host agent observed.
fn console_replied(outcome: serde_json::Value) -> FleetCommandReceipt {
    let mut receipt = captured_console_command();
    receipt.state = "succeeded".into();
    receipt.outcome = outcome.as_object().cloned();
    receipt
}

/// A stand-in announcer that records what it was asked to say, and how.
#[derive(Default)]
struct Recorder(RefCell<Vec<(&'static str, String)>>);

impl OutcomeAnnouncer for Recorder {
    fn succeeded(&self, text: String) {
        self.0.borrow_mut().push(("succeeded", text));
    }
    fn failed(&self, text: String) {
        self.0.borrow_mut().push(("failed", text));
    }
    fn noted(&self, text: String) {
        self.0.borrow_mut().push(("noted", text));
    }
}

/// Every state earns exactly the announcement it means, once — and a command still on its way
/// earns none, however good its acceptance looked.
#[test]
fn every_state_earns_exactly_its_announcement() {
    for (receipt, announced) in [
        (in_state("queued"), None),
        (moved_to("claimed"), None),
        (moved_to("executing"), None),
        (in_state("succeeded"), Some("succeeded")),
        (in_state("failed"), Some("failed")),
        (in_state("expired"), Some("failed")),
        (moved_to("cancelled"), Some("noted")),
        (moved_to("indeterminate"), Some("noted")),
        (moved_to("rolled_back"), Some("noted")),
    ] {
        let recorder = Recorder::default();
        let spoke = announce_receipt(&receipt, &recorder);
        let said = recorder.0.into_inner();
        match announced {
            None => {
                assert!(
                    !spoke && said.is_empty(),
                    "{} must not be announced: {said:?}",
                    receipt.state
                );
            }
            Some(kind) => {
                assert!(spoke, "{} must be announced", receipt.state);
                assert_eq!(said.len(), 1, "{} announced {said:?}", receipt.state);
                assert_eq!(
                    said[0].0, kind,
                    "{} was announced as {:?}",
                    receipt.state, said[0]
                );
            }
        }
    }
}

/// An indeterminate command is said to have an unknown outcome — never a success, never a failure —
/// and the operator is told nothing repeats it.
#[test]
fn an_indeterminate_outcome_is_said_to_be_unknown() {
    let Some(ReceiptOutcome::Unknown(text)) = receipt_outcome(&moved_to("indeterminate")) else {
        panic!("an indeterminate command must read as unknown");
    };
    assert!(text.contains("the outcome is unknown"), "{text}");
    assert!(text.contains("nothing repeats it"), "{text}");
    assert!(text.contains("Inspect the server"), "{text}");
    assert!(
        !text.contains("succeeded") && !text.contains("failed"),
        "{text}"
    );
    assert_eq!(state_label("indeterminate"), "Outcome unknown");
    assert_eq!(state_tone("indeterminate"), "warning");
}

/// The captured outcomes read as what the executor observed and what went wrong.
#[test]
fn captured_outcomes_read_as_observed() {
    assert_eq!(
        receipt_outcome(&in_state("succeeded")),
        Some(ReceiptOutcome::Succeeded(
            "Restart with mission (issued by a deployment) succeeded — config_path: \
             /srv/reforger/server-config.json · scenario_id: \
             {1111222233334444}Missions/TBD_Arland.conf · unit_active_state: active"
                .into()
        ))
    );
    assert_eq!(
        receipt_outcome(&in_state("failed")),
        Some(ReceiptOutcome::Failed(
            "Broadcast failed: the chat channel was unavailable".into()
        ))
    );
    assert_eq!(
        receipt_outcome(&in_state("expired")),
        Some(ReceiptOutcome::Failed(
            "Restart with mission (issued by a deployment) expired — no executor carried it out \
             before 2026-07-25 09:05 UTC, so nothing ran"
                .into()
        ))
    );
    assert_eq!(
        arguments_summary(&in_state("failed")),
        "message: Server restarts in 10 minutes for Operation Iron Veil"
    );
    assert_eq!(arguments_summary(&in_state("queued")), "—");
}

/// A player listing names its players and says when part of the response was not understood; the
/// kick form offers the players of the newest successful listing.
#[test]
fn a_player_listing_names_its_players() {
    let mut listing = in_state("queued");
    listing.state = "succeeded".into();
    listing.outcome = serde_json::from_value(json!({
        "players": [
            {"player_id": 0, "arma_id": "uid-a", "name": "Vance"},
            {"player_id": 1, "arma_id": "uid-b", "name": "Rhodes"}
        ],
        "raw_lines": ["Players on server:", "garbled"]
    }))
    .ok();
    assert_eq!(
        listed_players(&listing),
        vec![
            ("uid-a".to_string(), "Vance".to_string()),
            ("uid-b".to_string(), "Rhodes".to_string())
        ]
    );
    assert_eq!(
        outcome_summary(&listing).as_deref(),
        Some(
            "2 player(s): Vance (uid-a), Rhodes (uid-b) · 2 line(s) of the response were not understood"
        )
    );
    let mut receipts = captured();
    assert!(
        latest_player_listing(&receipts).is_none(),
        "the capture's listing is still queued"
    );
    receipts.insert(1, listing);
    assert_eq!(
        latest_player_listing(&receipts).map(|r| r.id.as_str()),
        Some("00000000-0000-4000-f200-000000000003")
    );
}

/// A broadcast and a kick are checked as the backend checks them.
#[test]
fn requests_are_checked_as_the_backend_checks_them() {
    assert_eq!(
        validated_broadcast("  Restart in 5  "),
        Ok("Restart in 5".to_string())
    );
    assert!(validated_broadcast("   ").is_err());
    assert!(validated_broadcast(&"x".repeat(257)).is_err());
    assert!(validated_broadcast(&"x".repeat(256)).is_ok());
    assert!(validated_broadcast("two\nlines").is_err());
    let session = "00000000-0000-4000-f300-000000000001";
    assert_eq!(
        validated_kick(" uid-a ", session, ""),
        Ok(("uid-a".to_string(), session.to_string(), None))
    );
    assert_eq!(
        validated_kick("uid-a", session, " AFK "),
        Ok((
            "uid-a".to_string(),
            session.to_string(),
            Some("AFK".to_string())
        ))
    );
    assert!(validated_kick("", session, "").is_err());
    assert!(validated_kick("uid-a", "not-a-session", "").is_err());
    assert!(validated_kick("uid-a", session, &"r".repeat(129)).is_err());
    assert!(is_uuid(session) && !is_uuid("00000000000040000f3000000000000001"));
}

/// A console line is checked as the API's argument gate checks it, case for case: what was typed
/// holds no control character and no line or paragraph separator, and once trimmed it holds 1 to
/// 256 bytes and does not start with `@`. The trimmed line is what is sent.
#[test]
fn a_console_line_is_checked_as_the_backend_checks_it() {
    for (typed, sent) in [
        ("#players", "#players"),
        ("  #players  ", "#players"),
        ("say -1 meet @ the gate", "say -1 meet @ the gate"),
    ] {
        assert_eq!(
            validated_console_line(typed),
            Ok(sent.to_string()),
            "{typed:?}"
        );
    }
    for line in ["p".repeat(256), "é".repeat(128)] {
        assert_eq!(validated_console_line(&line), Ok(line.clone()));
    }
    for typed in [
        String::new(),
        "   ".into(),
        "p".repeat(257),
        "é".repeat(129),
        "#players\n#shutdown".into(),
        "#players\n".into(),
        "#players\r".into(),
        "#players\t#shutdown".into(),
        "#players\u{7}".into(),
        "#players\u{85}".into(),
        "#players\u{2028}#shutdown".into(),
        "#players\u{2029}".into(),
        "@logout".into(),
        "  @logout".into(),
    ] {
        assert!(
            validated_console_line(&typed).is_err(),
            "{typed:?} must be refused"
        );
    }
    for (typed, why) in [
        (
            "#players\n",
            "The console line must be one line, without line breaks or control characters",
        ),
        ("   ", "Enter the console line"),
        (
            "  @logout",
            "The console line cannot start with @: those commands act on the host agent's own \
             RCON session",
        ),
    ] {
        assert_eq!(validated_console_line(typed), Err(why.to_string()));
    }
    assert_eq!(
        validated_console_line(&"p".repeat(257)),
        Err("The console line is too long: at most 256 bytes".to_string())
    );
    assert_eq!(FleetCommandRequest::CONSOLE_LINE_MAX_BYTES, 256);
}

/// A console command is accepted as waiting for the host agent, and its reply reads as the server
/// sent it: several lines counted, one line quoted, an empty reply named, a cut reply said to be
/// cut; a line the server never answered fails with the host agent's words.
#[test]
fn a_console_reply_reads_as_the_server_sent_it() {
    let accepted = captured_console_command();
    assert_eq!(
        accepted_line(&accepted),
        "Console command accepted — waiting for the host agent to carry it out"
    );
    assert_eq!(arguments_summary(&accepted), "line: #players");
    assert_eq!(receipt_outcome(&accepted), None);
    let players = console_replied(json!({
        "response": "Players on server:\n[#] [IP Address]:[Port] [Ping] [GUID] [Name]\n\
                     ------------------------------\n0   198.51.100.7:2001   38   0f1e2d3c   Vance\n\
                     (1 players in total)\n",
        "response_truncated": false
    }));
    assert_eq!(
        receipt_outcome(&players),
        Some(ReceiptOutcome::Succeeded(
            "Console command succeeded — the server replied with 5 lines".into()
        ))
    );
    let recorder = Recorder::default();
    assert!(announce_receipt(&players, &recorder));
    assert_eq!(recorder.0.into_inner()[0].0, "succeeded");
    let one =
        console_replied(json!({"response": "  Player kicked  \n", "response_truncated": false}));
    assert_eq!(
        outcome_summary(&one).as_deref(),
        Some("the server replied \"Player kicked\"")
    );
    let empty = console_replied(json!({"response": "", "response_truncated": false}));
    assert_eq!(
        outcome_summary(&empty).as_deref(),
        Some("the server's reply was empty")
    );
    let long = "y".repeat(ConsoleCommandOutcome::RESPONSE_MAX_BYTES);
    let cut = console_replied(json!({"response": long, "response_truncated": true}));
    assert_eq!(
        outcome_summary(&cut),
        Some(format!(
            "the server replied \"{}…\"; the host agent kept only its first 4096 bytes",
            "y".repeat(120)
        ))
    );
    let mut unanswered = captured_console_command();
    unanswered.state = "failed".into();
    unanswered.failure_reason =
        Some("no RCON response; the command may or may not have run".into());
    assert_eq!(
        receipt_outcome(&unanswered),
        Some(ReceiptOutcome::Failed(
            "Console command failed: no RCON response; the command may or may not have run".into()
        ))
    );
    let other_shape = console_replied(json!({"reply": "x"}));
    assert_eq!(outcome_summary(&other_shape).as_deref(), Some("reply: x"));
}
