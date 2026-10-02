//! Each pool reports exactly one closed reason, in precedence order, or none when open.

use super::*;
use crate::operations::models::reservation_quota::ReservationQuotaPool;
use chrono::Duration;

fn at(offset: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(1_900_000_000 + offset, 0).unwrap()
}

#[test]
fn pools_report_places_openings_and_the_reason_they_are_closed() {
    let quotas = ReservationQuotas {
        member: ReservationQuotaPool {
            seats: None,
            opens_at: at(0),
        },
        guest: ReservationQuotaPool {
            seats: Some(3),
            opens_at: at(60),
        },
        open: ReservationQuotaPool {
            seats: Some(0),
            opens_at: at(0),
        },
    };
    let usage = ReservationQuotaUsage {
        member: 7,
        guest: 0,
        open: 0,
        legacy_unclassified: 2,
    };
    let pools = quota_availability(&quotas, usage, at(0));
    assert_eq!(pools[0].remaining, None);
    assert!(pools[0].open);
    assert_eq!(pools[0].allocated, 7);
    assert_eq!(pools[1].closed_reason, Some(QuotaClosedReason::NotYetOpen));
    assert_eq!(pools[1].remaining, Some(3));
    assert_eq!(pools[2].closed_reason, Some(QuotaClosedReason::NoPlaces));
    let later = quota_availability(
        &quotas,
        ReservationQuotaUsage { guest: 3, ..usage },
        at(0) + Duration::seconds(60),
    );
    assert_eq!(later[1].closed_reason, Some(QuotaClosedReason::Full));
    assert_eq!(later[1].remaining, Some(0));
    assert!(!later[1].open);
}
