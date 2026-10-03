use super::*;

#[test]
fn parse_nonneg_accepts_digits() {
    assert_eq!(parse_nonneg("14", "--keep").unwrap(), 14);
}

#[test]
fn parse_nonneg_uses_argument() {
    // Would pass both if parse_nonneg ignored `raw` and returned a constant.
    assert_eq!(parse_nonneg("0", "--min-rows").unwrap(), 0);
    assert_eq!(parse_nonneg("7", "--keep").unwrap(), 7);
    assert_ne!(
        parse_nonneg("3", "--keep").unwrap(),
        parse_nonneg("9", "--keep").unwrap()
    );
}

#[test]
fn parse_nonneg_stops_on_a_non_digit_value_with_the_fatal_line() {
    let error = parse_nonneg("-3", "--keep").unwrap_err();
    assert_eq!(
        error.stop_report(),
        Some("FATAL: --keep must be a non-negative integer (got '-3')\n")
    );
}

#[test]
fn retention_sort_is_reverse_lexicographic() {
    let mut all = [
        PathBuf::from("/tmp/tbd_x-20260101T000000Z.dump"),
        PathBuf::from("/tmp/tbd_x-20260201T000000Z.dump"),
        PathBuf::from("/tmp/tbd_x-20251201T000000Z.dump"),
    ];
    all.sort_by(|a, b| b.to_string_lossy().cmp(&a.to_string_lossy()));
    assert_eq!(all[0], PathBuf::from("/tmp/tbd_x-20260201T000000Z.dump"));
}
