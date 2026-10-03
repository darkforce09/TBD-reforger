use std::net::Ipv4Addr;

use super::*;
use staging_load_plan::sample_plans::sample_workload;
use staging_load_plan::source_addresses::busiest_window;

fn ceiling(max_requests: usize, milliseconds: u64) -> CeilingWindow {
    CeilingWindow {
        max_requests,
        window: Duration::from_millis(milliseconds),
    }
}

/// `count` requests asked for at the same instant, as offsets from it; an auth request refused
/// by its ceiling asks again from the instant the refusal names, as a client does.
fn burst(guard: &mut AddressGuard, count: usize, auth: bool) -> Vec<Duration> {
    let start = Instant::now();
    (0..count)
        .map(|_| {
            if !auth {
                return guard.reserve(start) - start;
            }
            let mut requested = start;
            loop {
                match guard.reserve_auth(requested) {
                    Ok(at) => return at - start,
                    Err(allowed) => requested = allowed,
                }
            }
        })
        .collect()
}

#[test]
fn a_burst_leaves_at_most_the_ceiling_inside_any_window() {
    let (all, auth) = (ceiling(8, 1000), ceiling(1, 2000));
    let reads = burst(&mut AddressGuard::new(all, auth), 40, false);
    assert!(
        reads.windows(2).all(|pair| pair[0] <= pair[1]),
        "reservations never decrease"
    );
    assert!(
        reads[..8].iter().all(Duration::is_zero),
        "the first eight leave at once"
    );
    assert_eq!(busiest_window(&reads, all.window), 8);
    for run in reads.windows(9) {
        assert!(run[8] - run[0] >= all.window + GUARD_MARGIN, "{run:?}");
    }
    let refreshes = burst(&mut AddressGuard::new(all, auth), 10, true);
    assert_eq!(busiest_window(&refreshes, auth.window), 1);
    for pair in refreshes.windows(2) {
        assert!(pair[1] - pair[0] >= auth.window + GUARD_MARGIN, "{pair:?}");
    }
}

#[test]
fn an_auth_request_queues_behind_requests_but_never_holds_the_queue_for_its_own_ceiling() {
    let (all, auth) = (ceiling(8, 1000), ceiling(1, 2000));
    let mut guard = AddressGuard::new(all, auth);
    let start = Instant::now();
    for _ in 0..8 {
        assert_eq!(guard.reserve(start), start);
    }
    let spacing = all.window + GUARD_MARGIN;
    assert_eq!(
        guard.reserve_auth(start),
        Ok(start + spacing),
        "the full all-requests window holds it back in the queue"
    );
    assert_eq!(
        guard.reserve_auth(start),
        Err(start + spacing + auth.window + GUARD_MARGIN),
        "its own ceiling refuses it outright"
    );
    assert_eq!(
        guard.reserve(start),
        start + spacing,
        "the queue does not wait for the refused refresh"
    );
    assert_eq!(
        guard.reserve_auth(start + spacing + auth.window + GUARD_MARGIN),
        Ok(start + spacing + auth.window + GUARD_MARGIN),
        "asked again from the named instant, it passes"
    );
}

#[test]
fn an_address_below_its_ceilings_sends_at_the_requested_instant() {
    let mut guard = AddressGuard::new(ceiling(8, 1000), ceiling(1, 2000));
    let start = Instant::now();
    for step in 0..10u64 {
        let requested = start + Duration::from_millis(2100 * step);
        if step % 2 == 0 {
            assert_eq!(guard.reserve_auth(requested), Ok(requested));
        } else {
            assert_eq!(guard.reserve(requested), requested);
        }
    }
}

#[test]
fn clients_take_the_addresses_in_turn() {
    let settings = sample_workload().checked_settings().expect("settings");
    let addresses: Vec<IpAddr> = (2..5)
        .map(|last| IpAddr::V4(Ipv4Addr::new(127, 0, 0, last)))
        .collect();
    let pool = SourceAddressPool::new(&addresses, &settings);
    let taken: Vec<usize> = (0..7)
        .map(|client| pool.address_for_client(client))
        .collect();
    assert_eq!(taken, vec![0, 1, 2, 0, 1, 2, 0]);
    assert_eq!(pool.ip(1), addresses[1]);
}
