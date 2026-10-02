//! The action lists render as numbered approvals, the declared cases render with their NOT RUN
//! dependency, and an `AWAIT` line stays one line.
use std::path::Path;

use super::action_list::{PlannedAction, render, render_cases};
use super::awaited_effect::{await_line, effect_line, inbox_hint};
use crate::commands::staging::procedure_runner::step::DeclaredCase;

#[test]
fn staging_action_list_numbers_each_action_with_its_details() {
    let actions = [
        PlannedAction::new("harness", "Take the pre-run backup")
            .detail("cargo xtask staging backup --label pre-fleet"),
        PlannedAction::new(
            "orchestrator (browser)",
            "W1: Stop servers 1-5 in Server Control",
        ),
    ];
    assert_eq!(
        render("staging_fleet actions", &actions),
        "staging_fleet actions\n  \
         1. [harness] Take the pre-run backup\n       \
         cargo xtask staging backup --label pre-fleet\n  \
         2. [orchestrator (browser)] W1: Stop servers 1-5 in Server Control\n"
    );
    assert_eq!(
        render("staging_load recovery actions", &[]),
        "staging_load recovery actions\n  (this procedure declares no actions)\n"
    );
}

#[test]
fn staging_action_list_renders_declared_cases_with_missing_dependencies() {
    let cases = [
        DeclaredCase::runs("identity_link").unwrap(),
        DeclaredCase::not_run("kick_targets_one_of_two_clients", "second game client").unwrap(),
    ];
    assert_eq!(
        render_cases("staging_fleet", &cases),
        "staging_fleet: 2 declared cases\n  \
         case staging_fleet_identity_link\n  \
         case staging_fleet_kick_targets_one_of_two_clients ... NOT RUN (missing: second game client)\n"
    );
}

#[test]
fn staging_await_lines_stay_single_lines() {
    assert_eq!(
        await_line("w1_stop", "Stop servers 1-5\nin Server Control"),
        "AWAIT w1_stop: Stop servers 1-5 in Server Control"
    );
    assert_eq!(
        effect_line("w1_stop", "server1", Ok("stopped")),
        "  w1_stop.server1 ok (stopped)"
    );
    assert_eq!(
        effect_line("w1_stop", "server1", Err("late")),
        "  w1_stop.server1 FAILED (late)"
    );
    assert!(
        inbox_hint(Path::new("/r/browser_inbox/w4.json")).ends_with("as /r/browser_inbox/w4.json")
    );
}
