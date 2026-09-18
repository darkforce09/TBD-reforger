use super::*;

#[test]
fn accept_refuses_literal_null() {
    let err = validate_accept_dom("null").unwrap_err().to_string();
    assert!(
        err.contains("JSON null"),
        "expected null refusal, got: {err}"
    );
}

#[test]
fn accept_refuses_undersized_object() {
    // Valid JSON object but far below any committed golden (and below the floor).
    let tiny = r#"{"tag":"div","attrs":{},"style":{},"children":[]}"#;
    assert!(js_len(tiny) < MIN_ACCEPT_DOM_JS_LEN);
    let err = validate_accept_dom(tiny).unwrap_err().to_string();
    assert!(
        err.contains("js_len=") && err.contains("floor"),
        "expected size-floor refusal, got: {err}"
    );
}

#[test]
fn accept_refuses_non_json() {
    let err = validate_accept_dom("not-json").unwrap_err().to_string();
    assert!(
        err.contains("not valid JSON"),
        "expected parse refusal, got: {err}"
    );
}

#[test]
fn accept_allows_plausible_object() {
    // Pad a real-shaped root past the floor without needing a full golden on disk.
    let mut body = String::from(r#"{"tag":"div","attrs":{"id":"x"},"style":{},"children":["#);
    while js_len(&format!("{body}]}}")) < MIN_ACCEPT_DOM_JS_LEN {
        body.push_str(r#"{"tag":"span","attrs":{},"style":{},"children":[]},"#);
    }
    // trim trailing comma
    if body.ends_with(',') {
        body.pop();
    }
    body.push_str("]}");
    validate_accept_dom(&body).expect("plausible DOM must pass");
}
