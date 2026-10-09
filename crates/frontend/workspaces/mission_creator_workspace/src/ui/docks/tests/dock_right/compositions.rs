use super::*;

/// T-650 (RIGHT-MODE-002) — the Compositions palette mode + tab exist and map to their own
/// sub-mode. The pure pins: the tab-index → sub-mode function reports Compositions for tab 4, and
/// tab 4 is NOT one of the pre-existing surfaces (so it did not silently reuse another tab's
/// slot).
#[test]
fn compositions_tab_maps_to_its_own_submode() {
    assert_eq!(EdenSubmode::from_tab(4, false), EdenSubmode::Compositions);
    // Objects mode on the Compositions tab does not turn it into Objects (that split is the
    // Factions tab's alone) — tab index wins.
    assert_eq!(EdenSubmode::from_tab(4, true), EdenSubmode::Compositions);
    // The four pre-existing surfaces keep their tabs.
    assert_eq!(EdenSubmode::from_tab(0, false), EdenSubmode::Groups);
    assert_eq!(EdenSubmode::from_tab(1, false), EdenSubmode::Vehicles);
    assert_eq!(EdenSubmode::from_tab(2, false), EdenSubmode::Markers);
    assert_eq!(EdenSubmode::from_tab(3, false), EdenSubmode::Zones);
}
