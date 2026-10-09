use super::*;

/// Absent values serve page 1 of 20; a `per_page` above 100 is served as 100.
///
/// RED: drop the clamp — `per_page=500` reads 500 rows.
#[test]
fn personnel_page_window_defaults_and_clamps() {
    assert_eq!(page_window(None, None).expect("defaults"), (1, 20));
    assert_eq!(page_window(Some(3), Some(50)).expect("explicit"), (3, 50));
    assert_eq!(page_window(Some(1), Some(100)).expect("ceiling"), (1, 100));
    assert_eq!(page_window(Some(2), Some(101)).expect("clamped"), (2, 100));
    assert_eq!(
        page_window(Some(1), Some(i64::MAX)).expect("clamped"),
        (1, 100)
    );
}

/// The search pattern matches the text literally: `%`, `_` and `\` are escaped.
///
/// RED: bind `%{q}%` unescaped — `q=_` matches every member with a non-empty name.
#[test]
fn personnel_search_pattern_matches_literally() {
    assert_eq!(contains_pattern("Target Z"), "%Target Z%");
    assert_eq!(contains_pattern("50%_off"), "%50\\%\\_off%");
    assert_eq!(contains_pattern("a\\b"), "%a\\\\b%");
}
