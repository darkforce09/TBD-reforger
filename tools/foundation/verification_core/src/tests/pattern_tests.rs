use super::*;

#[test]
fn caret_is_a_line_anchor() {
    // THE TRAP. Default regex-crate semantics would fail this, and every `^`-anchored ban would
    // go quietly green over a file full of violations.
    let p = Pattern::regex("^forbidden").unwrap();
    assert!(p.is_match("ok line\nforbidden line\n"));
}

#[test]
fn dot_does_not_cross_a_newline() {
    // Matching is per-line, so a `.` run can never span lines.
    let p = Pattern::regex("a.*b").unwrap();
    assert!(!p.is_match("a\nb"));
    assert!(p.is_match("a x b"));
}

#[test]
fn invalid_regex_is_an_error_not_a_panic() {
    assert!(Pattern::regex("a(").is_err());
}
