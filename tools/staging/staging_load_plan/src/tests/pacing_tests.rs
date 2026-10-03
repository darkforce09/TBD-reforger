use std::time::Duration;

use super::*;
use crate::sample_plans::sample_workload;
use crate::source_addresses::{GUARD_MARGIN, busiest_window};

fn settings() -> RunSettings {
    sample_workload()
        .checked_settings()
        .expect("the sample settings")
}

fn close(left: Duration, right: Duration) -> bool {
    left.abs_diff(right) <= Duration::from_micros(1)
}

#[test]
fn a_stream_repeats_for_its_key_and_differs_across_seeds_clients_and_purposes() {
    let draws = |seed: u64, client: u32, stream: RandomStream| {
        let mut random = SeededRandom::for_stream(seed, client, stream);
        (0..4).map(|_| random.next_u64()).collect::<Vec<_>>()
    };
    let reference = draws(7, 3, RandomStream::Pacing);
    assert_eq!(reference, draws(7, 3, RandomStream::Pacing));
    assert_ne!(reference, draws(8, 3, RandomStream::Pacing));
    assert_ne!(reference, draws(7, 4, RandomStream::Pacing));
    assert_ne!(reference, draws(7, 3, RandomStream::Mix));
}

#[test]
fn a_keyed_stream_reproduces_its_pinned_draws_bit_for_bit() {
    let draws = |seed: u64, client: u32, stream: RandomStream| {
        let mut random = SeededRandom::for_stream(seed, client, stream);
        (0..4).map(|_| random.next_u64()).collect::<Vec<_>>()
    };
    assert_eq!(
        draws(0, 0, RandomStream::Pacing),
        [
            0x6EC8_5F1F_8547_BC0C,
            0x6CF6_3AFC_C21A_470A,
            0x8A27_B94C_FF75_26AA,
            0xD137_56F6_5520_A1EC
        ]
    );
    assert_eq!(
        draws(7, 5, RandomStream::Pacing),
        [
            0xC353_0381_A728_CF4F,
            0x0099_A7B2_5591_A05E,
            0xC282_8D4D_5CBE_0E13,
            0xFDD7_7B9E_6CA5_E679
        ]
    );
    assert_eq!(
        draws(0xDEAD_BEEF, 3, RandomStream::Mix),
        [
            0x083E_7DE8_1257_AD23,
            0x69C4_4824_2FB3_138D,
            0x7EAD_1125_7C2D_6588,
            0xC8CB_D68A_8499_EA74
        ]
    );
    let mut random = SeededRandom::for_stream(7, 5, RandomStream::Pacing);
    let unit = random.next_unit();
    assert_eq!(
        unit.to_bits(),
        ((0xC353_0381_A728_CF4F_u64 >> 11) as f64 / (1_u64 << 53) as f64).to_bits()
    );
}

#[test]
fn unit_draws_stay_in_the_half_open_unit_interval_and_spread_evenly() {
    let mut random = SeededRandom::for_stream(1, 0, RandomStream::Mix);
    let draws: Vec<f64> = (0..20_000).map(|_| random.next_unit()).collect();
    assert!(draws.iter().all(|draw| (0.0..1.0).contains(draw)));
    let mean = draws.iter().sum::<f64>() / draws.len() as f64;
    let below_tenth = draws.iter().filter(|&&draw| draw < 0.1).count();
    assert!((mean - 0.5).abs() < 0.01, "{mean}");
    assert!(below_tenth.abs_diff(2000) < 200, "{below_tenth}");
}

#[test]
fn clients_sign_in_evenly_across_the_ramp_and_prefetch_half_a_hold_before_each_switch() {
    let schedule = AccountSwitchSchedule::new(&settings(), 40);
    assert!(
        close(schedule.sign_in(), Duration::from_secs(24)),
        "{:?}",
        schedule.sign_in()
    );
    assert!(close(schedule.switch(3), Duration::from_secs(24 + 3 * 160)));
    assert!(close(
        schedule.prefetch(3),
        Duration::from_secs(24 + 3 * 160 - 80)
    ));
    assert!(
        schedule.prefetch(1) > schedule.sign_in(),
        "inside the first hold"
    );
}

#[test]
fn slots_keep_the_period_and_the_jitter_never_accumulates() {
    let settings = settings();
    let period = settings.period.as_secs_f64();
    let sign_in = AccountSwitchSchedule::new(&settings, 5).sign_in();
    let mut random = SeededRandom::for_stream(7, 5, RandomStream::Pacing);
    let mut schedule = MemberSlotSchedule::new(&settings, 5, &mut random);
    let first = schedule.first_slot_from(sign_in, &mut random).as_secs_f64();
    let slots: Vec<f64> = std::iter::once(first)
        .chain((1..500).map(|_| schedule.next_slot(&mut random).as_secs_f64()))
        .collect();
    let sign_in = sign_in.as_secs_f64();
    assert!(
        slots[0] >= sign_in && slots[0] < sign_in + 1.1 * period,
        "{} {sign_in}",
        slots[0]
    );
    for pair in slots.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(
            gap >= 0.9 * period - 1e-9 && gap <= 1.1 * period + 1e-9,
            "{gap}"
        );
    }
    // The jitter is drawn around a fixed grid, so 499 periods span 499·P within one jitter pair.
    let drift = slots[499] - slots[0] - 499.0 * period;
    assert!(drift.abs() <= 0.1 * period + 1e-6, "{drift}");
}

#[test]
fn every_phase_falls_in_its_clients_own_stratum_of_the_period() {
    let settings = settings();
    let stratum = settings.period / settings.clients;
    for client in 0..settings.clients {
        let mut random = SeededRandom::for_stream(7, client, RandomStream::Pacing);
        let phase = MemberSlotSchedule::new(&settings, client, &mut random).phase;
        assert!(
            phase >= stratum * client && phase < stratum * (client + 1),
            "client {client}: {phase:?}"
        );
    }
}

/// Every slot the clients of `address` pace over `periods` periods, ascending.
fn address_slots(settings: &RunSettings, seed: u64, address: u32, periods: u32) -> Vec<Duration> {
    let addresses = sample_workload().source_address_count;
    let mut slots: Vec<Duration> = (address..settings.clients)
        .step_by(addresses as usize)
        .flat_map(|client| {
            let mut random = SeededRandom::for_stream(seed, client, RandomStream::Pacing);
            let mut schedule = MemberSlotSchedule::new(settings, client, &mut random);
            (0..periods)
                .map(|_| schedule.next_slot(&mut random))
                .collect::<Vec<_>>()
        })
        .collect();
    slots.sort_unstable();
    slots
}

#[test]
fn the_paced_slots_alone_never_meet_an_addresss_all_requests_ceiling() {
    let settings = settings();
    let ceiling = settings.all_requests;
    for seed in [7, 20_260_929] {
        for address in 0..sample_workload().source_address_count {
            let slots = address_slots(&settings, seed, address, 120);
            let busiest = busiest_window(&slots, ceiling.window + GUARD_MARGIN);
            assert!(
                busiest <= ceiling.max_requests,
                "seed {seed}, address {address}: {busiest} slots inside one guarded window"
            );
        }
    }
}

#[test]
fn sign_ins_over_the_minimum_ramp_keep_every_address_under_its_auth_ceiling() {
    let mut workload = sample_workload();
    workload.ramp_seconds = workload.minimum_ramp_seconds();
    assert!((workload.ramp_seconds - 50.0).abs() < 1e-9);
    let settings = workload.checked_settings().expect("the minimum ramp");
    let ceiling = settings.auth_requests;
    for address in 0..workload.source_address_count {
        let sign_ins: Vec<Duration> = (address..settings.clients)
            .step_by(workload.source_address_count as usize)
            .map(|client| AccountSwitchSchedule::new(&settings, client).sign_in())
            .collect();
        assert_eq!(
            busiest_window(&sign_ins, ceiling.window + GUARD_MARGIN),
            ceiling.max_requests,
            "address {address}"
        );
    }
}

#[test]
fn reachable_member_accounts_count_the_switches_the_window_leaves_a_period_for() {
    let committed = sample_workload();
    assert_eq!(reachable_member_accounts(&committed).expect("reach"), 1_100);
    let mut rehearsal = sample_workload();
    rehearsal.ramp_seconds = 50.0;
    rehearsal.measured_seconds = 60.0;
    assert_eq!(reachable_member_accounts(&rehearsal).expect("reach"), 100);
    // A 165 s window leaves switch 1 (at sign-in + 160 s) a whole period for clients 0 to 2 only.
    rehearsal.measured_seconds = 115.0;
    assert_eq!(reachable_member_accounts(&rehearsal).expect("reach"), 103);
    rehearsal.accounts_per_client = 1;
    assert_eq!(reachable_member_accounts(&rehearsal).expect("reach"), 100);
    rehearsal.ramp_seconds = 10.0;
    assert!(reachable_member_accounts(&rehearsal).is_err());
}
