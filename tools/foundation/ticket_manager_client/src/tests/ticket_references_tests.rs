//! The reference shapes a command line may pass, and the slice a nested slice shares.

use super::{is_legacy_ticket_number, is_ticket_reference, parent_slice};

#[test]
fn legacy_numbers_and_slugs_are_references() {
    for accepted in [
        "T-674",
        "T-674.1",
        "t-181.7.1",
        "slot-identity",
        "slot-identity.flatten-emit",
        "no-23503-handler",
    ] {
        assert!(is_ticket_reference(accepted), "{accepted}");
    }
    assert!(is_legacy_ticket_number("T-674.1"));
    assert!(!is_legacy_ticket_number("slot-identity"));
}

#[test]
fn nothing_that_can_leave_a_folder_or_split_an_argument_is_a_reference() {
    for refused in [
        "",
        "../main",
        "slot/identity",
        "slot identity",
        "-rf",
        "slot.",
        ".slot",
        "slot..identity",
        "slot.-identity",
        "Slot-Identity",
        "T-",
        "T-12a",
        "verify_wave138",
    ] {
        assert!(!is_ticket_reference(refused), "{refused:?}");
    }
}

#[test]
fn a_sub_slice_shares_its_slice_and_a_slice_keeps_its_own() {
    assert_eq!(parent_slice("T-181.7.1"), "T-181.7");
    assert_eq!(parent_slice("T-181.7"), "T-181.7");
    assert_eq!(parent_slice("T-181"), "T-181");
    assert_eq!(
        parent_slice("game-mod.briefing-map.legend"),
        "game-mod.briefing-map"
    );
    assert_eq!(
        parent_slice("game-mod.briefing-map"),
        "game-mod.briefing-map"
    );
    assert_eq!(parent_slice(""), "");
}
