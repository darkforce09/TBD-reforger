//! Unit coverage for the byte-count labels: for the decimal label one case per unit and the
//! largest count still written in bytes; for the binary download size the empty and negative
//! counts, the megabyte rounding and the gigabyte boundary.

use super::*;

#[test]
fn format_bytes_units() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(999), "999 B");
    assert_eq!(format_bytes(2_048), "2 KB");
    assert_eq!(format_bytes(141_574_630), "141.6 MB");
    assert_eq!(format_bytes(1_500_000_000), "1.5 GB");
}

#[test]
fn format_download_size_reads_zero_bytes_below_one_byte() {
    assert_eq!(format_download_size(0), "0 B");
    assert_eq!(format_download_size(-42), "0 B");
    assert_eq!(format_download_size(i64::MIN), "0 B");
}

#[test]
fn format_download_size_rounds_to_whole_binary_megabytes_below_a_gibibyte() {
    const MEBIBYTE: i64 = 1024 * 1024;
    assert_eq!(format_download_size(1), "0 MB");
    assert_eq!(format_download_size(MEBIBYTE / 2 - 1), "0 MB");
    assert_eq!(format_download_size(MEBIBYTE), "1 MB");
    assert_eq!(format_download_size(500 * MEBIBYTE), "500 MB");
    assert_eq!(format_download_size(1024 * MEBIBYTE - 1), "1024 MB");
}

#[test]
fn format_download_size_writes_gigabytes_to_one_decimal_from_a_gibibyte() {
    const GIBIBYTE: i64 = 1024 * 1024 * 1024;
    assert_eq!(format_download_size(GIBIBYTE), "1.0 GB");
    assert_eq!(format_download_size(GIBIBYTE * 3 / 2), "1.5 GB");
    assert_eq!(format_download_size(GIBIBYTE * 20), "20.0 GB");
}
