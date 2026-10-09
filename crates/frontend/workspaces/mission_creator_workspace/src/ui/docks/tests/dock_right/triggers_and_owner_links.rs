use super::*;

/// T-079 (RIGHT-MODE-003) — the Triggers palette mode + tab exist and map to their own sub-mode.
/// The pure pins: tab-index → sub-mode reports Triggers for tab 5, and tab 5 is NOT one of the
/// pre-existing surfaces (so it did not silently reuse another tab's slot).
#[test]
fn triggers_tab_maps_to_its_own_submode() {
    assert_eq!(EdenSubmode::from_tab(5, false), EdenSubmode::Triggers);
    // Objects mode on the Triggers tab does not turn it into Objects (that split is the Factions
    // tab's alone) — tab index wins.
    assert_eq!(EdenSubmode::from_tab(5, true), EdenSubmode::Triggers);
    // The pre-existing surfaces keep their tabs.
    assert_eq!(EdenSubmode::from_tab(0, false), EdenSubmode::Groups);
    assert_eq!(EdenSubmode::from_tab(1, false), EdenSubmode::Vehicles);
    assert_eq!(EdenSubmode::from_tab(2, false), EdenSubmode::Markers);
    assert_eq!(EdenSubmode::from_tab(3, false), EdenSubmode::Zones);
    assert_eq!(EdenSubmode::from_tab(4, false), EdenSubmode::Compositions);
}

/// T-079 (CONN-TRG-OWNER-001) — the owner-link line's projection is PURE and native-tested: two
/// world endpoints through a projector give the screen `<line>` endpoints. Perturb / restore: a
/// projector that scales + offsets must move BOTH endpoints through it (a bug that projected only
/// one end, or dropped the offset, fails here). The dangling-owner "draw nothing" path is proven
/// in the store test; this proves the geometry the overlay draws when there IS a line.
#[test]
fn project_owner_line_maps_both_endpoints() {
    use mission_creator_state::zones::project_owner_line;
    // Trigger centre (10,20) → owner (110,220), through a scale-2 + offset projector.
    let l = project_owner_line((10.0, 20.0), (110.0, 220.0), |x, y| {
        (x * 2.0 + 5.0, y * 2.0 + 7.0)
    });
    assert!(
        (l.x1 - 25.0).abs() < 1e-9 && (l.y1 - 47.0).abs() < 1e-9,
        "endpoint A not projected"
    );
    assert!(
        (l.x2 - 225.0).abs() < 1e-9 && (l.y2 - 447.0).abs() < 1e-9,
        "endpoint B not projected"
    );
    // Identity projector → world coords pass through unchanged (the two ends are distinct).
    let id = project_owner_line((1.0, 2.0), (3.0, 4.0), |x, y| (x, y));
    assert_eq!((id.x1, id.y1, id.x2, id.y2), (1.0, 2.0, 3.0, 4.0));
}
