use super::*;
use time_source::ManualClock;

#[test]
fn a_payload_that_is_not_json_or_names_another_tool_is_allowed() {
    let clock = ManualClock::new(1_790_000_000_000);
    assert!(judge_tool_call("", &clock).is_none());
    assert!(judge_tool_call("{not json", &clock).is_none());
    assert!(
        judge_tool_call(
            r#"{"tool_name":"Edit","session_id":"s","tool_input":{"command":"rg foo src"}}"#,
            &clock
        )
        .is_none()
    );
    assert!(judge_tool_call(r#"{"tool_name":"Bash","session_id":"s"}"#, &clock).is_none());
}

#[test]
fn a_bash_payload_reaches_the_bash_rules() {
    let clock = ManualClock::new(1_790_000_000_000);
    let deny = judge_tool_call(
        r#"{"tool_name":"Bash","session_id":"s","tool_input":{"command":"rg foo src"}}"#,
        &clock,
    );
    assert!(deny.is_some_and(|msg| msg.starts_with("Uncapped repo search: `rg foo src`.")));
    assert!(
        judge_tool_call(
            r#"{"tool_name":"Bash","session_id":"s","tool_input":{"command":"rg foo src | head"}}"#,
            &clock
        )
        .is_none()
    );
}
