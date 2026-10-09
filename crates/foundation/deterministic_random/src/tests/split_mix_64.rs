//! Tests of SplitMix64: the published seed-0 reference stream, reproducibility per seed, the
//! finaliser as the draw one increment above the counter, the equality of the two unit-draw
//! spellings, and the ranges of the derived draws.

use super::*;

/// `next_unit` as the ballistics agreement lattice spells it: a multiply by `2^-53`.
fn unit_by_multiplying(draw: u64) -> f64 {
    (draw >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
}

/// `next_unit` as the placement scatter spells it: a divide by `2^53`.
fn unit_by_dividing(draw: u64) -> f64 {
    (draw >> 11) as f64 / (1u64 << 53) as f64
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
fn a_seed_reproduces_its_whole_stream() {
    for seed in [0, 1, 0x5EED, 0x00C0_FFEE, 0x5EED_0000_0000_0001, u64::MAX] {
        let mut first = SplitMix64::new(seed);
        let mut second = SplitMix64::new(seed);
        for _ in 0..1_000 {
            assert_eq!(first.next_u64(), second.next_u64(), "seed {seed:#x}");
            assert_eq!(
                first.next_unit().to_bits(),
                second.next_unit().to_bits(),
                "seed {seed:#x}"
            );
            assert_eq!(
                first.next_index(17),
                second.next_index(17),
                "seed {seed:#x}"
            );
        }
        assert_eq!(first, second);
    }
    assert_ne!(
        SplitMix64::new(1).next_u64(),
        SplitMix64::new(2).next_u64(),
        "different seeds start different streams"
    );
}

#[test]
fn a_copied_generator_continues_the_same_stream() {
    let mut original = SplitMix64::new(0x5EED);
    original.next_u64();
    let mut copy = original;
    assert_eq!(original.next_u64(), copy.next_u64());
}

#[test]
fn both_unit_spellings_give_identical_bits() {
    for draw in [
        0,
        1,
        1 << 11,
        (1 << 11) - 1,
        u64::MAX,
        u64::MAX - 1,
        1 << 63,
    ] {
        assert_eq!(
            unit_by_multiplying(draw).to_bits(),
            unit_by_dividing(draw).to_bits(),
            "{draw:#x}"
        );
    }
    let mut raw = SplitMix64::new(0x5EED);
    let mut units = SplitMix64::new(0x5EED);
    for _ in 0..100_000 {
        let draw = raw.next_u64();
        let unit = units.next_unit();
        assert_eq!(
            unit.to_bits(),
            unit_by_multiplying(draw).to_bits(),
            "{draw:#x}"
        );
        assert_eq!(
            unit.to_bits(),
            unit_by_dividing(draw).to_bits(),
            "{draw:#x}"
        );
    }
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

#[test]
fn a_zero_bound_draws_index_zero() {
    assert_eq!(
        SplitMix64::new(7).next_index(0),
        0,
        "a zero bound draws index 0"
    );
    let mut indexed = SplitMix64::new(7);
    let mut raw = SplitMix64::new(7);
    assert_eq!(indexed.next_index(1_000), raw.next_u64() % 1_000);
}

#[test]
fn the_finaliser_is_the_draw_of_a_counter_one_increment_below_the_word() {
    assert_eq!(SplitMix64::INCREMENT, 0x9E37_79B9_7F4A_7C15);
    assert_eq!(
        SplitMix64::finalise(SplitMix64::INCREMENT),
        0xE220_A839_7B1D_CDAF,
        "the finaliser of the first counter value is the first reference draw"
    );
    for word in [0, 1, 0x5EED, 0x2545_F491_4F6C_DD1D, u64::MAX] {
        assert_eq!(
            SplitMix64::finalise(word),
            SplitMix64::new(word.wrapping_sub(SplitMix64::INCREMENT)).next_u64(),
            "{word:#x}"
        );
    }
    let mut draws = SplitMix64::new(0x00C0_FFEE);
    let mut counter = 0x00C0_FFEE_u64;
    for _ in 0..10_000 {
        counter = counter.wrapping_add(SplitMix64::INCREMENT);
        assert_eq!(draws.next_u64(), SplitMix64::finalise(counter));
    }
}
