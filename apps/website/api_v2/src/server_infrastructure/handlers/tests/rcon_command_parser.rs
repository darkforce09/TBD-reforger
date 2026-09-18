use super::*;

use crate::server_infrastructure::handlers::rcon_console::{
    RCON_ACTION_UNSUPPORTED, RCON_NO_TRANSPORT,
};

/// `custom` must not be accepted without a command.
///
/// A handler that never reads `command` makes `{"action":"custom"}` and
/// `{"action":"custom","command":"#shutdown"}` the *same request* to this API. This is the
/// smallest possible proof that the field is read.
#[test]
fn custom_action_requires_a_command() {
    assert_eq!(
        parse_rcon_command("custom", "", ""),
        Err("command required for custom action"),
        "custom with no command must be rejected, not silently accepted"
    );
    assert_eq!(
        parse_rcon_command("custom", "", "   \t\n"),
        Err("command required for custom action"),
        "whitespace-only command is not a command"
    );
    assert_eq!(
        parse_rcon_command("custom", "", "  #shutdown  "),
        Ok(RconCommand::Custom("#shutdown".to_string())),
        "a real command must survive parsing, trimmed"
    );
}

/// The operand must reach the audit row, the only sink that exists.
///
/// A row naming only the action cannot distinguish a shutdown from a message-of-the-day, and a
/// test that only checked the action name would pass against a handler that discarded the
/// operand entirely.
#[test]
fn audit_detail_carries_the_command_operand() {
    let detail = RconCommand::Custom("#shutdown 30".to_string()).audit_detail();
    assert!(
        detail.contains("#shutdown 30"),
        "audit detail must name the command that was requested, got {detail:?}"
    );
    assert_eq!(
        RconCommand::ChangeMap("Everon".to_string()).audit_detail(),
        "change_map -> Everon"
    );
    // A whitespace-only map degrades to the bare action, never invents a destination.
    assert_eq!(
        parse_rcon_command("change_map", "   ", "")
            .expect("change_map parses")
            .audit_detail(),
        "change_map"
    );
    assert_eq!(RconCommand::Restart.audit_detail(), "restart");
    assert_eq!(RconCommand::Kick.audit_detail(), "kick");
}

/// The action enum is an exact-set test, untrimmed.
#[test]
fn action_enum_still_fails_closed() {
    assert_eq!(parse_rcon_command("", "", ""), Err("action required"));
    assert_eq!(parse_rcon_command("nuke", "", ""), Err("unknown action"));
    // Whitespace-padded actions are normalised by failing, not by trimming.
    assert_eq!(
        parse_rcon_command(" restart", "", ""),
        Err("unknown action")
    );
    assert_eq!(
        parse_rcon_command("restart", "", ""),
        Ok(RconCommand::Restart)
    );
    assert_eq!(RconCommand::Restart.action(), "restart");
    assert_eq!(RconCommand::Custom("x".into()).action(), "custom");
}

/// The three unsupported actions must not be reported as a transport failure.
///
/// `restart` is the only command with a host-side representation. The distinction matters
/// operationally: "the socket is down" and "this button was never buildable" send an operator to
/// completely different places.
#[test]
fn only_restart_maps_to_a_host_verb() {
    assert_eq!(
        agent_action_for(&RconCommand::Restart),
        Some(AgentAction::Restart)
    );
    // `kick` cannot be built at all yet: `RconInput` has no player field, so even a perfect
    // channel could not name a target. See RCON_ACTION_UNSUPPORTED.
    assert_eq!(agent_action_for(&RconCommand::Kick), None);
    assert_eq!(
        agent_action_for(&RconCommand::ChangeMap("Everon".into())),
        None
    );
    assert_eq!(
        agent_action_for(&RconCommand::Custom("#shutdown".into())),
        None
    );

    // And the two refusals must not read the same to a client.
    assert_ne!(
        RCON_ACTION_UNSUPPORTED, RCON_NO_TRANSPORT,
        "an unbuildable action and a dead socket are different failures"
    );
    assert!(
        RCON_ACTION_UNSUPPORTED.contains("kick"),
        "the unsupported-action message must name why kick in particular is refused"
    );
}
