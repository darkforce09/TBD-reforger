//! Line of sight through every object placed on a terrain, over the chunks a host made resident.
//!
//! **Role:** holds the resident world as the [`WorldOccluder`] ([`world_occluder`]), fed by the
//! residency calls ([`residency`]): each chunk's rows under a box tree ([`chunk_occluder`],
//! [`instance_box_tree`]), the prefab occluder library ([`occluder_library`]) expanding a prefab
//! into its exact meshes once its files arrive and a proxy box standing in until then. Answers a
//! segment by walking the chunks under it ([`chunk_cells`], [`raycast`]) and evaluating its
//! crossings into a verdict that says what it could not see ([`sight_line`], [`query_types`]);
//! tells a host what to fetch next ([`occluder_readouts`]).
//! **Position:** line of sight category, tier 5, over `world_chunks`, `prefab_catalog`,
//! `interior_line_of_sight` (the instance trace and the shared sight-line evaluation),
//! `building_interiors`, `spatial_indexes`, `world_file_formats`, `map_coordinates` and
//! `geometry_primitives`. The map engine's occluder loader, line-of-sight tool and object wash,
//! the Mission Creator's debug world bench and the developer tools' world check and blueprint
//! tooling read it. The crate root is its public surface.
//! **Signals & state:** the occluder owns the resident state; everything else is pure.
//! **Invariants:** a verdict over a proxy box or a chunk that is not resident is `Provisional`;
//! the box trees answer exactly what the brute-force scans answer; the BLAS byte budget evicts
//! only what no placed, expanded prefab needs.

pub mod chunk_cells;
pub mod chunk_occluder;
pub mod error;
pub mod instance_box_tree;
pub mod occluder_library;
pub mod occluder_readouts;
pub mod prelude;
pub mod query_types;
pub mod raycast;
pub mod residency;
pub mod sight_line;
pub mod world_occluder;

#[cfg(test)]
#[path = "tests/occluder_tests.rs"]
mod tests;

pub use chunk_cells::cells_on_segment;
pub use chunk_occluder::{ChunkOccluder, NO_BOX, WorldInstance, rows_of_chunk};
pub use error::{Error, Result};
pub use instance_box_tree::{AabbTlas, Candidate};
pub use occluder_library::{
    ArchiveBoot, ArchiveProjectionError, BlasEntry, BlasManifest, BuildingArchiveBytes,
    DESCRIPTOR_SCHEMA_VERSION, DescEntry, KindTotals, MANIFEST_SCHEMA_VERSION, PrefabDescriptor,
    Totals,
};
pub use occluder_readouts::Wanted;
pub use query_types::{
    BlockPolicy, Coverage, Fidelity, WorldEvent, WorldLos, WorldVerdict, map_to_engine,
};
pub use world_occluder::{DEFAULT_BLAS_CAP_BYTES, PrefabOccluder, WorldOccluder};
