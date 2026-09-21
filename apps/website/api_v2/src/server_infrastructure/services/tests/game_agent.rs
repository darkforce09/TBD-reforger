use super::*;

/// The template that owns the other end of this protocol. Pinning against the real file (not a
/// copy) is what makes the dwell relationship below a live constraint.
///
/// This is an `include_str!`, so it is a COMPILE-TIME dependency across a crate boundary —
/// deleting or moving the template breaks `website-api`, which no `cargo test -p xtask` would
/// ever catch. It is deliberately not a cargo dependency (`website-api` does not depend on
/// `xtask`): the pin exists precisely because the two sides are otherwise unconnected.
const RENDERED_AGENT_TEMPLATE: &str =
    include_str!("../../../../../../../tools_v2/xtask/src/commands/deploy/staging/agent.rs");

#[test]
fn verbs_are_the_agents_four_literals() {
    assert_eq!(AgentAction::Status.verb(), "status");
    assert_eq!(AgentAction::Start.verb(), "start");
    assert_eq!(AgentAction::Stop.verb(), "stop");
    assert_eq!(AgentAction::Restart.verb(), "restart");
    assert_eq!(AgentAction::Restart.to_string(), "restart");
    // And the agent must still accept exactly those four. If its case arm is edited, this
    // client is speaking to a protocol that no longer exists.
    assert!(
        RENDERED_AGENT_TEMPLATE.contains(r#"status|start|stop|restart) ACTION="$candidate" ;;"#),
        "the rendered agent no longer matches exactly the four verbs this client sends"
    );
}

/// **The timeout must exceed the dwell.**
///
/// Read the dwell out of the agent template rather than hardcoding 8 here, because the failure
/// this guards against is somebody *raising the dwell* and leaving this client timing out under
/// it. A hardcoded 8 would keep passing while every honest slow answer started reading as
/// `unreachable` — a check that looked at the wrong thing.
#[test]
fn timeout_must_exceed_the_agents_dwell() {
    // The template declares the default twice: `: "${TBD_AGENT_DWELL_S:=8}"` as the script's own
    // env default, and `DWELL="${TBD_AGENT_DWELL_S:-8}"` inside the agent it renders. The second
    // is the one that actually governs how long the agent sleeps before answering, so it is the
    // one this client must not time out under — read it, not the first.
    let marker = "TBD_AGENT_DWELL_S:-";
    let at = RENDERED_AGENT_TEMPLATE
        .find(marker)
        .expect("the rendered agent must still declare TBD_AGENT_DWELL_S");
    let rest = &RENDERED_AGENT_TEMPLATE[at + marker.len()..];
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let dwell: u64 = digits
        .parse()
        .unwrap_or_else(|_| panic!("TBD_AGENT_DWELL_S default is not a number: {digits:?}"));

    assert!(
        dwell > 0,
        "a zero dwell would mean the agent stopped waiting for the unit to prove it stays up"
    );
    assert!(
        AGENT_TIMEOUT.as_secs() > dwell,
        "AGENT_TIMEOUT is {}s but the agent dwells {dwell}s before answering start/restart. \
         A timeout at or under the dwell turns every honest slow answer into a false \
         `unreachable` — the client would be lying about a server that is fine.",
        AGENT_TIMEOUT.as_secs()
    );
}

/// The four literal lines the agent's `emit()` produces, byte-for-byte.
#[test]
fn parses_the_agents_exact_replies() {
    let accepted: AgentReply = serde_json::from_str(
        r#"{"ok":true,"action":"restart","result":"accepted","state":"active","detail":"unit active after restart"}"#,
    )
    .expect("accepted reply parses");
    assert!(accepted.ok);
    assert_eq!(accepted.result, AgentResult::Accepted);
    assert_eq!(accepted.state, "active");

    // The case the agent exists for: systemctl exited 0, the unit is dead.
    let rejected: AgentReply = serde_json::from_str(
        r#"{"ok":false,"action":"restart","result":"rejected","state":"failed","detail":"unit is failed after restart; systemctl rc=0"}"#,
    )
    .expect("rejected reply parses");
    assert_eq!(rejected.result, AgentResult::Rejected);
    assert_eq!(rejected.state, "failed");
    assert!(rejected.detail.contains("rc=0"));

    let unreachable: AgentReply = serde_json::from_str(
        r#"{"ok":false,"action":"status","result":"unreachable","state":"unknown","detail":"unit not installed: not-found"}"#,
    )
    .expect("unreachable reply parses");
    assert_eq!(unreachable.result, AgentResult::Unreachable);
}

/// An answer this client does not understand must NOT become a verdict.
#[test]
fn unrecognised_or_truncated_replies_fail_closed() {
    // A verdict from a future agent we cannot interpret.
    assert!(
        serde_json::from_str::<AgentReply>(
            r#"{"ok":true,"action":"restart","result":"probably","state":"active","detail":"x"}"#
        )
        .is_err(),
        "an unknown `result` must fail the parse, never fall through to accepted"
    );
    // `state` missing — the field that carries the observed truth.
    assert!(
        serde_json::from_str::<AgentReply>(
            r#"{"ok":true,"action":"restart","result":"accepted","detail":"x"}"#
        )
        .is_err(),
        "a reply with no `state` must not parse into a defaulted empty string"
    );
    assert!(serde_json::from_str::<AgentReply>("not json at all").is_err());
}

/// No listener ⇒ `Err`, and quickly. This is the "socket absent / unit not installed"
/// case the handler turns into a 503, and it must not spend the full timeout.
#[tokio::test]
async fn a_socket_with_no_listener_is_an_error_not_a_reply() {
    let path = std::env::temp_dir().join("game-agent-absent-socket.sock");
    let _ = std::fs::remove_file(&path);
    let started = std::time::Instant::now();
    let err = send(&path, AgentAction::Status)
        .await
        .expect_err("connecting to a nonexistent socket must fail");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "ENOENT must surface immediately, not after the {}s timeout",
        AGENT_TIMEOUT.as_secs()
    );
    assert!(
        err.to_string().contains("cannot connect"),
        "the error must say the channel failed, got: {err}"
    );
}
