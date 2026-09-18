use super::*;

// The C1/C2/C3/C4/C6 tests live next to the state machine they exercise, in
// `gate_ui_layouts_awk.rs`. What remains here is the C5 half: the two `grep -o` extractions
// that turn a line into widget names.

#[test]
fn finder_names_reads_every_occurrence_leftmost_longest() {
    assert_eq!(
        finder_names(r#"a = FindAnyWidget("Title"); b = w.FindText("Detail");"#),
        vec!["Title".to_string(), "Detail".to_string()]
    );
    // `grep -o` has no word boundary — the `Find` arm really does match inside `MyFind(`.
    assert_eq!(finder_names(r#"MyFind("X")"#), vec!["X".to_string()]);
    // FindAnyWidget must not be truncated to the `Find` arm.
    assert_eq!(finder_names(r#"FindAnyWidget("Q")"#), vec!["Q".to_string()]);
    // Not matches: no `("` immediately after, and `FindTextWidget` is neither alternative.
    assert!(finder_names(r#"FindAnyWidget(name)"#).is_empty());
    assert!(finder_names(r#"FindTextWidget("Z")"#).is_empty());
}

#[test]
fn declared_name_extraction_is_line_anchored() {
    assert_eq!(declared_name(" Name \"RowBody\""), Some("RowBody".into()));
    assert_eq!(declared_name(" m_sName \"RowBody\""), None);
}
