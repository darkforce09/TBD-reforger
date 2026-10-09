//! Tests of the two linear congruential generators: their seed-0 reference streams, the step
//! formula over many states, reproducibility, and the range and exactness of the 32-bit unit draw.

use super::*;

#[test]
fn linear_congruential_64_matches_the_mmix_reference_stream() {
    // State 0 steps to the increment itself, then onward by `state × M + I`.
    let mut draws = LinearCongruential64::new(0);
    assert_eq!(draws.next_u64(), 0x1405_7B7E_F767_814F);
    assert_eq!(draws.next_u64(), 0x1A08_EE11_84BA_6D32);
    assert_eq!(draws.next_u64(), 0x9AF6_7822_2E72_8119);
}

#[test]
fn linear_congruential_32_matches_the_c_library_reference_stream() {
    // The C library's `rand` state sequence from state 0; masked to 31 bits it is the familiar
    // 12345, 1406932606, 654583775, 1449466924.
    let mut draws = LinearCongruential32::new(0);
    let stream: Vec<u32> = (0..4).map(|_| draws.next_u32()).collect();
    assert_eq!(
        stream,
        [12_345, 3_554_416_254, 2_802_067_423, 3_596_950_572]
    );
    let masked: Vec<u32> = stream.iter().map(|state| state & 0x7FFF_FFFF).collect();
    assert_eq!(masked, [12_345, 1_406_932_606, 654_583_775, 1_449_466_924]);
}

#[test]
fn every_draw_is_the_wrapping_step_of_the_previous_state() {
    let mut wide = LinearCongruential64::new(0x5EED);
    let mut previous = 0x5EED_u64;
    for _ in 0..10_000 {
        let draw = wide.next_u64();
        assert_eq!(
            draw,
            previous
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407)
        );
        previous = draw;
    }
    let mut narrow = LinearCongruential32::new(0xC0FF_EE11);
    let mut previous = 0xC0FF_EE11_u32;
    for _ in 0..10_000 {
        let draw = narrow.next_u32();
        assert_eq!(
            draw,
            previous.wrapping_mul(1_103_515_245).wrapping_add(12_345)
        );
        previous = draw;
    }
}

#[test]
fn a_state_reproduces_its_whole_stream() {
    for state in [0, 1, 0x5EED, u64::MAX] {
        let mut first = LinearCongruential64::new(state);
        let mut second = LinearCongruential64::new(state);
        for _ in 0..1_000 {
            assert_eq!(first.next_u64(), second.next_u64(), "state {state:#x}");
        }
        assert_eq!(first, second);
    }
    for state in [0, 1, 0xC0FF_EE11, u32::MAX] {
        let mut first = LinearCongruential32::new(state);
        let mut second = LinearCongruential32::new(state);
        for _ in 0..1_000 {
            assert_eq!(first.next_unit().to_bits(), second.next_unit().to_bits());
        }
        assert_eq!(first, second);
    }
}

#[test]
fn the_32_bit_unit_draw_is_the_top_24_bits_over_2_pow_24() {
    let mut units = LinearCongruential32::new(0xC0FF_EE11);
    let mut raw = LinearCongruential32::new(0xC0FF_EE11);
    for _ in 0..100_000 {
        let unit = units.next_unit();
        let top = raw.next_u32() >> 8;
        assert!((0.0..1.0).contains(&unit), "{unit}");
        // Every integer below 2^24 is an exact f32, so scaling back recovers the top bits.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let recovered = (unit * 16_777_216.0) as u32;
        assert_eq!(recovered, top);
    }
}
