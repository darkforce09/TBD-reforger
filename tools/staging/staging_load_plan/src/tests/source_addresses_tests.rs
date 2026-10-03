use std::net::Ipv4Addr;

use super::*;

#[test]
fn the_busiest_window_is_half_open() {
    let at = Duration::from_millis;
    assert_eq!(busiest_window(&[], at(1000)), 0);
    assert_eq!(busiest_window(&[at(0), at(999), at(1000)], at(1000)), 2);
    assert_eq!(
        busiest_window(&[at(0), at(10), at(20), at(2000)], at(1000)),
        3
    );
    assert_eq!(busiest_window(&[at(5), at(5), at(5)], at(1)), 3);
}

#[test]
fn only_addresses_of_this_machine_verify() {
    verify_source_addresses(&[IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2))]).expect("a loopback alias");
    let foreign = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
    let error = verify_source_addresses(&[foreign]).expect_err("a documentation address");
    assert!(format!("{error:#}").contains("192.0.2.1"), "{error:#}");
}
