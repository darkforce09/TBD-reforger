//! Why an offline pack cannot be built.
//!
//! **Role:** the crate's one error type and its `Result` alias.
//! **Position:** returned by [`crate::offline_pack::terrain_pack`]; the page reports it as the
//! reason the offline pack is unavailable.
//! **Signals & state:** none; plain data.
//! **Invariants:** a malformed manifest or tile index is an [`Error`], never an empty pack.

use crate::offline_pack::MapTileIndexEntry;

/// Why a pack cannot be built from a served terrain manifest and its map tile index.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The manifest is not JSON of the expected shape.
    #[error("the terrain manifest is unreadable: {0}")]
    ManifestUnreadable(String),
    /// The tile index is not JSON of the expected shape.
    #[error("the map tile index is unreadable: {0}")]
    TileIndexUnreadable(String),
    /// The manifest lacks a section the pack needs, named by its JSON path.
    #[error("the terrain manifest has no `{0}` section")]
    ManifestSectionMissing(&'static str),
    /// The manifest's `terrainId` is not a lower-case slug.
    #[error("the terrain manifest's terrainId {0:?} is not a lower-case slug")]
    InvalidTerrainId(String),
    /// The manifest declares a stub elevation model (`widthPx` or `heightPx` of 0).
    #[error("the terrain manifest declares a stub elevation model")]
    StubElevationModel,
    /// The map `urlTemplate` lacks one of `{z}`, `{x}`, `{y}`.
    #[error("the map urlTemplate {0:?} lacks one of {{z}}, {{x}}, {{y}}")]
    MapUrlTemplateInvalid(String),
    /// The tile index belongs to another terrain.
    #[error("the map tile index belongs to terrain {index:?}, the manifest to {manifest:?}")]
    TerrainMismatch {
        /// The manifest's `terrainId`.
        manifest: String,
        /// The index's `terrainId`.
        index: String,
    },
    /// The tile index has another `schemaVersion`.
    #[error("the map tile index has schemaVersion {0}")]
    UnsupportedTileIndexVersion(u32),
    /// A tile lies outside the manifest's zoom range or its level's `2^z` grid.
    #[error("map tile z{}/x{}/y{} lies outside the manifest's pyramid", .0.z, .0.x, .0.y)]
    TileOutsidePyramid(MapTileIndexEntry),
}

/// The result of a fallible call of this crate.
pub type Result<T> = std::result::Result<T, Error>;
