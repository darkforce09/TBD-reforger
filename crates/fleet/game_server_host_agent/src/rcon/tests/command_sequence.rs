use super::*;

#[test]
fn numbering_starts_at_zero_and_counts_up() {
    let mut sequence = CommandSequence::default();
    assert_eq!(
        (
            sequence.allocate(),
            sequence.allocate(),
            sequence.allocate()
        ),
        (0, 1, 2)
    );
}

#[test]
fn numbering_wraps_after_255_and_reuses_every_number() {
    let mut sequence = CommandSequence::default();
    let first_round: Vec<u8> = (0..256).map(|_| sequence.allocate()).collect();
    let expected: Vec<u8> = (0..=255).collect();
    assert_eq!(first_round, expected);
    let second_round: Vec<u8> = (0..256).map(|_| sequence.allocate()).collect();
    assert_eq!(
        second_round, expected,
        "after 255 the numbers are reused from 0"
    );
}
