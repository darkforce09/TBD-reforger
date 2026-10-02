//! Byte-identity tests of the identifier types inside every archive that holds one.
//!
//! **Role:** proves that an archive whose identifier fields are identifier types serialises to
//! exactly the bytes of the same archive with bare primitive identifier fields, and that those
//! primitive-shaped bytes validate and read back as the identifier-typed archive.
//! **Position:** test-only child of [`crate::archives`]; the identifier-typed values come from
//! `archive_round_trip_fixtures.rs`, their primitive twins from `primitive_id_record_shapes.rs`.
//! **Signals & state:** none; pure functions.
//! **Invariants:** every committed archive written before the identifier types existed keeps
//! reading unchanged, because no byte depends on the field being an identifier type.

use rkyv::rancor::Error as RkyvError;

use super::archive_round_trip_fixtures::{
    building_blueprints, forest_regions, prefab_catalog, road_network, water_vectors,
};
use super::primitive_id_record_shapes as primitive;
use crate::archives::codec::{access_checked, to_bytes};

/// Serialises `typed` and `primitive`, asserts the bytes are equal, and reads the primitive
/// bytes back as `T`.
fn assert_same_wire_bytes<T, P>(typed: &T, primitive: &P)
where
    T: rkyv::Archive
        + core::fmt::Debug
        + PartialEq
        + for<'a> rkyv::Serialize<
            rkyv::api::high::HighSerializer<
                rkyv::util::AlignedVec,
                rkyv::ser::allocator::ArenaHandle<'a>,
                RkyvError,
            >,
        >,
    rkyv::Archived<T>: rkyv::Portable
        + for<'a> rkyv::bytecheck::CheckBytes<rkyv::api::high::HighValidator<'a, RkyvError>>
        + rkyv::Deserialize<T, rkyv::api::high::HighDeserializer<RkyvError>>,
    P: for<'a> rkyv::Serialize<
            rkyv::api::high::HighSerializer<
                rkyv::util::AlignedVec,
                rkyv::ser::allocator::ArenaHandle<'a>,
                RkyvError,
            >,
        >,
{
    let typed_bytes = to_bytes(typed).expect("serialise the identifier-typed archive");
    let primitive_bytes = to_bytes(primitive).expect("serialise the primitive twin");
    assert_eq!(
        typed_bytes.as_slice(),
        primitive_bytes.as_slice(),
        "an identifier type changed the archive bytes"
    );
    let archived =
        access_checked::<T>(&primitive_bytes).expect("primitive-shaped bytes validate as T");
    let back: T = rkyv::deserialize::<T, RkyvError>(archived).expect("deserialise");
    assert_eq!(
        &back, typed,
        "primitive-shaped bytes read back as a different value"
    );
}

#[test]
fn prefab_catalog_ids_keep_the_primitive_wire_bytes() {
    let typed = prefab_catalog();
    assert_same_wire_bytes(&typed, &primitive::prefab_catalog(&typed));
}

#[test]
fn road_network_ids_keep_the_primitive_wire_bytes() {
    let typed = road_network();
    assert_same_wire_bytes(&typed, &primitive::road_network(&typed));
}

#[test]
fn forest_region_ids_keep_the_primitive_wire_bytes() {
    let typed = forest_regions();
    assert_same_wire_bytes(&typed, &primitive::forest_regions(&typed));
}

#[test]
fn water_feature_ids_keep_the_primitive_wire_bytes() {
    let typed = water_vectors();
    assert_same_wire_bytes(&typed, &primitive::water_vectors(&typed));
}

#[test]
fn building_blueprint_ids_keep_the_primitive_wire_bytes() {
    let typed = building_blueprints();
    assert_same_wire_bytes(&typed, &primitive::building_blueprints(&typed));
}

#[test]
fn archived_ids_read_through_their_accessors() {
    let catalog = to_bytes(&prefab_catalog()).expect("serialise");
    let catalog = access_checked::<crate::archives::prefabs::PrefabCatalogArchive>(&catalog)
        .expect("validated access");
    assert_eq!(catalog.prefabs[0].prefab_id.get(), 1622);
    assert_eq!(
        catalog.prefabs[0].prefab_id.to_native(),
        crate::ids::PrefabId::new(1622)
    );
    assert_eq!(catalog.type_inventory.terrain_id.as_str(), "everon");

    let blueprints = to_bytes(&building_blueprints()).expect("serialise");
    let blueprints =
        access_checked::<crate::archives::blueprints::BuildingBlueprintArchive>(&blueprints)
            .expect("validated access");
    let level = &blueprints.blueprints[0].levels[0];
    assert_eq!(level.walls[0].id, "w0");
    assert_eq!(
        level.doors[0].wall_id.to_native(),
        crate::ids::WallId::new("w0")
    );
    assert_eq!(
        level.windows[0].wall_id.as_str(),
        level.walls[0].id.as_str()
    );
    assert_eq!(level.furniture[0].id.to_string(), "f0");
}
