//! Unit tests for the reserved Discord id range.

use super::{
    FIRST_RESERVED_DISCORD_ID, LAST_RESERVED_DISCORD_ID, RESERVED_DISCORD_ID_PATTERN,
    is_reserved_discord_id,
};

#[test]
fn staging_fixtures_reserved_range_holds_its_bounds_and_nothing_outside() {
    assert!(is_reserved_discord_id("9100000000000000000"));
    assert!(is_reserved_discord_id("9100000000000012345"));
    assert!(is_reserved_discord_id("9100000000000099999"));
    assert!(!is_reserved_discord_id("9099999999999999999"));
    assert!(!is_reserved_discord_id("9100000000000100000"));
    assert!(!is_reserved_discord_id("222222222222222222"));
}

#[test]
fn staging_fixtures_reserved_range_accepts_only_the_canonical_form() {
    assert!(!is_reserved_discord_id("09100000000000000000"));
    assert!(!is_reserved_discord_id("+9100000000000000000"));
    assert!(!is_reserved_discord_id(" 9100000000000000000"));
    assert!(!is_reserved_discord_id("9100000000000000000 "));
    assert!(!is_reserved_discord_id(""));
    assert!(!is_reserved_discord_id("91000000000000000000000"));
}

/// The SQL pattern accepts the shared 14-digit prefix followed by any five digits; that is the
/// range exactly when the range starts at `…00000`, ends at `…99999` and both share the prefix.
#[test]
fn staging_fixtures_reserved_pattern_matches_the_numeric_range() {
    let first = FIRST_RESERVED_DISCORD_ID.to_string();
    let last = LAST_RESERVED_DISCORD_ID.to_string();
    assert_eq!((first.len(), last.len()), (19, 19));
    assert_eq!(first[..14], last[..14]);
    assert_eq!((&first[14..], &last[14..]), ("00000", "99999"));
    assert_eq!(
        RESERVED_DISCORD_ID_PATTERN,
        format!("^{}[0-9]{{5}}$", &first[..14])
    );
}
