//! **Role:** unit tests for the grid reference: formatting at 6, 8 and 10 figures, agreement
//! with the edge labels, parsing, the cell-centre round trip and malformed-input refusals.
//! **Position:** `camera/tests` in the map engine, declared by `camera/grid_reference.rs`.
//! **Signals & state:** none; pure functions over literal coordinates.
//! **Invariants:** every expected string and coordinate is written out by hand from the
//! `floor(m / cell) mod (100 km / cell)` convention, never derived from the code under test.

use super::*;

#[test]
fn format_writes_each_precision_from_the_same_point() {
    let (x, y) = (6423.7, 12987.2);
    assert_eq!(format_grid(x, y, GridFigures::Six), "064 129");
    assert_eq!(format_grid(x, y, GridFigures::Eight), "0642 1298");
    assert_eq!(format_grid(x, y, GridFigures::Ten), "06423 12987");
}

#[test]
fn format_six_figure_halves_are_the_edge_label_digits() {
    let mut x = 0.0;
    while x < 250_000.0 {
        let y = 250_000.0 - x;
        assert_eq!(
            format_grid(x, y, GridFigures::Six),
            format!("{} {}", grid_ref_3digit(x), grid_ref_3digit(y)),
            "x = {x}, y = {y}"
        );
        x += 37.3;
    }
}

#[test]
fn format_longer_precision_appends_digits_to_the_shorter() {
    let mut x = 0.0;
    while x < 120_000.0 {
        let six = format_grid(x, x * 0.7, GridFigures::Six);
        let eight = format_grid(x, x * 0.7, GridFigures::Eight);
        let ten = format_grid(x, x * 0.7, GridFigures::Ten);
        let halves = |s: &str| {
            let (e, n) = s.split_once(' ').unwrap();
            (e.to_string(), n.to_string())
        };
        let (e6, n6) = halves(&six);
        let (e8, n8) = halves(&eight);
        let (e10, n10) = halves(&ten);
        assert!(
            e8.starts_with(&e6) && n8.starts_with(&n6),
            "{six} / {eight}"
        );
        assert!(
            e10.starts_with(&e8) && n10.starts_with(&n8),
            "{eight} / {ten}"
        );
        x += 123.45;
    }
}

#[test]
fn format_wraps_every_100_km() {
    assert_eq!(
        format_grid(100_050.0, 199_999.0, GridFigures::Six),
        "000 999"
    );
    assert_eq!(
        format_grid(100_050.0, 199_999.0, GridFigures::Eight),
        "0005 9999"
    );
    assert_eq!(
        format_grid(100_050.0, 199_999.0, GridFigures::Ten),
        "00050 99999"
    );
}

#[test]
fn format_off_terrain_reads_all_zeros() {
    for bad in [-0.5, -1000.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(format_grid(bad, bad, GridFigures::Six), "000 000");
        assert_eq!(format_grid(bad, 5.0, GridFigures::Eight), "0000 0000");
        assert_eq!(format_grid(7.0, bad, GridFigures::Ten), "00007 00000");
    }
}

#[test]
fn parse_returns_the_cell_centre() {
    assert_eq!(parse_grid("064 129"), Ok((6450.0, 12950.0)));
    assert_eq!(parse_grid("0642 1298"), Ok((6425.0, 12985.0)));
    assert_eq!(parse_grid("06423 12987"), Ok((6423.5, 12987.5)));
    assert_eq!(parse_grid("000 000"), Ok((50.0, 50.0)));
    assert_eq!(parse_grid("99999 99999"), Ok((99_999.5, 99_999.5)));
}

#[test]
fn parse_accepts_contiguous_digits_and_surrounding_whitespace() {
    assert_eq!(parse_grid("064129"), Ok((6450.0, 12950.0)));
    assert_eq!(parse_grid("06421298"), Ok((6425.0, 12985.0)));
    assert_eq!(parse_grid("0642312987"), Ok((6423.5, 12987.5)));
    assert_eq!(parse_grid("  064   129\t"), Ok((6450.0, 12950.0)));
}

#[test]
fn round_trip_lands_on_the_cell_centre_at_every_precision() {
    for figures in GridFigures::ALL {
        let cell = figures.cell_m();
        let mut x = 0.0;
        while x < GRID_WRAP_M {
            let y = GRID_WRAP_M - 1.0 - x;
            let text = format_grid(x, y, figures);
            let (cx, cy) = parse_grid(&text).unwrap();
            assert!((cx - x).abs() <= cell / 2.0, "{figures:?} x {x} -> {cx}");
            assert!((cy - y).abs() <= cell / 2.0, "{figures:?} y {y} -> {cy}");
            assert_eq!(((cx / cell).fract()), 0.5, "{figures:?} centre {cx}");
            assert_eq!(
                format_grid(cx, cy, figures),
                text,
                "{figures:?} at {x}, {y}"
            );
            assert_eq!(parse_grid(&format_grid(cx, cy, figures)), Ok((cx, cy)));
            x += 311.7;
        }
    }
}

#[test]
fn parse_rejects_malformed_input() {
    let cases: [(&str, GridParseError); 16] = [
        ("", GridParseError::Empty),
        ("   \t ", GridParseError::Empty),
        ("06a 129", GridParseError::InvalidCharacter('a')),
        ("064-129", GridParseError::InvalidCharacter('-')),
        ("064,129", GridParseError::InvalidCharacter(',')),
        ("-064 129", GridParseError::InvalidCharacter('-')),
        ("064.5 129", GridParseError::InvalidCharacter('.')),
        (
            "\u{0660}64 129",
            GridParseError::InvalidCharacter('\u{0660}'),
        ),
        ("0641", GridParseError::UnsupportedDigitCount(4)),
        ("0641290", GridParseError::UnsupportedDigitCount(7)),
        ("123456789012", GridParseError::UnsupportedDigitCount(12)),
        ("06 12", GridParseError::UnsupportedDigitCount(4)),
        ("123456 123456", GridParseError::UnsupportedDigitCount(12)),
        (
            "064 12",
            GridParseError::MismatchedHalves {
                easting_digits: 3,
                northing_digits: 2,
            },
        ),
        (
            "0642 129",
            GridParseError::MismatchedHalves {
                easting_digits: 4,
                northing_digits: 3,
            },
        ),
        ("064 129 1", GridParseError::TooManyGroups(3)),
    ];
    for (text, expected) in cases {
        assert_eq!(parse_grid(text), Err(expected), "input {text:?}");
    }
}

#[test]
fn figure_counts_map_to_precisions() {
    assert_eq!(GridFigures::from_figure_count(6), Some(GridFigures::Six));
    assert_eq!(GridFigures::from_figure_count(8), Some(GridFigures::Eight));
    assert_eq!(GridFigures::from_figure_count(10), Some(GridFigures::Ten));
    for other in [0, 3, 4, 5, 7, 9, 11, 12] {
        assert_eq!(GridFigures::from_figure_count(other), None, "{other}");
    }
    for figures in GridFigures::ALL {
        assert_eq!(
            GridFigures::from_figure_count(figures.figure_count()),
            Some(figures)
        );
        assert_eq!(
            figures.cell_m() * 10f64.powi(figures.digits_per_axis() as i32),
            GRID_WRAP_M
        );
    }
}

#[test]
fn parse_errors_describe_the_refusal() {
    assert_eq!(
        GridParseError::UnsupportedDigitCount(7).to_string(),
        "grid reference has 7 digits; expected 6, 8 or 10"
    );
    assert_eq!(
        GridParseError::MismatchedHalves {
            easting_digits: 3,
            northing_digits: 2
        }
        .to_string(),
        "grid reference halves differ: 3 easting digits, 2 northing digits"
    );
}
