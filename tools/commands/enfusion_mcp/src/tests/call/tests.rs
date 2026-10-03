use super::*;

#[test]
fn usage_when_tool_missing() {
    assert_eq!(run(None, None), 1);
    assert_eq!(run(Some(String::new()), None), 1);
}

#[test]
fn emit_requests_embeds_tool_and_args() {
    let s = emit_requests("wb_state", r#"{"x":1}"#);
    assert!(s.contains(r#""name":"wb_state""#));
    assert!(s.contains(r#""arguments":{"x":1}"#));
    assert!(s.contains(r#""id":2"#));
}
