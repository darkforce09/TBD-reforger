//! Unit tests of the identifier types' conversions and serde form.
//!
//! **Role:** proves the prefab identifier widths convert into each other (widening always,
//! narrowing checked), that the row width is a `Pod` of exactly two bytes, and that every
//! identifier serialises in JSON as its bare inner value.
//! **Position:** test-only child of [`crate::ids`].
//! **Signals & state:** none; pure functions.
//! **Invariants:** no conversion changes an identifier's value or silently truncates it.

use crate::ids::{
    DoorId, ForestRegionId, FurnitureId, InstancePrefabId, PrefabId, RoadSegmentId, StairsId,
    TerrainId, WallId, WaterFeatureId, WindowId,
};

#[test]
fn a_row_prefab_id_widens_to_the_catalogue_id() {
    assert_eq!(
        PrefabId::from(InstancePrefabId::new(u16::MAX)),
        PrefabId::new(65_535)
    );
}

#[test]
fn a_catalogue_id_narrows_to_the_row_width_only_when_it_fits() {
    assert_eq!(
        InstancePrefabId::try_from(PrefabId::new(1623)),
        Ok(InstancePrefabId::new(1623))
    );
    assert!(InstancePrefabId::try_from(PrefabId::new(65_536)).is_err());
}

#[test]
fn the_row_prefab_id_is_two_pod_bytes() {
    assert_eq!(size_of::<InstancePrefabId>(), 2);
    assert_eq!(align_of::<InstancePrefabId>(), 2);
    let id = InstancePrefabId::new(0x0657);
    assert_eq!(bytemuck::bytes_of(&id), &0x0657_u16.to_le_bytes());
    assert_eq!(InstancePrefabId::default().get(), 0);
}

#[test]
fn identifiers_serialise_as_their_bare_values() {
    let json = serde_json::to_string(&(
        PrefabId::new(1622),
        InstancePrefabId::new(7),
        TerrainId::new("everon"),
        RoadSegmentId::new("road-1"),
        ForestRegionId::new("forest-0"),
        WaterFeatureId::new("lake-0"),
    ))
    .expect("serialise");
    assert_eq!(json, r#"[1622,7,"everon","road-1","forest-0","lake-0"]"#);

    let json = serde_json::to_string(&(
        WallId::new("w0"),
        DoorId::new("d0"),
        WindowId::new("n0"),
        StairsId::new("s0"),
        FurnitureId::new("f0"),
    ))
    .expect("serialise");
    assert_eq!(json, r#"["w0","d0","n0","s0","f0"]"#);

    let back: (PrefabId, WallId) = serde_json::from_str(r#"[1622,"w0"]"#).expect("deserialise");
    assert_eq!(back, (PrefabId::new(1622), WallId::new("w0")));
}
