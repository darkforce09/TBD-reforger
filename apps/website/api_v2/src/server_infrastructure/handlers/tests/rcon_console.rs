use super::*;

/// Drop `//`, `///` and `//!` lines so a source pin measures **code**, not prose.
///
/// The pin below would otherwise catch its own documentation: the module's prose quotes the
/// shapes it bans in order to explain them, and an unstripped `contains` reads a quotation as the
/// thing itself. A source pin that cannot tell a comment from a statement is one bad rename away
/// from being either permanently red or quietly satisfied by a comment — both are the "passing
/// check that looked at the wrong thing" this class of test exists to prevent.
fn strip_comments(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Class-R — the two literal shapes of the defect must never appear.
///
/// This pins the source, not the behaviour, because both regressions are one line each and both
/// look harmless in review: discarding the operator's `command`, or minting a `202 accepted:true`
/// over a command nothing carried. The 202 is legitimate only when the agent's own
/// [`AgentResult::Accepted`] produces it, so the ban is narrowed to that invariant rather than to
/// the literal. The behavioural half — that `Rejected` and `Unreachable` are *not* 202 — is
/// [`delivery_verdict_comes_from_the_agents_result`], which a source pin cannot express.
#[test]
fn rcon_handler_neither_discards_the_command_nor_claims_success() {
    const SRC: &str = include_str!("../rcon_console.rs");
    let production = strip_comments(
        SRC.split("#[cfg(test)]")
            .next()
            .expect("production source before tests module"),
    );

    assert!(
        !production.contains("let _ = &input.command"),
        "the RCON handler must not discard `command` — reading it is the whole point"
    );
    // The 202 exists, and exactly one construct may produce it.
    assert!(
        production.contains("AgentResult::Accepted => RconDelivery {"),
        "the 202 must be produced only by the agent's own `accepted` verdict"
    );
    assert_eq!(
        production.matches("StatusCode::ACCEPTED").count(),
        1,
        "`StatusCode::ACCEPTED` must appear exactly once — in `rcon_delivery`'s \
         `AgentResult::Accepted` arm. A second occurrence is a second way to claim success, \
         and the second way is always the one nobody tests."
    );
    // And the honest failure paths must still be present.
    assert!(
        production.contains("StatusCode::SERVICE_UNAVAILABLE"),
        "an undeliverable RCON command must fail closed with 503"
    );
    assert!(
        production.contains("StatusCode::CONFLICT"),
        "a command the host ran and refused must be 409, not folded into 503"
    );
}

fn reply(result: AgentResult, state: &str, detail: &str) -> AgentReply {
    AgentReply {
        // Deliberately the OPPOSITE of what `result` implies on two of the three cases
        // below, so any code that reached for `ok` instead of `result` shows up here.
        ok: !matches!(result, AgentResult::Accepted),
        action: "restart".to_string(),
        result,
        state: state.to_string(),
        detail: detail.to_string(),
    }
}

/// The status comes from `result`, and from nothing else.
///
/// Each reply below carries an `ok` that *disagrees* with its `result`. A handler that read the
/// agent's boolean summary — the single-scalar habit this whole design exists to break — would
/// invert all three verdicts and this test would fail on the first one.
#[test]
fn delivery_verdict_comes_from_the_agents_result() {
    let accepted = rcon_delivery(&reply(AgentResult::Accepted, "active", "unit active"));
    assert_eq!(accepted.status, StatusCode::ACCEPTED);
    assert_eq!(accepted.severity, AuditSeverity::Info);
    assert!(accepted.accepted && accepted.delivered);

    let rejected = rcon_delivery(&reply(AgentResult::Rejected, "failed", "rc=0"));
    assert_eq!(
        rejected.status,
        StatusCode::CONFLICT,
        "the host ran the verb and the unit did not get there — that is a conflict, not an outage"
    );
    assert_eq!(rejected.severity, AuditSeverity::Warn);
    assert!(!rejected.accepted, "a refused command was never accepted");
    assert!(
        rejected.delivered,
        "the agent RAN the verb — calling this 'not delivered' sends the operator hunting a \
         network fault over a unit fault"
    );

    let unreachable = rcon_delivery(&reply(AgentResult::Unreachable, "unknown", "not installed"));
    assert_eq!(unreachable.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(unreachable.severity, AuditSeverity::Warn);
    assert!(!unreachable.delivered && !unreachable.accepted);
}

/// Class-R — **the trap.** `systemctl` exits 0 over a dead unit on this host
/// (`docs/mod/STAGING-SERVER.md:246-250`), which is why the agent returns `result` **and**
/// `state`. Reading only one of them re-introduces exactly the trust the agent removes.
///
/// The observed state must reach the audit row on every outcome. Drop `{state}` from any arm of
/// [`rcon_delivery`] and this goes red — which is the perturbation that proves it is looking at
/// something.
#[test]
fn every_outcome_carries_the_observed_state_into_the_audit_row() {
    for (result, state) in [
        (AgentResult::Accepted, "active"),
        (AgentResult::Rejected, "failed"),
        (AgentResult::Unreachable, "unknown"),
    ] {
        let d = rcon_delivery(&reply(result, state, "systemctl rc=0"));
        assert!(
            d.outcome.contains(state),
            "the audit fragment for {result:?} must name the state it rests on; got {:?}",
            d.outcome
        );
        assert!(
            d.outcome.contains("systemctl rc=0"),
            "the agent's own detail must survive into the audit row; got {:?}",
            d.outcome
        );
    }

    // The specific lie: unit `failed`, systemctl `rc=0`. An audit row that recorded this as
    // a success would be the signature defect, in the audit log, forever.
    let d = rcon_delivery(&reply(
        AgentResult::Rejected,
        "failed",
        "unit is failed after restart; systemctl rc=0",
    ));
    assert_eq!(d.status, StatusCode::CONFLICT);
    assert!(d.outcome.contains("REFUSED"));
    assert!(d.outcome.contains("failed"));
}

/// Class-R — the refuted premise must not creep into the source.
///
/// "The game server is a separate host" is false here, and citing `TBD_SSH_HOST` as evidence for
/// it is the mistake that makes it look true: that host is separate from the *developer's PC*, not
/// from the API. The whole no-secret design rests on the API and the game server being sibling
/// units under one uid, so a reader who believes the separate-host story will build a network
/// protocol nobody needs. This pin reads the **comments** (not the code) because the defect is a
/// comment.
///
/// It is phrased as a paraphrase rather than a quotation for the same reason [`strip_comments`]
/// exists: a check that cannot tell an assertion from a quotation of it is one edit away from
/// being permanently red or quietly satisfied.
#[test]
fn the_separate_host_premise_stays_refuted() {
    const SRC: &str = include_str!("../rcon_console.rs");
    let production = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("production source before tests module");

    assert!(
        !production.contains("The game server is a **separate host**"),
        "`rcon_console.rs` must not state the refuted premise — the API and the game server are \
         sibling `systemctl --user` units under one uid"
    );
    assert!(
        production.contains("sibling `systemctl --user` units"),
        "the corrected premise must be stated in the file, or the next reader has no reason to \
         doubt the version they remember"
    );
}
