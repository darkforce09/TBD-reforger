//! Seatability agrees with exhaustive assignment, keeps earlier holders, and ignores ordering.

use super::*;
use proptest::prelude::*;

fn matrix_strategy() -> impl Strategy<Value = (usize, Vec<Vec<bool>>)> {
    (0usize..=7, 0usize..=7).prop_flat_map(|(holders, seats)| {
        (
            Just(seats),
            prop::collection::vec(prop::collection::vec(any::<bool>(), seats), holders),
        )
    })
}

fn eligibility(seats: usize, rows: &[Vec<bool>]) -> SeatEligibility {
    let mut value = SeatEligibility::new(seats);
    for row in rows {
        value.push_holder(|seat| row[seat]);
    }
    value
}

/// Independent exhaustive search over injective holder → seat assignments.
fn exhaustively_matchable(rows: &[Vec<bool>], holders: &[usize], used: &mut Vec<bool>) -> bool {
    let Some((first, rest)) = holders.split_first() else {
        return true;
    };
    for seat in 0..used.len() {
        if rows[*first][seat] && !used[seat] {
            used[seat] = true;
            let found = exhaustively_matchable(rows, rest, used);
            used[seat] = false;
            if found {
                return true;
            }
        }
    }
    false
}

fn matchable(rows: &[Vec<bool>], seats: usize, holders: &[usize]) -> bool {
    exhaustively_matchable(rows, holders, &mut vec![false; seats])
}

fn exhaustive_maximum(rows: &[Vec<bool>], seats: usize) -> usize {
    let holders = rows.len();
    (0u32..(1 << holders))
        .filter(|mask| {
            let subset: Vec<usize> = (0..holders).filter(|h| mask & (1 << h) != 0).collect();
            matchable(rows, seats, &subset)
        })
        .map(u32::count_ones)
        .max()
        .unwrap_or(0) as usize
}

#[test]
fn seat_matching_agrees_with_exhaustive_assignment() {
    crate::property_evidence::run_property(
        "seat_matching_agrees_with_exhaustive_assignment",
        512,
        &matrix_strategy(),
        |(seats, rows)| {
            let value = eligibility(seats, &rows);
            let all: Vec<usize> = (0..rows.len()).collect();
            prop_assert_eq!(value.all_holders_seatable(), matchable(&rows, seats, &all));
            let assignment = value.priority_matching();
            let matched: Vec<usize> = assignment.iter().flatten().copied().collect();
            let mut distinct = matched.clone();
            distinct.sort_unstable();
            distinct.dedup();
            prop_assert_eq!(distinct.len(), matched.len(), "a seat was assigned twice");
            for (holder, seat) in assignment.iter().enumerate() {
                if let Some(seat) = seat {
                    prop_assert!(rows[holder][*seat], "an inadmissible seat was assigned");
                }
            }
            prop_assert_eq!(matched.len(), exhaustive_maximum(&rows, seats));
            Ok(())
        },
    );
}

#[test]
fn priority_matching_keeps_the_greedy_basis_of_earlier_holders() {
    crate::property_evidence::run_property(
        "priority_matching_keeps_the_greedy_basis_of_earlier_holders",
        512,
        &matrix_strategy(),
        |(seats, rows)| {
            let mut kept = Vec::new();
            let mut expected_yielding = Vec::new();
            for holder in 0..rows.len() {
                kept.push(holder);
                if !matchable(&rows, seats, &kept) {
                    kept.pop();
                    expected_yielding.push(holder);
                }
            }
            let value = eligibility(seats, &rows);
            prop_assert_eq!(value.unseatable_holders(), expected_yielding);
            // The survivors are seatable together once the yielding holders leave.
            prop_assert!(matchable(&rows, seats, &kept));
            Ok(())
        },
    );
}

#[test]
fn seatability_is_independent_of_holder_and_seat_order() {
    crate::property_evidence::run_property(
        "seatability_is_independent_of_holder_and_seat_order",
        512,
        &(
            matrix_strategy(),
            prop::collection::vec(any::<usize>(), 0..16),
        ),
        |((seats, rows), swaps)| {
            let original = eligibility(seats, &rows);
            let mut holder_order: Vec<usize> = (0..rows.len()).collect();
            let mut seat_order: Vec<usize> = (0..seats).collect();
            for (index, swap) in swaps.iter().enumerate() {
                if !holder_order.is_empty() {
                    let count = holder_order.len();
                    holder_order.swap(index % count, swap % count);
                }
                if !seat_order.is_empty() {
                    let count = seat_order.len();
                    seat_order.swap(swap % count, index % count);
                }
            }
            let permuted_rows: Vec<Vec<bool>> = holder_order
                .iter()
                .map(|holder| seat_order.iter().map(|seat| rows[*holder][*seat]).collect())
                .collect();
            let permuted = eligibility(seats, &permuted_rows);
            prop_assert_eq!(
                permuted.all_holders_seatable(),
                original.all_holders_seatable()
            );
            prop_assert_eq!(
                permuted.priority_matching().iter().flatten().count(),
                original.priority_matching().iter().flatten().count()
            );
            Ok(())
        },
    );
}

#[test]
fn removing_a_seat_is_checked_against_every_holder() {
    // Two holders both only fit seat 0 or seat 1; either seat alone strands one of them.
    let value = eligibility(3, &[vec![true, true, false], vec![true, true, false]]);
    assert!(value.all_holders_seatable());
    assert!(!value.seatable_without(0));
    assert!(!value.seatable_without(1));
    assert!(value.seatable_without(2));
    // No holders are always seatable, even without seats.
    assert!(SeatEligibility::new(0).all_holders_seatable());
    assert!(eligibility(0, &[]).unseatable_holders().is_empty());
    // A holder with no admitted seat yields before later holders are considered.
    let yielding = eligibility(1, &[vec![false], vec![true], vec![true]]);
    assert_eq!(yielding.unseatable_holders(), vec![0, 2]);
}
