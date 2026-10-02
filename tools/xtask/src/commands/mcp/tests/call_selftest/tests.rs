use super::strip_trailing_newlines;

#[test]
fn trailing_newlines_are_stripped_so_a_blank_body_reads_as_empty() {
    assert_eq!(strip_trailing_newlines("a\n\n"), "a");
    assert_eq!(strip_trailing_newlines(""), "");
}
