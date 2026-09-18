use super::{assert_js_ok, to_code};
use serde_json::json;

/// T-386 Class-R: `{pass:false}` must fail assert_ok and map to nonzero exit.
#[test]
fn pass_false_object_fails() {
    let v = json!({"pass": false, "failed": ["W1_reached_editor", "THROWN"]});
    assert!(
        !assert_js_ok(&v),
        "pre-T-386 truthiness treated this as pass"
    );
    assert_eq!(to_code(assert_js_ok(&v)), 1);
}

/// T-386 Class-R: `{pass:true}` is the authoritative object pass.
#[test]
fn pass_true_object_ok() {
    let v = json!({"pass": true, "checks": {"a": true}});
    assert!(assert_js_ok(&v));
    assert_eq!(to_code(assert_js_ok(&v)), 0);
}

/// T-386 Class-R: bare diagnostic object (no `pass`) must FAIL, not pass.
#[test]
fn bare_diagnostic_object_fails() {
    let v = json!({"reached": true, "username": "cpl-authed", "failed": []});
    assert!(
        !assert_js_ok(&v),
        "object without boolean pass must not truthiness-pass"
    );
    assert_eq!(to_code(assert_js_ok(&v)), 1);
}

#[test]
fn literal_true_ok_literal_false_fails() {
    assert!(assert_js_ok(&json!(true)));
    assert!(!assert_js_ok(&json!(false)));
    assert_eq!(to_code(assert_js_ok(&json!(true))), 0);
    assert_eq!(to_code(assert_js_ok(&json!(false))), 1);
}

#[test]
fn diagnostic_string_is_echoable_not_pass() {
    // render_check still echoes assertValue; the string must not flip assertOk.
    assert!(!assert_js_ok(&json!("all good, username=cpl")));
    assert!(!assert_js_ok(&json!("")));
    assert!(!assert_js_ok(&json!(null)));
    assert!(!assert_js_ok(&json!(0)));
    assert!(!assert_js_ok(&json!(1)));
    assert!(!assert_js_ok(&json!([])));
}

/// Non-boolean `pass` is not a recognised verdict.
#[test]
fn pass_non_boolean_fails() {
    assert!(!assert_js_ok(&json!({"pass": "yes"})));
    assert!(!assert_js_ok(&json!({"pass": 1})));
}
