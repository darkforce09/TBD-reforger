use super::*;

#[test]
fn capture_reproduces_the_greedy_sed() {
    let reply =
        r#"{"ok":true,"action":"status","result":"accepted","state":"active","detail":"observed"}"#;
    assert_eq!(capture(r#""result":"([a-z]*)""#, reply), "accepted");
    assert_eq!(capture(r#""state":"([a-z-]*)""#, reply), "active");
    // No match prints nothing under `sed -n`, i.e. the empty string.
    assert_eq!(capture(r#""result":"([a-z]*)""#, "CLIENT-ERROR nope"), "");
    // Greedy leading `.*` takes the LAST occurrence; pinned so a future rewrite to `find()`
    // does not silently change which one wins.
    assert_eq!(
        capture(
            r#""result":"([a-z]*)""#,
            r#""result":"first" "result":"second""#
        ),
        "second"
    );
    // `state` accepts a hyphen (`not-found` never reaches the wire, but `[a-z-]` is the
    // bash's class and a narrower one would silently drop a future state name).
    assert_eq!(
        capture(r#""state":"([a-z-]*)""#, r#""state":"not-found""#),
        "not-found"
    );
}

#[test]
fn contract_key_check_is_exact() {
    assert!(json_has_exactly_contract_keys(
        r#"{"ok":true,"action":"status","result":"accepted","state":"active","detail":"x"}"#
    ));
    // Extra key -> not the contract.
    assert!(!json_has_exactly_contract_keys(
        r#"{"ok":true,"action":"s","result":"a","state":"a","detail":"x","extra":1}"#
    ));
    // Missing key -> not the contract.
    assert!(!json_has_exactly_contract_keys(r#"{"ok":true}"#));
    // Not JSON at all — the case the bash could only detect when python3 happened to exist.
    assert!(!json_has_exactly_contract_keys("CLIENT-ERROR nope"));
    assert!(!json_has_exactly_contract_keys(""));
}

#[test]
fn stub_systemctl_can_express_a_zero_exit_over_a_dead_unit() {
    // The stub is the whole reason case 4 is expressible: STUB_VERB_RC and STUB_ACTIVE are
    // independent, so "the verb says OK and the unit is failed" is a state it can produce.
    assert!(STUB_SYSTEMCTL.contains("${STUB_VERB_RC:-0}"));
    assert!(STUB_SYSTEMCTL.contains("${STUB_ACTIVE:-inactive}"));
    assert!(STUB_SYSTEMCTL.contains("${STUB_LOAD:-loaded}"));
}
