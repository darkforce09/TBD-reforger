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

/// `--only` with a slug that names no route is refused, so a mistyped slug can never select
/// nothing and report a pass.
#[test]
fn only_refuses_a_slug_that_names_no_route() {
    let all = routes();
    let refusal = routes::select_routes(&all, "dashbaord")
        .err()
        .expect("an unknown slug is refused");
    assert!(refusal.contains("dashbaord"), "{refusal}");
}

#[test]
fn only_selects_exactly_the_named_route() {
    let all = routes();
    let selected = routes::select_routes(&all, "wiki").expect("a known slug selects its route");
    let slugs: Vec<&str> = selected.iter().map(|route| route.slug).collect();
    assert_eq!(slugs, ["wiki"]);
}

#[test]
fn no_only_selects_every_route() {
    let all = routes();
    let selected = routes::select_routes(&all, "").expect("an empty --only selects every route");
    assert_eq!(selected.len(), all.len());
}

/// A verify or accept run that covered no route is a usage error, never a pass.
#[test]
fn a_run_that_covers_no_route_exits_2() {
    assert_eq!(run_modes::run_exit_code(0, 0), 2);
    assert_eq!(run_modes::run_exit_code(25, 0), 0);
    assert_eq!(run_modes::run_exit_code(25, 1), 1);
}

/// A capture records the lowercase hex SHA-256 of its DOM beside it.
#[test]
fn a_capture_digest_is_the_lowercase_hex_sha256_of_the_dom() {
    assert_eq!(
        sha_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
