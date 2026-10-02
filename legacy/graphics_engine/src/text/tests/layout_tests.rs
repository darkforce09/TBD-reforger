//! Role: text layout tests.
//! Position: `text/tests` in the graphics engine.
//! Signals & state: pure layout arithmetic.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::text::layout::{GlyphSpec, declutter_specs_by_width};

fn spec(id: u32, x: i32, y: i32, text: &str) -> GlyphSpec {
    GlyphSpec {
        id,
        x,
        y,
        text: text.into(),
        size_m: 10.0,
    }
}

// T-0xx Phase 1D: moved here with `declutter_specs_by_width` from the map engine's
// `renderers/text/layout/tests/cases_1.rs`. The two labels were `LabelSpec`s there; the
// importance column the caller decluttered on plays no part in a WIDTH overlap, which is why
// the function could cross at all.
#[test]
fn width_declutter_drops_overlapping_long_names() {
    let a = spec(0, 0, 0, "Mountains West Ridge 02 - 332 m");
    let b = spec(1, 100, 0, "Mountains West Ridge 01 - 130 m");

    let kept = declutter_specs_by_width(&[a.clone(), b.clone()], 10.0);
    assert_eq!(kept.len(), 1, "overlapping long names collapse to one");
    assert_eq!(
        kept[0].text, a.text,
        "higher-priority (first) label survives"
    );

    let far = GlyphSpec {
        x: 2000,
        ..b.clone()
    };
    assert_eq!(declutter_specs_by_width(&[a.clone(), far], 10.0).len(), 2);

    let below = GlyphSpec { y: 500, ..b };
    assert_eq!(declutter_specs_by_width(&[a, below], 10.0).len(), 2);
}
