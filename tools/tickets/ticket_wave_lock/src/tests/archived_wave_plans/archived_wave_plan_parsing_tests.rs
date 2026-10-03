use super::*;

#[test]
fn parse_rows_drops_comments_header_blanks_and_short_lines() {
    let text = "# c\nwave\tticket\ttitle\towns\n\n80\tT-1\tTitle\towns\nshort\tline\n";
    assert_eq!(parse_rows(text), vec![("80".into(), "T-1".into())]);
}
