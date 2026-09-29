//! Unit tests for the id range and the synthetic profile of a load population.

use super::{SyntheticIdRange, synthetic_handle, synthetic_username};
use crate::reserved_accounts::{
    FIRST_RESERVED_DISCORD_ID, LAST_RESERVED_DISCORD_ID, is_reserved_discord_id,
};
use crate::tool_failure::ToolFailure;

fn refused(result: Result<SyntheticIdRange, ToolFailure>) -> String {
    match result {
        Err(ToolFailure::Refused(reason)) => reason,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn staging_fixtures_population_range_numbers_accounts_from_the_base() {
    let range = SyntheticIdRange::new(FIRST_RESERVED_DISCORD_ID, 1100).expect("inside the range");
    assert_eq!(range.count(), 1100);
    assert_eq!(range.discord_id(0), "9100000000000000000");
    assert_eq!(range.discord_id(1099), "9100000000000001099");
    assert_eq!(range.last(), FIRST_RESERVED_DISCORD_ID + 1099);
    assert!((0..1100).all(|index| is_reserved_discord_id(&range.discord_id(index))));
}

#[test]
fn staging_fixtures_population_range_may_end_on_the_last_reserved_id() {
    let range = SyntheticIdRange::new(LAST_RESERVED_DISCORD_ID - 9, 10).expect("inside");
    assert_eq!(range.last(), LAST_RESERVED_DISCORD_ID);
    assert_eq!(range.discord_id(9), LAST_RESERVED_DISCORD_ID.to_string());
}

#[test]
fn staging_fixtures_population_range_refuses_ids_outside_the_reserved_range() {
    assert!(refused(SyntheticIdRange::new(LAST_RESERVED_DISCORD_ID - 9, 11)).contains("leave"));
    assert!(refused(SyntheticIdRange::new(FIRST_RESERVED_DISCORD_ID - 1, 1)).contains("leave"));
    assert!(refused(SyntheticIdRange::new(u64::MAX, 2)).contains("leave"));
    assert!(refused(SyntheticIdRange::new(FIRST_RESERVED_DISCORD_ID, 0)).contains("at least 1"));
}

#[test]
fn staging_fixtures_population_profiles_name_the_account_index() {
    assert_eq!(synthetic_username(42), "Load Member 00042");
    assert_eq!(synthetic_handle(1099), "load-member-01099");
}
