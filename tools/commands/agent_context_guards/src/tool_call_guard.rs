//! The PreToolUse hook entry of the tool-call guard.
//!
//! **Role:** [`run_tool_call_guard`] reads the harness hook JSON (`tool_name`, `session_id`,
//! `tool_input`) on stdin, hands a `Read` call to `crate::read_guard` and a `Bash` call's
//! `command` to `crate::bash_command_guard`, and answers by exit code alone: 0 allow, 2 deny with
//! the reason and the bounded alternative on stderr. Prompt instructions do not stop an agent
//! that is stuck from re-reading a file; a PreToolUse hook refuses the call regardless of intent.
//! **Position:** behind `cargo xtask ai guard`, which the PreToolUse hook in
//! `.claude/settings.json` runs for every Read and Bash call; the live wall clock is
//! [`time_source::PlatformClock`].
//! **Signals & state:** reads stdin once; the Read rules append to the session's read set.
//! **Invariants:** fails open: unreadable stdin, a payload that is not JSON, an unknown tool or
//! any other surprise answers 0. A guard that wedges an agent costs far more than the tokens it
//! saves, so the only deny is a rule that positively matched.

use crate::bash_command_guard::guard_bash;
use crate::read_guard::guard_read;
use serde_json::Value;
use std::io::Read as _;
use time_source::{Clock, PlatformClock};

/// The PreToolUse contract's "block this call" exit code; the harness shows stderr to the agent.
const DENY_EXIT_CODE: u8 = 2;

/// Answers one PreToolUse hook call from the hook JSON on stdin and returns the exit code:
/// 0 allow, 2 deny (the reason is printed to stderr). Anything unexpected allows.
#[must_use]
pub fn run_tool_call_guard() -> u8 {
    let mut raw = String::new();
    if std::io::stdin().read_to_string(&mut raw).is_err() {
        return 0; // fail open
    }
    match judge_tool_call(&raw, &PlatformClock) {
        Some(msg) => {
            eprintln!("{msg}");
            DENY_EXIT_CODE
        }
        None => 0,
    }
}

/// The deny message for one hook payload, or None to allow it.
fn judge_tool_call(raw: &str, clock: &dyn Clock) -> Option<String> {
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return None; // fail open
    };

    let tool = v.get("tool_name").and_then(Value::as_str).unwrap_or("");
    let session = v
        .get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("nosession");
    let empty = Value::Object(Default::default());
    let input = v.get("tool_input").unwrap_or(&empty);

    match tool {
        "Read" => guard_read(session, input, clock),
        "Bash" => input
            .get("command")
            .and_then(Value::as_str)
            .and_then(guard_bash),
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests/tool_call_guard_tests.rs"]
mod tests;
