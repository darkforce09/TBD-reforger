use super::*;

#[test]
fn py_json_error_reproduces_the_measured_python_messages() {
    // FIRST FAMILY: the first non-whitespace token cannot begin a value. Every expectation was
    // measured against python3 on 2026-08-12.
    let exact = [
        (
            "not json at all {",
            "Expecting value: line 1 column 1 (char 0)",
        ),
        ("nul", "Expecting value: line 1 column 1 (char 0)"),
        ("tru", "Expecting value: line 1 column 1 (char 0)"),
        ("True", "Expecting value: line 1 column 1 (char 0)"),
        ("'a'", "Expecting value: line 1 column 1 (char 0)"),
        ("}", "Expecting value: line 1 column 1 (char 0)"),
        ("-", "Expecting value: line 1 column 1 (char 0)"),
        ("-x", "Expecting value: line 1 column 1 (char 0)"),
        ("", "Expecting value: line 1 column 1 (char 0)"),
        ("   ", "Expecting value: line 1 column 4 (char 3)"),
        ("\n\n  x", "Expecting value: line 3 column 3 (char 4)"),
    ];
    for (input, want) in exact {
        let e = serde_json::from_str::<Value>(input).expect_err("must not parse");
        assert_eq!(py_json_error(input, &e), want, "input={input:?}");
    }
    // SECOND FAMILY: a bad token in a value position inside a container — the shape a bad
    // TBD_GAME_PORT or TBD_MAX_PLAYERS produces in the rendered config.
    let in_container = [
        ("[1,]", "Expecting value: line 1 column 4 (char 3)"),
        (r#"{"a":}"#, "Expecting value: line 1 column 6 (char 5)"),
        (r#"{"a": }"#, "Expecting value: line 1 column 7 (char 6)"),
        (r#"{"a": ,}"#, "Expecting value: line 1 column 7 (char 6)"),
        (
            r#"{"a": nope}"#,
            "Expecting value: line 1 column 7 (char 6)",
        ),
        (
            r#"{"a": abc, "b":1}"#,
            "Expecting value: line 1 column 7 (char 6)",
        ),
        ("[1, oops]", "Expecting value: line 1 column 5 (char 4)"),
        (
            r#"{"p": not-a-port,}"#,
            "Expecting value: line 1 column 7 (char 6)",
        ),
        // Multi-line, which is the shape the rendered server config actually has.
        (
            "{\n  \"a\": 1,\n  \"b\": zzz\n}",
            "Expecting value: line 3 column 8 (char 19)",
        ),
    ];
    for (input, want) in in_container {
        let e = serde_json::from_str::<Value>(input).expect_err("must not parse");
        assert_eq!(py_json_error(input, &e), want, "input={input:?}");
    }
    // The declared fallbacks. python answers `Extra data`, `Expecting ':' delimiter`,
    // `Expecting property name…`, `Unterminated string…` or (for `[1,`) an `Expecting value`
    // that serde classifies as EOF rather than syntax. None of those positions is derivable
    // here, so serde's own wording stands and the divergence is declared rather than guessed.
    for input in ["{", r#"{"a""#, "[1,", "1 2", "\"abc", "0x1", "01", "{}x"] {
        let e = serde_json::from_str::<Value>(input).expect_err("must not parse");
        let got = py_json_error(input, &e);
        assert!(
            !got.starts_with("Expecting value:"),
            "input={input:?} must fall back, got {got}"
        );
    }
}

#[test]
fn py_repr_follows_pythons_quote_choice() {
    assert_eq!(py_repr("B"), "'B'");
    assert_eq!(py_repr("<unnamed>"), "'<unnamed>'");
    // repr switches to double quotes when the value has a single quote and no double.
    assert_eq!(py_repr("it's"), "\"it's\"");
    // Both present -> single quotes, with the inner single quote escaped.
    assert_eq!(py_repr("it's \"x\""), "'it\\'s \"x\"'");
}

#[test]
fn ensure_ascii_matches_json_dumps_default() {
    assert_eq!(ensure_ascii("plain"), "plain");
    assert_eq!(ensure_ascii("Café"), "Caf\\u00e9");
    // Above the BMP -> a surrogate pair, exactly as python emits.
    assert_eq!(ensure_ascii("\u{1F600}"), "\\ud83d\\ude00");
}

#[test]
fn py_type_names_match() {
    assert_eq!(py_type_name(&serde_json::json!([])), "list");
    assert_eq!(py_type_name(&serde_json::json!({})), "dict");
    assert_eq!(py_type_name(&serde_json::json!("s")), "str");
    assert_eq!(py_type_name(&serde_json::json!(1)), "int");
    assert_eq!(py_type_name(&serde_json::json!(1.5)), "float");
    assert_eq!(py_type_name(&Value::Null), "NoneType");
}

#[test]
fn py_str_or_empty_follows_pythons_truthiness() {
    // `str(m.get(k) or "")` — every falsy value collapses to the empty string, which is what
    // makes `mods[i].name is empty` fire for `null`, `false`, `0` and `""` alike.
    assert_eq!(py_str_or_empty(None), "");
    assert_eq!(py_str_or_empty(Some(&Value::Null)), "");
    assert_eq!(py_str_or_empty(Some(&serde_json::json!(false))), "");
    assert_eq!(py_str_or_empty(Some(&serde_json::json!(0))), "");
    assert_eq!(py_str_or_empty(Some(&serde_json::json!(""))), "");
    assert_eq!(py_str_or_empty(Some(&serde_json::json!([]))), "");
    // …and every truthy one keeps python's `str()`.
    assert_eq!(py_str_or_empty(Some(&serde_json::json!(true))), "True");
    assert_eq!(py_str_or_empty(Some(&serde_json::json!(5))), "5");
    assert_eq!(py_str_or_empty(Some(&serde_json::json!("x"))), "x");
}

#[test]
fn json_repr_of_a_scalar() {
    assert_eq!(json_repr(None), "None");
    assert_eq!(json_repr(Some(&serde_json::json!(2001))), "2001");
    assert_eq!(json_repr(Some(&serde_json::json!("2001"))), "'2001'");
    assert_eq!(json_repr(Some(&serde_json::json!(true))), "True");
}
