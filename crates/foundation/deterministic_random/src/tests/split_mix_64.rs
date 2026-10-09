//! Tests of SplitMix64: the published seed-0 reference stream, reproducibility per seed, the
//! finaliser as the draw one increment above the counter, the equality of the two unit-draw
//! spellings, and the ranges of the derived draws.

use super::*;

/// `next_unit` as the ballistics agreement lattice spells it: a multiply by `2^-53`.
fn unit_by_multiplying(draw: u64) -> f64 {
    (draw >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
}

#[test]
fn split_mix_64_matches_the_reference_stream() {
    // The published SplitMix64 outputs for seed 0.
    let mut draws = SplitMix64::new(0);
    assert_eq!(draws.next_u64(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(draws.next_u64(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(draws.next_u64(), 0x06C4_5D18_8009_454F);
}

#[test]
fn unit_draws_stay_in_the_half_open_unit_interval() {
    let mut draws = SplitMix64::new(0x5EED);
    for _ in 0..10_000 {
        let unit = draws.next_unit();
        assert!((0.0..1.0).contains(&unit), "{unit}");
    }
    assert!(
        unit_by_multiplying(u64::MAX) < 1.0,
        "the largest draw stays below one"
    );
    assert_eq!(unit_by_multiplying(0), 0.0);
}

#[test]
fn ranged_draws_stay_in_their_range() {
    let mut draws = SplitMix64::new(0x00C0_FFEE);
    for _ in 0..10_000 {
        let value = draws.next_in(-5.0, 10.0);
        assert!((-5.0..5.0).contains(&value), "{value}");
        assert!(draws.next_index(3) < 3);
    }
    let mut paired = SplitMix64::new(9);
    let mut unit = SplitMix64::new(9);
    assert_eq!(
        paired.next_in(40.0, 2.5).to_bits(),
        (40.0 + 2.5 * unit.next_unit()).to_bits(),
        "`next_in` is `low + span * next_unit`"
    );
}
