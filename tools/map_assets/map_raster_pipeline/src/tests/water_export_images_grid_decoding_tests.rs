//! **Role:** unit tests of [`crate::water_export_images::water_grid_decoding`]: the digit-run
//! reader's wrap, truncation, trailing number, sign and line-ending rules, and both grid files.
//! **Position:** `src/tests` of `map_raster_pipeline`, declared by
//! `water_export_images/water_grid_decoding.rs`.
//! **Signals & state:** the file cases write one grid under the system temporary folder and
//! remove it; the others read byte slices.
//! **Invariants:** every expected value is the rule's arithmetic, worked by hand.

use std::path::PathBuf;

use super::*;

/// The runs `text` yields for a grid of `sample_count` samples, and how many were stored.
fn runs_of(text: &str, sample_count: usize) -> (Vec<u64>, usize) {
    let mut values = vec![0_u64; sample_count];
    let stored = for_each_digit_run(text.as_bytes(), sample_count, |index, value| {
        values[index] = value;
    })
    .expect("read a byte slice");
    (values, stored)
}

/// A grid file under the system temporary folder holding `text`.
fn grid_file(case: &str, text: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "tbd-water-grid-{case}-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&path, text).expect("write the grid file");
    path
}

#[test]
fn digit_runs_split_on_any_other_byte() {
    assert_eq!(runs_of("1 22\t333,4", 4), (vec![1, 22, 333, 4], 4));
}

#[test]
fn a_number_at_the_end_of_the_file_is_stored_without_a_separator() {
    assert_eq!(runs_of("7 8", 2), (vec![7, 8], 2));
    assert_eq!(runs_of("7 8\n", 2), (vec![7, 8], 2));
}

#[test]
fn runs_past_the_sample_count_are_dropped() {
    assert_eq!(runs_of("1 2 3 4 5", 3), (vec![1, 2, 3], 3));
    assert_eq!(runs_of("1 2 3 4 5", 0), (vec![], 0));
}

#[test]
fn a_short_file_leaves_the_rest_untouched() {
    assert_eq!(runs_of("9", 3), (vec![9, 0, 0], 1));
}

#[test]
fn a_minus_sign_and_a_decimal_point_are_separators() {
    assert_eq!(runs_of("-5 1.5", 3), (vec![5, 1, 5], 3));
}

#[test]
fn crlf_line_endings_separate_like_line_feeds() {
    assert_eq!(runs_of("1 2\r\n3 4\r\n", 4), (vec![1, 2, 3, 4], 4));
}

#[test]
fn separators_alone_store_nothing() {
    assert_eq!(runs_of(" \r\n\t,, ", 2), (vec![0, 0], 0));
}

#[test]
fn the_mask_grid_stores_each_value_modulo_256() {
    let path = grid_file("mask", "300 255 256\n3");
    let mut mask = vec![0_u8; 4];
    let stored = decode_mask_grid(&path, &mut mask).expect("decode the mask");
    std::fs::remove_file(&path).ok();
    assert_eq!(stored, 4);
    assert_eq!(mask, vec![44, 255, 0, 3]);
}

#[test]
fn the_depth_grid_stores_each_value_modulo_65536() {
    let path = grid_file("depth", "65535 65536 70000 12\r\n");
    let mut depth = vec![0_u16; 4];
    let stored = decode_depth_grid(&path, &mut depth).expect("decode the depth");
    std::fs::remove_file(&path).ok();
    assert_eq!(stored, 4);
    assert_eq!(depth, vec![65535, 0, 4464, 12]);
}

#[test]
fn a_missing_grid_file_is_refused_with_its_path() {
    let path = std::env::temp_dir().join("tbd-water-grid-missing-file.txt");
    let error = decode_mask_grid(&path, &mut [0_u8; 1]).expect_err("no file");
    assert!(
        error
            .to_string()
            .contains("tbd-water-grid-missing-file.txt"),
        "{error}"
    );
}
