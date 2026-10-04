//! The fleet command console: how a followed command's outcome is read and announced, the checks a
//! request passes, how a console reply reads, what a refusal is told, and the wiring of the
//! request, console and follow paths.
//!
//! The follow loop is browser-only, so the decision it makes — what a receipt's state means, and
//! which announcement it earns — lives in [`announce_receipt`], which runs here natively against a
//! recording announcer for every state the backend sends. The loop's use of it is pinned on the
//! scrubbed source, and the last test proves that pin still says no to a call parked in dead code.

use super::command_wording::*;
use frontend_api_dtos::{
    ConsoleCommandOutcome, FleetCommandList, FleetCommandReceipt, FleetCommandRequest,
};
use frontend_test_support::class_r_scrub::{live_code, only_body, only_item};
use frontend_test_support::fixtures::golden;
use frontend_transport::Error;
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

/// A console reply is shown under the followed command and under its history row, as the text the
/// server sent.
#[test]
fn a_console_reply_is_shown_under_the_panel_and_the_history_row() {
    let history = live_code(include_str!("../command_history.rs"));
    for item in ["fn followed_panel(", "fn history_row("] {
        assert!(
            compact(only_body(&history, item)).contains("{console_reply(&receipt)}"),
            "{item} must show a console reply"
        );
    }
    let reply = compact(only_body(&history, "fn console_reply("));
    assert!(reply.contains("receipt.console_outcome()?.response"));
    assert!(reply.contains("{reply}") && !reply.contains("inner_html"));
}

/// A refused cancellation names the state the command reached; an ended session says why the kick
/// cannot land; anything else keeps the backend's sentence.
#[test]
fn refusals_say_why() {
    let not_cancellable = Error::from_error_body(
        409,
        Some(
            &json!({"error": "a claimed command can no longer be cancelled", "details": {"code": "COMMAND_NOT_CANCELLABLE", "state": "claimed"}}),
        ),
    );
    assert_eq!(
        command_refusal_sentence(&not_cancellable, "fallback"),
        "The command is claimed — an executor has taken it up, so it can no longer be cancelled."
    );
    let ended = Error::from_error_body(
        409,
        Some(
            &json!({"error": "the runtime session is not this server's open session", "details": {"code": "RUNTIME_SESSION_ENDED"}}),
        ),
    );
    assert!(command_refusal_sentence(&ended, "fallback").contains("not the server's open session"));
    let inactive = Error::from_error_body(
        409,
        Some(&json!({"error": "a deactivated server accepts no commands"})),
    );
    assert_eq!(
        command_refusal_sentence(&inactive, "fallback"),
        "A deactivated server accepts no commands"
    );
}

/// Source with every whitespace run and every trailing comma before a closing parenthesis removed,
/// so a pin survives the formatter's line breaks.
fn compact(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .replace(",)", ")")
}

/// Every `FleetCommandRequest::<constructor>(` call in `src`, by constructor name.
fn requested_constructors(src: &str) -> std::collections::BTreeSet<String> {
    let prefix = "FleetCommandRequest::";
    src.match_indices(prefix)
        .filter_map(|(at, _)| {
            let rest = &src[at + prefix.len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            rest[name.len()..].starts_with('(').then_some(name)
        })
        .collect()
}

/// The console offers the seven operator actions, each built by its request constructor — the
/// console line from the checked line, never from the raw field — and names the two a deployment
/// issues without offering them.
#[test]
fn the_console_offers_exactly_the_operator_actions() {
    let requests = compact(&live_code(include_str!("../command_requests.rs")));
    let console_box = compact(&live_code(include_str!("../console_command_form.rs")));
    let src = format!("{requests}{console_box}");
    assert_eq!(
        requested_constructors(&src),
        [
            "broadcast",
            "console_command",
            "kick",
            "list_players",
            "restart",
            "start",
            "stop"
        ]
        .map(String::from)
        .into()
    );
    for constructor in [
        "FleetCommandRequest::start()",
        "FleetCommandRequest::stop()",
        "FleetCommandRequest::restart()",
        "FleetCommandRequest::list_players()",
        "FleetCommandRequest::broadcast(&text)",
        "FleetCommandRequest::kick(&identity.as_str().into(), &runtime_session.as_str().into(), why.as_deref())",
        "FleetCommandRequest::console_command(&checked)",
    ] {
        assert!(
            src.contains(&compact(constructor)),
            "the console must request {constructor}"
        );
    }
    assert!(requests.contains("{console_command_form(console)}"));
    assert!(console_box.contains("ev.prevent_default()"));
    assert!(console_box.contains(&compact("if console.busy.get_untracked() { return; }")));
    let checked = console_box
        .find(&compact(
            "match validated_console_line(&line.get_untracked()) { Ok(checked) => {",
        ))
        .expect("the console box sends only a checked line");
    let cleared = console_box
        .find(&compact("line.set(String::new())"))
        .expect("a sent line leaves the field");
    let sent = console_box
        .find(&compact(
            "console.request(FleetCommandRequest::console_command(&checked))",
        ))
        .expect("the console box requests the checked line");
    assert!(checked < cleared && cleared < sent);
    assert!(!src.contains("load_mission") && !src.contains("restart_with_mission"));
    assert_eq!(
        serde_json::to_value(FleetCommandRequest::list_players()).unwrap(),
        json!({"action": "list_players"})
    );
    assert_eq!(
        serde_json::to_value(FleetCommandRequest::console_command("#players")).unwrap(),
        json!({"action": "console_command", "arguments": {"line": "#players"}})
    );
    assert_eq!(action_label("console_command"), "Console command");
    assert!(action_label("load_mission").contains("issued by a deployment"));
    assert!(action_label("restart_with_mission").contains("issued by a deployment"));
}

/// The console's scrubbed source.
fn console_source() -> String {
    live_code(include_str!("../mod.rs"))
}

/// A 202 is an acceptance: the request path says so and follows the receipt, and only the follow
/// announces an outcome — through [`announce_receipt`], after reading the receipt again.
#[test]
fn an_acceptance_is_followed_to_its_outcome() {
    let src = console_source();
    let request = compact(only_body(&src, "pub(super) fn request("));
    assert!(request.contains(&compact(
        "request_fleet_command(self.store, &server.as_str().into(), &request)"
    )));
    assert!(request.contains("accepted_line(&receipt)"));
    assert!(request.contains("self.follow(id)"));
    assert!(
        !request.contains(".success(") && !request.contains("announce_receipt("),
        "an acceptance must not be announced as an outcome"
    );
    let follow = compact(only_body(&src, "pub(super) fn follow("));
    let read = follow
        .find(&compact(
            "load_fleet_command(self.store, &server.as_str().into(), &command_id)",
        ))
        .expect("the follow reads the receipt again");
    let announce = follow
        .find(&compact("announce_receipt(&receipt, &self.toasts)"))
        .expect("the follow announces through announce_receipt");
    assert!(read < announce);
    assert!(follow.contains("TimeoutFuture::new(FOLLOW_INTERVAL_MS)"));
    assert!(follow.contains(&compact("try_get_value() != Some(generation)")));
    let text = accepted_line(&in_state("queued"));
    assert_eq!(
        text,
        "List players accepted — waiting for the host agent to carry it out"
    );
}

/// The follow pin above can still say no: a call to `announce_receipt` parked in dead code, or a
/// shadow copy of the follow, is not what the scrubbed source shows.
#[test]
fn the_follow_pin_rejects_every_dead_code_wrapper() {
    let needle = "announce_receipt(&receipt, &self.toasts)";
    let attacks: [(&str, String); 10] = [
        (
            "if true == false",
            format!("if true == false {{ {needle}; }}"),
        ),
        ("loop { break; … }", format!("loop {{ break; {needle}; }}")),
        (
            "#[cfg(any())]",
            format!("#[cfg(any())] fn d() {{ {needle}; }}"),
        ),
        ("while false", format!("while false {{ {needle}; }}")),
        ("if !true", format!("if !true {{ {needle}; }}")),
        (
            "const C: bool = false; if C",
            format!("const C: bool = false;\nfn d() {{ if C {{ {needle}; }} }}"),
        ),
        ("return; above", format!("fn d() {{ return; {needle}; }}")),
        (
            "match guard",
            format!("match () {{ _ if false => {{ {needle}; }} _ => {{}} }}"),
        ),
        ("comment", format!("// {needle}")),
        ("string", format!("let _ = \"{needle}\";")),
    ];
    for (label, body) in attacks {
        let forged = format!("pub(super) fn follow(\n) {{\n    {body}\n}}\n#[cfg(test)]\n");
        assert!(
            !live_code(&forged).contains(needle),
            "{label}: the needle survived scrubbing"
        );
    }
    let shadow = "pub(super) fn follow() { good; }\nmod real { pub(super) fn follow() { bad; } }\n#[cfg(test)]\n";
    let scrubbed = live_code(shadow);
    assert!(
        std::panic::catch_unwind(|| only_item(&scrubbed, "pub(super) fn follow(")).is_err(),
        "two definitions must be refused rather than one picked"
    );
    let live = format!("pub(super) fn follow() {{\n    {needle};\n}}\n#[cfg(test)]\n");
    assert!(live_code(&live).contains(needle));
}
