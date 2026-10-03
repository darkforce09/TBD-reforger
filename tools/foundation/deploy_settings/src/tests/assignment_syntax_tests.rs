use super::*;

fn values(text: &str) -> Vec<(String, String)> {
    parse_assignments(text)
        .expect("parses")
        .into_iter()
        .map(|assignment| (assignment.key, assignment.value))
        .collect()
}

fn one(text: &str) -> String {
    let mut all = values(text);
    assert_eq!(all.len(), 1, "{text:?}");
    all.remove(0).1
}

#[test]
fn blank_and_comment_lines_are_skipped_and_lines_are_numbered_from_one() {
    let assignments =
        parse_assignments("# header\n\n   \nA=1\n  # indented\nB=2\n").expect("parses");
    let lines: Vec<(String, usize)> = assignments
        .into_iter()
        .map(|assignment| (assignment.key, assignment.line))
        .collect();
    assert_eq!(lines, [("A".to_string(), 4), ("B".to_string(), 6)]);
}

#[test]
fn export_prefix_and_spaces_around_the_equals_sign_are_accepted() {
    assert_eq!(one("export KEY=value"), "value");
    assert_eq!(one("export   KEY = value"), "value");
    assert_eq!(one("KEY =  value  "), "value");
    assert_eq!(one("exported=1"), "1");
}

#[test]
fn quoted_values_are_verbatim_in_either_quote() {
    assert_eq!(
        one("KEY=\"a b # not a comment $(hostname)\""),
        "a b # not a comment $(hostname)"
    );
    assert_eq!(one("KEY='a \"double\" inside'"), "a \"double\" inside");
    assert_eq!(one("KEY='$HOME \\n'"), "$HOME \\n");
    assert_eq!(one("KEY=\"x\"   "), "x");
    assert_eq!(one("KEY=\"x\" # trailing note"), "x");
    assert_eq!(one("KEY=\"\""), "");
}

#[test]
fn an_unquoted_value_ends_at_a_hash_after_whitespace() {
    assert_eq!(one("KEY= # note"), "");
    assert_eq!(one("KEY=value # note"), "value");
    assert_eq!(one("PASS=a#b"), "a#b");
    assert_eq!(one("KEY=#first"), "#first");
    assert_eq!(one("KEY=inner  spaces kept"), "inner  spaces kept");
    assert_eq!(one("KEY="), "");
}

#[test]
fn a_repeated_key_is_listed_once_per_assignment_in_order() {
    assert_eq!(
        values("KEY=first\nKEY=last\n"),
        [
            ("KEY".to_string(), "first".to_string()),
            ("KEY".to_string(), "last".to_string())
        ]
    );
}

#[test]
fn each_error_names_its_line() {
    let cases = [
        ("A=1\ntouch /tmp/canary\n", 2, "no `=`"),
        ("A=1\nB=2\n=3\n", 3, "variable name before `=` is missing"),
        ("1KEY=x\n", 1, "`1KEY` is not a variable name"),
        ("MY KEY=x\n", 1, "`MY KEY` is not a variable name"),
        ("A=1\nKEY=\"open\n", 2, "unterminated double quote"),
        ("KEY='open\n", 1, "unterminated single quote"),
        ("KEY=\"x\"y\n", 1, "text after the closing double quote"),
        ("KEY='x' y\n", 1, "text after the closing single quote"),
    ];
    for (text, line, message) in cases {
        let (got_line, got_message) = parse_assignments(text).expect_err(text);
        assert_eq!(got_line, line, "{text:?}");
        assert!(got_message.contains(message), "{text:?}: {got_message}");
    }
}

#[test]
fn an_error_never_echoes_the_value() {
    let (_, message) = parse_assignments("TBD_SSH_PASS=\"hunter2\"trailing\n").expect_err("bad");
    assert!(!message.contains("hunter2"), "{message}");
}
