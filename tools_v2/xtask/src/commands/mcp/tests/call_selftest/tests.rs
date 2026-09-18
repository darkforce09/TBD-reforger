use super::bash_chomp;

#[test]
fn bash_chomp_strips_trailing_newlines() {
    assert_eq!(bash_chomp("a\n\n"), "a");
    assert_eq!(bash_chomp(""), "");
}
