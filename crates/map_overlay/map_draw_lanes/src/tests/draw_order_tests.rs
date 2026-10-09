//! Cases of the two lane wire id namespaces: exhaustive round trips and unknown ids refused.

use crate::lane_roles::ALL_LANES;
use crate::lane_roles::lane_role_from_u32;
use crate::lane_roles::lane_role_to_u32;

use crate::lane_roles::tex_lane_role_from_u32;

#[test]
fn wire_round_trip_is_exhaustive_both_ways() {
    let mut with_id = 0;
    for role in ALL_LANES {
        match lane_role_to_u32(role) {
            Some(id) => {
                assert_eq!(
                    lane_role_from_u32(id),
                    Some(role),
                    "role {role:?} → id {id} did not round-trip"
                );
                with_id += 1;
            }
            None => assert!(
                !(0..=crate::lane_roles::role_id::MAX).any(|i| lane_role_from_u32(i) == Some(role)),
                "{role:?} has no to_u32 id but is reachable from from_u32"
            ),
        }
    }

    for id in 0..=crate::lane_roles::role_id::MAX {
        let role = lane_role_from_u32(id).unwrap_or_else(|| {
            panic!(
                "id {id} is a hole in a dense 0..={} range",
                crate::lane_roles::role_id::MAX
            )
        });
        assert_eq!(
            lane_role_to_u32(role),
            Some(id),
            "id {id} did not round-trip"
        );
    }

    assert_eq!(
        with_id,
        usize::try_from(crate::lane_roles::role_id::MAX).unwrap() + 1
    );
    assert_eq!(
        with_id, 24,
        "ids 0..=23 are the vector lanes; 24 of them carry an upload id"
    );
}

#[test]
fn unknown_role_ids_are_none_not_a_panic() {
    for id in [
        crate::lane_roles::role_id::MAX + 1,
        crate::lane_roles::role_id::MAX + 2,
        24,
        99,
        256,
        65_536,
        u32::MAX - 1,
        u32::MAX,
    ] {
        assert_eq!(lane_role_from_u32(id), None, "id {id} must be unknown");
    }
}

#[test]
fn unknown_tex_role_ids_are_none_not_hillshade() {
    for id in [
        crate::lane_roles::tex_role_id::MAX + 1,
        crate::lane_roles::tex_role_id::MAX + 2,
        crate::lane_roles::role_id::MISSION_ZONES,
        7,
        99,
        u32::MAX,
    ] {
        assert_eq!(
            tex_lane_role_from_u32(id),
            None,
            "tex id {id} must be unknown, not silently Hillshade"
        );
    }
}
