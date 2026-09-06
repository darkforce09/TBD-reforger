//! T-935.1 — Tier 2: the rkyv 0.8 archive types (spec §4).
//!
//! Everything here is metadata with variable-length, nested shape — road centrelines, label sets,
//! water rings, the prefab catalogue, forest regions, building blueprints, the satellite tile
//! index. That is exactly what raw POD cannot express and what gzip-JSON currently pays for in
//! parse time on the main thread, so these seven get rkyv: one fetch, one validation pass, then
//! `&Archived<T>` views read in place with no deserialise and no allocation.
//!
//! | Type | File |
//! |---|---|
//! | [`RoadNetworkArchive`] | `roads/road_network.rkyv` |
//! | [`MapLabelsArchive`] | `locations/map_labels.rkyv` |
//! | [`WaterVectorsArchive`] | `water/water_vectors.rkyv` |
//! | [`PrefabCatalogArchive`] | `objects/prefabs.rkyv` (+ `objects/type-inventory.rkyv`) |
//! | [`ForestRegionsArchive`] | `objects/forest-regions.rkyv` |
//! | [`BuildingBlueprintArchive`] | `prefabs/building_blueprints.rkyv` |
//! | [`TbdSatIndexV2`] | the index block inside `satellite/{terrain}-sat.tbd-sat` v2 |
//!
//! # Reading and writing
//!
//! [`super::to_bytes`] and [`super::access_checked`] — the latter validates, and it is the only
//! reader entry point (see the module docs). Nothing here is read with `access_unchecked`.
//!
//! # These are wire types, not the in-memory ones
//!
//! They deliberately do **not** reuse [`crate::world`]'s parser structs. Two reasons. Those types
//! are `f64` because they mirror JSON numbers under a Class-R byte-parity contract, and paying 8
//! bytes for a coordinate that ends up in an `f32` vertex buffer would double the file for nothing;
//! and adding rkyv derives to them would drag rkyv into the `blueprint`-only builds. So each type
//! here is the compact `f32` wire twin of a parser struct, named for the spec, and lives in its own
//! namespace — `world::binary::archives::PrefabEntry` is the *wire* row, `world::PrefabEntry` is the
//! parsed one, and they are not interchangeable.

/// Contract version written into every archive that carries one; bump when a field's meaning
/// changes rather than reusing a name.
pub const ARCHIVE_SCHEMA_VERSION: u16 = 1;

/// Every archive type derives the same four things, plus `Deserialize` back to the owned form.
/// `CheckBytes` is *not* listed: rkyv 0.8's `Archive` derive emits it automatically whenever the
/// crate's `bytecheck` feature is on (`rkyv_derive/src/archive/printing.rs:57`), which is why the
/// feature is non-optional in our Cargo.toml — turning it off would silently delete validation
/// from `access_checked` rather than fail the build.
macro_rules! archive_type {
    ($(#[$m:meta])* pub struct $name:ident { $($(#[$fm:meta])* pub $f:ident : $t:ty),* $(,)? }) => {
        $(#[$m])*
        #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq)]
        // `Archived<T>` is what a loader actually holds, so it gets `Debug` too — without this a
        // panic message or a `dbg!` on the archived side prints nothing useful.
        #[rkyv(derive(Debug))]
        pub struct $name {
            $($(#[$fm])* pub $f: $t),*
        }
    };
}

/* ──────────────────────────────────────── roads ──────────────────────────────────────── */

archive_type! {
    /// One centrelined road. `road_class` is the numeric code, not the string the JSON carries —
    /// the class table is closed (`road_style_width`), so a byte is the whole of it.
    pub struct RoadSegmentArchive {
        pub id: String,
        pub road_class: u8,
        pub width_m: f32,
        /// Centreline vertices, world metres, y-up.
        pub centerline: Vec<[f32; 2]>,
    }
}

archive_type! {
    /// `roads/road_network.rkyv` — replaces `objects/roads.json.gz`, whose quad-soup the emitter
    /// centrelines once at build time instead of on every page load.
    pub struct RoadNetworkArchive {
        pub schema_version: u16,
        pub segments: Vec<RoadSegmentArchive>,
    }
}

/* ──────────────────────────────────────── labels ─────────────────────────────────────── */

archive_type! {
    /// A settlement label: position, the declutter importance, and the taxonomy lane.
    pub struct TownLabel {
        pub name: String,
        pub position: [f32; 2],
        pub importance: f32,
        /// `city` / `village` / `hamlet` … as the locations export spells it.
        pub kind: String,
    }
}

archive_type! {
    /// A spot-height label — the metres are the label text, so they are carried decoded.
    pub struct HeightLabel {
        pub position: [f32; 2],
        pub elevation_m: f32,
    }
}

archive_type! {
    /// A road-name label placement (the `road_labels` lane), already anchored and angled.
    pub struct RoadNameLabel {
        pub name: String,
        pub position: [f32; 2],
        pub angle_deg: f32,
        pub road_class: u8,
    }
}

archive_type! {
    /// `locations/map_labels.rkyv` — every cartographic label in one file.
    pub struct MapLabelsArchive {
        pub schema_version: u16,
        pub towns: Vec<TownLabel>,
        pub height_labels: Vec<HeightLabel>,
        pub road_names: Vec<RoadNameLabel>,
    }
}

/* ───────────────────────────────────────── water ─────────────────────────────────────── */

archive_type! {
    /// A closed water polygon. `surface_y` is the still-water height in metres, which is what the
    /// renderer needs and what a ring of 2D points cannot say.
    pub struct WaterBody {
        pub id: String,
        pub surface_y: f32,
        /// Outer ring; first point is not repeated.
        pub ring: Vec<[f32; 2]>,
    }
}

archive_type! {
    /// A flowing water line (river), carried as a centreline plus a width like a road.
    pub struct WaterLine {
        pub id: String,
        pub width_m: f32,
        pub centerline: Vec<[f32; 2]>,
    }
}

archive_type! {
    /// `water/water_vectors.rkyv`. Bathymetry is the separate `TBDB` raster
    /// ([`super::chunk_container::TbdbHeader`]) — vectors here, depth field there.
    pub struct WaterVectorsArchive {
        pub schema_version: u16,
        pub lakes: Vec<WaterBody>,
        pub rivers: Vec<WaterLine>,
        pub ponds: Vec<WaterBody>,
    }
}

/* ─────────────────────────────────── prefab catalogue ────────────────────────────────── */

archive_type! {
    /// One catalogue row — the wire twin of [`crate::world::PrefabRow`] + its class code.
    ///
    /// `prefab_id` is `u32` here because it is the join key and the catalogue is addressed by it;
    /// the per-instance [`ObjectInstancePod`](super::pod::ObjectInstancePod) narrows it to the
    /// `u16` *index* into this vector, which is what keeps the POD at 32 bytes.
    pub struct PrefabEntry {
        pub prefab_id: u32,
        pub kind: String,
        pub class: String,
        pub class_code: u8,
        pub label: String,
        pub resource_name: String,
        /// Half-extents in metres, `[x, y, z]`.
        ///
        /// ABSENT IS `NaN`, NOT ZERO. This doc said zero until T-946.19, and it was wrong from the
        /// day the archive gained a writer: `prefab.rs`'s `num_to_wire` spells an absent optional
        /// as `NaN` precisely because zero is a LEGAL half-extent, so a prefab authored
        /// `halfExtents: [0, 0, 0]` would read back as "no spatial block" and the archive lane
        /// would disagree with the JSON parse on a real row. Everon carries no zero half-extent
        /// today, which is why the ambiguity went unnoticed; the writer was already right and this
        /// sentence was the only thing claiming otherwise.
        pub half_extents: [f32; 3],
        pub height_m: f32,
        pub icon_key: String,
        pub base_size_px: f32,
        /// Packed `RGBA`, one byte each — the parsed form of the `#rrggbb` string.
        pub default_color: [u8; 4],
        pub importance_zoom: f32,
    }
}

archive_type! {
    /// One `byKind` census row of `type-inventory.json`.
    pub struct KindCensus {
        pub kind: String,
        pub prefab_types: u32,
        pub instances: u64,
    }
}

archive_type! {
    /// `type-inventory.json` in archive form: the totals plus the per-kind census.
    pub struct TypeInventory {
        pub terrain_id: String,
        pub census_status: String,
        pub unique_prefabs: u32,
        pub total_instances: u64,
        /// The per-kind census, in `INSTANCE_KINDS` order with `road` last — the order every
        /// committed inventory has, and an ORDER CONTRACT, not an incidental one: a reader that
        /// indexes it by position gets a different kind's counts if it drifts.
        pub by_kind: Vec<KindCensus>,
    }
}

archive_type! {
    /// `objects/prefabs.rkyv` — the catalogue plus the inventory that describes it. The inventory
    /// also ships standalone as `objects/type-inventory.rkyv` for callers that want only the
    /// census; it is the same [`TypeInventory`] either way.
    pub struct PrefabCatalogArchive {
        pub schema_version: u16,
        pub prefabs: Vec<PrefabEntry>,
        pub type_inventory: TypeInventory,
    }
}

/* ─────────────────────────────────── forest regions ──────────────────────────────────── */

archive_type! {
    /// A land-cover region — the wire twin of [`crate::world::LandCoverRegion`]. `polygon` is
    /// ring-major: index 0 is the outer ring, the rest are holes.
    pub struct ForestRegion {
        pub id: String,
        pub kind: String,
        pub polygon: Vec<Vec<[f32; 2]>>,
        pub tree_count: u32,
        pub dominant_species_class: String,
        pub density_per_ha: f32,
        pub area_ha: f32,
        pub cover_type: String,
    }
}

archive_type! {
    /// `objects/forest-regions.rkyv`.
    pub struct ForestRegionsArchive {
        pub schema_version: u16,
        pub regions: Vec<ForestRegion>,
    }
}

/* ───────────────────────────────── building blueprints ───────────────────────────────── */

archive_type! {
    /// One BLAS file in the library — the wire twin of
    /// [`crate::world::occluder::BlasEntry`](crate::world::occluder).
    pub struct BlasEntry {
        /// `blas/<stem>.bvh`, relative to the prefabs root.
        pub path: String,
        pub bytes: u64,
        pub tris: u32,
        /// Triangle counts per surface kind: `[opaque, glass, foliage]`.
        pub kinds: [u32; 3],
    }
}

archive_type! {
    /// One prefab's collision closure, indexed by `prefab_id` — the wire twin of the
    /// `prefabs/descriptors/<pid>.json` occluder descriptor. `blas` indexes into
    /// [`BuildingBlueprintArchive::blas_index`], which is what turns 1623 descriptor fetches into
    /// one archive fetch.
    pub struct OccluderDescriptor {
        pub prefab_id: u32,
        pub slug: String,
        pub kind: String,
        /// Something in the closure collides; a `false` descriptor never blocks a shot.
        pub blocks: bool,
        /// Carries foliage triangles (a tree canopy).
        pub canopy: bool,
        /// Union of every instance's placed bounds in the object frame, `[min_xyz, max_xyz]`.
        pub local_bounds: [[f32; 3]; 2],
        /// Indices into [`BuildingBlueprintArchive::blas_index`], first-use order, root first.
        pub blas: Vec<u32>,
    }
}

archive_type! {
    /// Heights that define a building's silhouette, metres above the pivot.
    pub struct VerticalProfile {
        pub pivot_elevation_offset_m: f32,
        pub foundation_skirt_depth_m: f32,
        pub total_height_m: f32,
        pub eave_height_m: f32,
        pub ridge_height_m: f32,
        pub roof_type: String,
    }
}

archive_type! {
    /// A wall segment in the level's plan.
    pub struct WallRec {
        pub id: String,
        pub start: [f32; 2],
        pub end: [f32; 2],
        pub thickness_m: f32,
        pub is_exterior: bool,
        pub material: String,
    }
}

archive_type! {
    /// A door opening. `wall_id` refers to a [`WallRec::id`] in the same level.
    pub struct DoorRec {
        pub id: String,
        pub wall_id: String,
        pub position: [f32; 2],
        pub width_m: f32,
        pub height_m: f32,
        pub is_exterior: bool,
        pub has_glass: bool,
    }
}

archive_type! {
    /// A window opening — `normal` and `fov_deg` are what the LOS pass needs to decide whether a
    /// shooter can see through it.
    pub struct WindowRec {
        pub id: String,
        pub wall_id: String,
        pub position: [f32; 2],
        pub width_m: f32,
        pub sill_height_m: f32,
        pub window_height_m: f32,
        pub normal: [f32; 2],
        pub fov_deg: f32,
        pub has_glass: bool,
    }
}

archive_type! {
    /// A stair run connecting this level to `connects_to_level`.
    pub struct StairsRec {
        pub id: String,
        /// `[min, max]` of the footprint.
        pub bounds: [[f32; 2]; 2],
        pub connects_to_level: u8,
        pub direction_deg: f32,
        pub step_count: u16,
        pub transparent_steps: bool,
        /// 0.0 = no concealment, 1.0 = opaque.
        pub los_concealment: f32,
    }
}

archive_type! {
    /// A furniture item — `blocks_movement` and `los_cover` are the two properties the tactical
    /// layers read; the model itself is not carried.
    pub struct FurnitureRec {
        pub id: String,
        pub name: String,
        pub category: String,
        pub position: [f32; 2],
        pub rotation_deg: f32,
        pub height_m: f32,
        pub blocks_movement: bool,
        /// `none` / `partial` / `full`.
        pub los_cover: String,
    }
}

archive_type! {
    /// One storey of a building.
    pub struct BuildingLevel {
        pub level_index: u8,
        /// `[floor_m, ceiling_m]` relative to the building pivot.
        pub elevation_range: [f32; 2],
        pub footprint_polygon: Vec<[f32; 2]>,
        pub walls: Vec<WallRec>,
        pub doors: Vec<DoorRec>,
        pub windows: Vec<WindowRec>,
        pub stairs: Vec<StairsRec>,
        pub furniture: Vec<FurnitureRec>,
    }
}

archive_type! {
    /// One building's architectural blueprint, keyed by the catalogue `prefab_id`.
    pub struct BuildingBlueprint {
        pub prefab_id: u32,
        pub slug: String,
        pub vertical_profile: VerticalProfile,
        pub levels: Vec<BuildingLevel>,
    }
}

archive_type! {
    /// `prefabs/building_blueprints.rkyv` — descriptors, the BLAS index they point into, and the
    /// blueprints, in one file. This is the archive that replaces 1623 individual descriptor
    /// fetches at editor boot; the `.bvh` sidecars stay separate and are fetched by index.
    pub struct BuildingBlueprintArchive {
        pub schema_version: u16,
        pub descriptors: Vec<OccluderDescriptor>,
        pub blas_index: Vec<BlasEntry>,
        pub blueprints: Vec<BuildingBlueprint>,
    }
}

/* ───────────────────────────────── satellite index v2 ────────────────────────────────── */

archive_type! {
    /// One tile's location inside a `.tbd-sat` v2 payload. `offset` is measured from
    /// [`TbdsHeader::tiles_offset`](super::chunk_container::TbdsHeader::tiles_offset), so a reader
    /// can turn it into an HTTP Range without knowing the index's length twice.
    pub struct SatTile {
        pub offset: u64,
        pub len: u32,
        /// Codec code — `0` = webp.
        pub format: u8,
    }
}

archive_type! {
    /// One mip level of the satellite pyramid, in `w_tiles * h_tiles` row-major order.
    pub struct SatLevel {
        pub w_tiles: u32,
        pub h_tiles: u32,
        pub tiles: Vec<SatTile>,
    }
}

archive_type! {
    /// The index block of `satellite/{terrain}-sat.tbd-sat` **v2** (spec §3.5) — the rkyv
    /// replacement for v1's hand-packed offset table. Level 0 is the base resolution.
    pub struct TbdSatIndexV2 {
        pub base_w: u32,
        pub base_h: u32,
        pub tile_px: u16,
        pub levels: Vec<SatLevel>,
    }
}

#[cfg(test)]
mod tests {
    use super::super::chunk_container::{ContainerHeader, TbdsHeader};
    use super::super::{BinaryError, access_checked, to_bytes};
    use super::*;
    use rkyv::rancor::Error as RkyvError;

    /// The round-trip every archive must survive, and the strongest form of it:
    ///
    /// 1. serialise,
    /// 2. `access_checked` — validating — and read a field through the archived view,
    /// 3. deserialise back to the owned value and compare it to the original,
    /// 4. re-serialise the deserialised value and compare the **bytes** to step 1.
    ///
    /// Step 4 is what makes it byte-for-byte rather than merely "equal enough": a field that
    /// serialises but loses information (a dropped `Vec` tail, a truncated string) survives step 3
    /// only if `PartialEq` also lost it, and step 4 catches that.
    fn round_trip<T>(value: &T) -> rkyv::util::AlignedVec
    where
        T: rkyv::Archive
            + Clone
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
    {
        let bytes = to_bytes(value).expect("serialise");
        let archived = access_checked::<T>(&bytes).expect("validated access");
        let back: T = rkyv::deserialize::<T, RkyvError>(archived).expect("deserialise");
        assert_eq!(&back, value, "deserialised value differs from the original");
        let again = to_bytes(&back).expect("re-serialise");
        assert_eq!(
            bytes.as_slice(),
            again.as_slice(),
            "re-serialising the round-tripped value produced different bytes"
        );
        bytes
    }

    /// Corruption must be an `Err` from `access_checked`, never a panic and never a silent read.
    ///
    /// The bytes flipped are in the archive's **tail**, where rkyv puts the root and its relative
    /// pointers — flipping a byte in a string's payload is legal data, but flipping a pointer or a
    /// length is exactly the wild-read this validation exists to stop.
    fn corruption_is_rejected<T>(bytes: &rkyv::util::AlignedVec)
    where
        T: rkyv::Archive,
        rkyv::Archived<T>: rkyv::Portable
            + for<'a> rkyv::bytecheck::CheckBytes<rkyv::api::high::HighValidator<'a, RkyvError>>,
    {
        let mut rejected = 0_usize;
        let root = bytes.len().saturating_sub(size_of::<rkyv::Archived<T>>());
        for i in root..bytes.len() {
            let mut bad = bytes.clone();
            bad[i] ^= 0xFF;
            if let Err(e) = access_checked::<T>(&bad) {
                assert!(matches!(e, BinaryError::Archive { .. }), "{e}");
                rejected += 1;
            }
        }
        assert!(
            rejected > 0,
            "flipping every byte of the archive root was accepted — validation is not running"
        );

        // Truncation, at several depths, must also be an error rather than a wild read.
        for cut in [0, 1, bytes.len() / 2, bytes.len() - 1] {
            assert!(
                access_checked::<T>(&bytes[..cut]).is_err(),
                "a {cut}-byte prefix of a {}-byte archive was accepted",
                bytes.len()
            );
        }
    }

    fn road_network() -> RoadNetworkArchive {
        RoadNetworkArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            segments: vec![
                RoadSegmentArchive {
                    id: "road-1".into(),
                    road_class: 3,
                    width_m: 2.5,
                    centerline: vec![[0.0, 0.0], [10.5, -4.25], [22.0, 9.75]],
                },
                RoadSegmentArchive {
                    id: "runway-0".into(),
                    road_class: 6,
                    width_m: 20.0,
                    centerline: vec![[100.0, 100.0], [1100.0, 100.0]],
                },
            ],
        }
    }

    fn map_labels() -> MapLabelsArchive {
        MapLabelsArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            towns: vec![TownLabel {
                name: "Montignac".into(),
                position: [6400.0, 6400.0],
                importance: 0.875,
                kind: "town".into(),
            }],
            height_labels: vec![HeightLabel {
                position: [1024.0, 2048.0],
                elevation_m: 375.53,
            }],
            road_names: vec![RoadNameLabel {
                name: "Route 1".into(),
                position: [12.0, 34.0],
                angle_deg: -33.5,
                road_class: 1,
            }],
        }
    }

    fn water_vectors() -> WaterVectorsArchive {
        WaterVectorsArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            lakes: vec![WaterBody {
                id: "lake-0".into(),
                surface_y: 12.5,
                ring: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 80.0], [0.0, 80.0]],
            }],
            rivers: vec![WaterLine {
                id: "river-0".into(),
                width_m: 6.0,
                centerline: vec![[0.0, 0.0], [50.0, 60.0]],
            }],
            ponds: vec![],
        }
    }

    fn prefab_catalog() -> PrefabCatalogArchive {
        PrefabCatalogArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            prefabs: vec![PrefabEntry {
                prefab_id: 1622,
                kind: "building".into(),
                class: "house".into(),
                class_code: 2,
                label: "Farm House".into(),
                resource_name: "Prefabs/Buildings/FarmHouse_E_1L01_Wood.et".into(),
                half_extents: [6.5, 4.25, 8.0],
                height_m: 8.0,
                icon_key: "building".into(),
                base_size_px: 14.0,
                default_color: [0xC8, 0xB4, 0x96, 0xFF],
                importance_zoom: -2.5,
            }],
            type_inventory: TypeInventory {
                terrain_id: "everon".into(),
                census_status: "partial".into(),
                unique_prefabs: 1623,
                total_instances: 1_216_066,
                by_kind: vec![
                    KindCensus {
                        kind: "building".into(),
                        prefab_types: 287,
                        instances: 4131,
                    },
                    KindCensus {
                        kind: "tree".into(),
                        prefab_types: 51,
                        instances: 501_828,
                    },
                ],
            },
        }
    }

    fn forest_regions() -> ForestRegionsArchive {
        ForestRegionsArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            regions: vec![ForestRegion {
                id: "forest-0".into(),
                kind: "forest".into(),
                polygon: vec![
                    vec![[0.0, 0.0], [200.0, 0.0], [200.0, 150.0], [0.0, 150.0]],
                    vec![[50.0, 50.0], [60.0, 50.0], [60.0, 60.0]],
                ],
                tree_count: 4821,
                dominant_species_class: "pine".into(),
                density_per_ha: 160.5,
                area_ha: 3.0,
                cover_type: "dense".into(),
            }],
        }
    }

    fn building_blueprints() -> BuildingBlueprintArchive {
        BuildingBlueprintArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            descriptors: vec![OccluderDescriptor {
                prefab_id: 1622,
                slug: "FarmHouse_E_1L01_Wood".into(),
                kind: "building".into(),
                blocks: true,
                canopy: false,
                local_bounds: [[-6.5, -4.25, 0.0], [6.5, 4.25, 8.0]],
                blas: vec![0, 1],
            }],
            blas_index: vec![
                BlasEntry {
                    path: "blas/FarmHouse_E_1L01_Wood.bvh".into(),
                    bytes: 40_960,
                    tris: 1204,
                    kinds: [1100, 96, 8],
                },
                BlasEntry {
                    path: "blas/Door_Wood_01.bvh".into(),
                    bytes: 2048,
                    tris: 24,
                    kinds: [24, 0, 0],
                },
            ],
            blueprints: vec![BuildingBlueprint {
                prefab_id: 1622,
                slug: "FarmHouse_E_1L01_Wood".into(),
                vertical_profile: VerticalProfile {
                    pivot_elevation_offset_m: 0.0,
                    foundation_skirt_depth_m: 0.35,
                    total_height_m: 8.0,
                    eave_height_m: 5.5,
                    ridge_height_m: 8.0,
                    roof_type: "gable".into(),
                },
                levels: vec![BuildingLevel {
                    level_index: 0,
                    elevation_range: [0.0, 2.8],
                    footprint_polygon: vec![[0.0, 0.0], [13.0, 0.0], [13.0, 8.5], [0.0, 8.5]],
                    walls: vec![WallRec {
                        id: "w0".into(),
                        start: [0.0, 0.0],
                        end: [13.0, 0.0],
                        thickness_m: 0.3,
                        is_exterior: true,
                        material: "brick".into(),
                    }],
                    doors: vec![DoorRec {
                        id: "d0".into(),
                        wall_id: "w0".into(),
                        position: [4.0, 0.0],
                        width_m: 0.9,
                        height_m: 2.1,
                        is_exterior: true,
                        has_glass: false,
                    }],
                    windows: vec![WindowRec {
                        id: "n0".into(),
                        wall_id: "w0".into(),
                        position: [9.0, 0.0],
                        width_m: 1.2,
                        sill_height_m: 0.9,
                        window_height_m: 1.4,
                        normal: [0.0, -1.0],
                        fov_deg: 140.0,
                        has_glass: true,
                    }],
                    stairs: vec![StairsRec {
                        id: "s0".into(),
                        bounds: [[10.0, 5.0], [12.0, 8.0]],
                        connects_to_level: 1,
                        direction_deg: 90.0,
                        step_count: 16,
                        transparent_steps: false,
                        los_concealment: 0.75,
                    }],
                    furniture: vec![FurnitureRec {
                        id: "f0".into(),
                        name: "Table".into(),
                        category: "table".into(),
                        position: [6.0, 4.0],
                        rotation_deg: 45.0,
                        height_m: 0.75,
                        blocks_movement: true,
                        los_cover: "partial".into(),
                    }],
                }],
            }],
        }
    }

    fn sat_index() -> TbdSatIndexV2 {
        TbdSatIndexV2 {
            base_w: 12800,
            base_h: 12800,
            tile_px: 256,
            levels: vec![
                SatLevel {
                    w_tiles: 50,
                    h_tiles: 50,
                    tiles: vec![
                        SatTile {
                            offset: 0,
                            len: 32_768,
                            format: 0,
                        },
                        SatTile {
                            offset: 32_768,
                            len: 30_000,
                            format: 0,
                        },
                    ],
                },
                SatLevel {
                    w_tiles: 25,
                    h_tiles: 25,
                    tiles: vec![SatTile {
                        offset: 62_768,
                        len: 28_000,
                        format: 0,
                    }],
                },
            ],
        }
    }

    #[test]
    fn road_network_round_trips_and_rejects_corruption() {
        let v = road_network();
        let bytes = round_trip(&v);
        let a = access_checked::<RoadNetworkArchive>(&bytes).expect("access");
        assert_eq!(a.segments.len(), 2);
        assert_eq!(a.segments[0].id.as_str(), "road-1");
        assert_eq!(a.segments[1].width_m.to_native(), 20.0);
        assert_eq!(a.segments[0].centerline.len(), 3);
        corruption_is_rejected::<RoadNetworkArchive>(&bytes);
    }

    #[test]
    fn map_labels_round_trips_and_rejects_corruption() {
        let bytes = round_trip(&map_labels());
        let a = access_checked::<MapLabelsArchive>(&bytes).expect("access");
        assert_eq!(a.towns[0].name.as_str(), "Montignac");
        assert_eq!(a.height_labels.len(), 1);
        assert_eq!(a.road_names[0].road_class, 1);
        corruption_is_rejected::<MapLabelsArchive>(&bytes);
    }

    #[test]
    fn water_vectors_round_trips_and_rejects_corruption() {
        let bytes = round_trip(&water_vectors());
        let a = access_checked::<WaterVectorsArchive>(&bytes).expect("access");
        assert_eq!(a.lakes[0].ring.len(), 4);
        assert_eq!(a.rivers[0].width_m.to_native(), 6.0);
        assert!(
            a.ponds.is_empty(),
            "an empty Vec must survive the round trip"
        );
        corruption_is_rejected::<WaterVectorsArchive>(&bytes);
    }

    #[test]
    fn prefab_catalog_round_trips_and_rejects_corruption() {
        let bytes = round_trip(&prefab_catalog());
        let a = access_checked::<PrefabCatalogArchive>(&bytes).expect("access");
        assert_eq!(a.prefabs[0].prefab_id.to_native(), 1622);
        assert_eq!(a.type_inventory.total_instances.to_native(), 1_216_066);
        assert_eq!(a.type_inventory.by_kind.len(), 2);
        corruption_is_rejected::<PrefabCatalogArchive>(&bytes);
    }

    #[test]
    fn forest_regions_round_trips_and_rejects_corruption() {
        let bytes = round_trip(&forest_regions());
        let a = access_checked::<ForestRegionsArchive>(&bytes).expect("access");
        assert_eq!(a.regions[0].polygon.len(), 2, "outer ring plus one hole");
        assert_eq!(a.regions[0].tree_count.to_native(), 4821);
        corruption_is_rejected::<ForestRegionsArchive>(&bytes);
    }

    #[test]
    fn building_blueprints_round_trip_and_reject_corruption() {
        let bytes = round_trip(&building_blueprints());
        let a = access_checked::<BuildingBlueprintArchive>(&bytes).expect("access");
        assert_eq!(a.descriptors[0].blas.len(), 2);
        assert_eq!(a.blas_index[0].tris.to_native(), 1204);
        let level = &a.blueprints[0].levels[0];
        assert_eq!(level.level_index, 0);
        assert_eq!(level.walls[0].id.as_str(), "w0");
        assert_eq!(level.doors[0].wall_id.as_str(), "w0");
        assert_eq!(level.windows[0].fov_deg.to_native(), 140.0);
        assert_eq!(level.stairs[0].step_count.to_native(), 16);
        assert_eq!(level.furniture[0].los_cover.as_str(), "partial");
        corruption_is_rejected::<BuildingBlueprintArchive>(&bytes);
    }

    #[test]
    fn sat_index_round_trips_and_rejects_corruption() {
        let bytes = round_trip(&sat_index());
        let a = access_checked::<TbdSatIndexV2>(&bytes).expect("access");
        assert_eq!(a.tile_px.to_native(), 256);
        assert_eq!(a.levels.len(), 2);
        assert_eq!(a.levels[0].tiles[1].offset.to_native(), 32_768);
        corruption_is_rejected::<TbdSatIndexV2>(&bytes);
    }

    /// The `.tbd-sat` v2 file shape end to end: the fixed header frames the rkyv index, and the
    /// index survives `access_checked` after being carved back out of the payload by `index_len`.
    /// This is the join between Tier 1 and Tier 2, and it is the one place a wrong `index_len`
    /// would be invisible in either half's own tests.
    #[test]
    fn sat_v2_header_frames_an_index_that_still_validates() {
        let index = to_bytes(&sat_index()).expect("serialise index");
        let head = TbdsHeader::new(u32::try_from(index.len()).expect("index fits u32"));
        let tiles: [u8; 12] = [0xAB; 12];

        let mut file = Vec::with_capacity(head.tiles_offset() + tiles.len());
        file.extend_from_slice(&head.to_header_bytes());
        file.extend_from_slice(&index);
        file.extend_from_slice(&tiles);
        assert_eq!(file.len(), head.tiles_offset() + tiles.len());

        let (h, payload) = TbdsHeader::read(&file).expect("header");
        let index_bytes = h.index(payload).expect("index block");
        assert_eq!(index_bytes, index.as_slice());
        // rkyv needs the archive aligned; a Vec<u8> is not, so copy it back into an AlignedVec —
        // exactly what the T-935.10 reader will do with a Range response.
        let mut aligned = rkyv::util::AlignedVec::<16>::new();
        aligned.extend_from_slice(index_bytes);
        let a = access_checked::<TbdSatIndexV2>(&aligned).expect("index validates in place");
        assert_eq!(a.base_w.to_native(), 12800);
        assert_eq!(&file[h.tiles_offset()..], &tiles[..]);
    }

    /// Wrong type over right bytes: reading a road network as a sat index must fail validation
    /// rather than hand back nonsense. This is what proves `access_checked` inspects the buffer
    /// instead of just casting it.
    #[test]
    fn a_different_archive_type_does_not_validate() {
        let bytes = to_bytes(&map_labels()).expect("serialise");
        let err = access_checked::<TbdSatIndexV2>(&bytes)
            .expect_err("MapLabelsArchive bytes are not a TbdSatIndexV2");
        assert!(matches!(err, BinaryError::Archive { .. }), "{err}");
    }

    #[test]
    fn empty_archives_round_trip() {
        round_trip(&RoadNetworkArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            segments: vec![],
        });
        round_trip(&ForestRegionsArchive {
            schema_version: ARCHIVE_SCHEMA_VERSION,
            regions: vec![],
        });
        round_trip(&TbdSatIndexV2 {
            base_w: 0,
            base_h: 0,
            tile_px: 0,
            levels: vec![],
        });
    }
}
