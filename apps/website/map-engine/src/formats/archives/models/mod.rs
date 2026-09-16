//! Role: Module boundary for formats/archives/models.
//! Position: `formats/archives/models` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::formats::archives::version::ARCHIVE_SCHEMA_VERSION`.
pub use crate::formats::archives::version::ARCHIVE_SCHEMA_VERSION;

/// Re-export `crate::formats::archives::roads::ArchivedRoadNetworkArchive`.
pub use crate::formats::archives::roads::ArchivedRoadNetworkArchive;

/// Re-export `crate::formats::archives::roads::ArchivedRoadSegmentArchive`.
pub use crate::formats::archives::roads::ArchivedRoadSegmentArchive;

/// Re-export `crate::formats::archives::roads::RoadNetworkArchive`.
pub use crate::formats::archives::roads::RoadNetworkArchive;

/// Re-export `crate::formats::archives::roads::RoadNetworkArchiveResolver`.
pub use crate::formats::archives::roads::RoadNetworkArchiveResolver;

/// Re-export `crate::formats::archives::roads::RoadSegmentArchive`.
pub use crate::formats::archives::roads::RoadSegmentArchive;

/// Re-export `crate::formats::archives::roads::RoadSegmentArchiveResolver`.
pub use crate::formats::archives::roads::RoadSegmentArchiveResolver;

/// Re-export `crate::formats::archives::labels::ArchivedHeightLabel`.
pub use crate::formats::archives::labels::ArchivedHeightLabel;

/// Re-export `crate::formats::archives::labels::ArchivedMapLabelsArchive`.
pub use crate::formats::archives::labels::ArchivedMapLabelsArchive;

/// Re-export `crate::formats::archives::labels::ArchivedRoadNameLabel`.
pub use crate::formats::archives::labels::ArchivedRoadNameLabel;

/// Re-export `crate::formats::archives::labels::ArchivedTownLabel`.
pub use crate::formats::archives::labels::ArchivedTownLabel;

/// Re-export `crate::formats::archives::labels::HeightLabel`.
pub use crate::formats::archives::labels::HeightLabel;

/// Re-export `crate::formats::archives::labels::HeightLabelResolver`.
pub use crate::formats::archives::labels::HeightLabelResolver;

/// Re-export `crate::formats::archives::labels::MapLabelsArchive`.
pub use crate::formats::archives::labels::MapLabelsArchive;

/// Re-export `crate::formats::archives::labels::MapLabelsArchiveResolver`.
pub use crate::formats::archives::labels::MapLabelsArchiveResolver;

/// Re-export `crate::formats::archives::labels::RoadNameLabel`.
pub use crate::formats::archives::labels::RoadNameLabel;

/// Re-export `crate::formats::archives::labels::RoadNameLabelResolver`.
pub use crate::formats::archives::labels::RoadNameLabelResolver;

/// Re-export `crate::formats::archives::labels::TownLabel`.
pub use crate::formats::archives::labels::TownLabel;

/// Re-export `crate::formats::archives::labels::TownLabelResolver`.
pub use crate::formats::archives::labels::TownLabelResolver;

/// Re-export `crate::formats::archives::water::ArchivedWaterBody`.
pub use crate::formats::archives::water::ArchivedWaterBody;

/// Re-export `crate::formats::archives::water::ArchivedWaterLine`.
pub use crate::formats::archives::water::ArchivedWaterLine;

/// Re-export `crate::formats::archives::water::ArchivedWaterVectorsArchive`.
pub use crate::formats::archives::water::ArchivedWaterVectorsArchive;

/// Re-export `crate::formats::archives::water::WaterBody`.
pub use crate::formats::archives::water::WaterBody;

/// Re-export `crate::formats::archives::water::WaterBodyResolver`.
pub use crate::formats::archives::water::WaterBodyResolver;

/// Re-export `crate::formats::archives::water::WaterLine`.
pub use crate::formats::archives::water::WaterLine;

/// Re-export `crate::formats::archives::water::WaterLineResolver`.
pub use crate::formats::archives::water::WaterLineResolver;

/// Re-export `crate::formats::archives::water::WaterVectorsArchive`.
pub use crate::formats::archives::water::WaterVectorsArchive;

/// Re-export `crate::formats::archives::water::WaterVectorsArchiveResolver`.
pub use crate::formats::archives::water::WaterVectorsArchiveResolver;

/// Re-export `crate::formats::archives::prefabs::ArchivedKindCensus`.
pub use crate::formats::archives::prefabs::ArchivedKindCensus;

/// Re-export `crate::formats::archives::prefabs::ArchivedPrefabCatalogArchive`.
pub use crate::formats::archives::prefabs::ArchivedPrefabCatalogArchive;

/// Re-export `crate::formats::archives::prefabs::ArchivedPrefabEntry`.
pub use crate::formats::archives::prefabs::ArchivedPrefabEntry;

/// Re-export `crate::formats::archives::prefabs::ArchivedTypeInventory`.
pub use crate::formats::archives::prefabs::ArchivedTypeInventory;

/// Re-export `crate::formats::archives::prefabs::KindCensus`.
pub use crate::formats::archives::prefabs::KindCensus;

/// Re-export `crate::formats::archives::prefabs::KindCensusResolver`.
pub use crate::formats::archives::prefabs::KindCensusResolver;

/// Re-export `crate::formats::archives::prefabs::PrefabCatalogArchive`.
pub use crate::formats::archives::prefabs::PrefabCatalogArchive;

/// Re-export `crate::formats::archives::prefabs::PrefabCatalogArchiveResolver`.
pub use crate::formats::archives::prefabs::PrefabCatalogArchiveResolver;

/// Re-export `crate::formats::archives::prefabs::PrefabEntry`.
pub use crate::formats::archives::prefabs::PrefabEntry;

/// Re-export `crate::formats::archives::prefabs::PrefabEntryResolver`.
pub use crate::formats::archives::prefabs::PrefabEntryResolver;

/// Re-export `crate::formats::archives::prefabs::TypeInventory`.
pub use crate::formats::archives::prefabs::TypeInventory;

/// Re-export `crate::formats::archives::prefabs::TypeInventoryResolver`.
pub use crate::formats::archives::prefabs::TypeInventoryResolver;

/// Re-export `crate::formats::archives::forest::ArchivedForestRegion`.
pub use crate::formats::archives::forest::ArchivedForestRegion;

/// Re-export `crate::formats::archives::forest::ArchivedForestRegionsArchive`.
pub use crate::formats::archives::forest::ArchivedForestRegionsArchive;

/// Re-export `crate::formats::archives::forest::ForestRegion`.
pub use crate::formats::archives::forest::ForestRegion;

/// Re-export `crate::formats::archives::forest::ForestRegionResolver`.
pub use crate::formats::archives::forest::ForestRegionResolver;

/// Re-export `crate::formats::archives::forest::ForestRegionsArchive`.
pub use crate::formats::archives::forest::ForestRegionsArchive;

/// Re-export `crate::formats::archives::forest::ForestRegionsArchiveResolver`.
pub use crate::formats::archives::forest::ForestRegionsArchiveResolver;

/// Re-export `crate::formats::archives::blueprints::ArchivedBlasEntry`.
pub use crate::formats::archives::blueprints::ArchivedBlasEntry;

/// Re-export `crate::formats::archives::blueprints::ArchivedBuildingBlueprint`.
pub use crate::formats::archives::blueprints::ArchivedBuildingBlueprint;

/// Re-export `crate::formats::archives::blueprints::ArchivedBuildingBlueprintArchive`.
pub use crate::formats::archives::blueprints::ArchivedBuildingBlueprintArchive;

/// Re-export `crate::formats::archives::blueprints::ArchivedBuildingLevel`.
pub use crate::formats::archives::blueprints::ArchivedBuildingLevel;

/// Re-export `crate::formats::archives::blueprints::ArchivedDoorRec`.
pub use crate::formats::archives::blueprints::ArchivedDoorRec;

/// Re-export `crate::formats::archives::blueprints::ArchivedFurnitureRec`.
pub use crate::formats::archives::blueprints::ArchivedFurnitureRec;

/// Re-export `crate::formats::archives::blueprints::ArchivedOccluderDescriptor`.
pub use crate::formats::archives::blueprints::ArchivedOccluderDescriptor;

/// Re-export `crate::formats::archives::blueprints::ArchivedStairsRec`.
pub use crate::formats::archives::blueprints::ArchivedStairsRec;

/// Re-export `crate::formats::archives::blueprints::ArchivedVerticalProfile`.
pub use crate::formats::archives::blueprints::ArchivedVerticalProfile;

/// Re-export `crate::formats::archives::blueprints::ArchivedWallRec`.
pub use crate::formats::archives::blueprints::ArchivedWallRec;

/// Re-export `crate::formats::archives::blueprints::ArchivedWindowRec`.
pub use crate::formats::archives::blueprints::ArchivedWindowRec;

/// Re-export `crate::formats::archives::blueprints::BlasEntry`.
pub use crate::formats::archives::blueprints::BlasEntry;

/// Re-export `crate::formats::archives::blueprints::BlasEntryResolver`.
pub use crate::formats::archives::blueprints::BlasEntryResolver;

/// Re-export `crate::formats::archives::blueprints::BuildingBlueprint`.
pub use crate::formats::archives::blueprints::BuildingBlueprint;

/// Re-export `crate::formats::archives::blueprints::BuildingBlueprintArchive`.
pub use crate::formats::archives::blueprints::BuildingBlueprintArchive;

/// Re-export `crate::formats::archives::blueprints::BuildingBlueprintArchiveResolver`.
pub use crate::formats::archives::blueprints::BuildingBlueprintArchiveResolver;

/// Re-export `crate::formats::archives::blueprints::BuildingBlueprintResolver`.
pub use crate::formats::archives::blueprints::BuildingBlueprintResolver;

/// Re-export `crate::formats::archives::blueprints::BuildingLevel`.
pub use crate::formats::archives::blueprints::BuildingLevel;

/// Re-export `crate::formats::archives::blueprints::BuildingLevelResolver`.
pub use crate::formats::archives::blueprints::BuildingLevelResolver;

/// Re-export `crate::formats::archives::blueprints::DoorRec`.
pub use crate::formats::archives::blueprints::DoorRec;

/// Re-export `crate::formats::archives::blueprints::DoorRecResolver`.
pub use crate::formats::archives::blueprints::DoorRecResolver;

/// Re-export `crate::formats::archives::blueprints::FurnitureRec`.
pub use crate::formats::archives::blueprints::FurnitureRec;

/// Re-export `crate::formats::archives::blueprints::FurnitureRecResolver`.
pub use crate::formats::archives::blueprints::FurnitureRecResolver;

/// Re-export `crate::formats::archives::blueprints::OccluderDescriptor`.
pub use crate::formats::archives::blueprints::OccluderDescriptor;

/// Re-export `crate::formats::archives::blueprints::OccluderDescriptorResolver`.
pub use crate::formats::archives::blueprints::OccluderDescriptorResolver;

/// Re-export `crate::formats::archives::blueprints::StairsRec`.
pub use crate::formats::archives::blueprints::StairsRec;

/// Re-export `crate::formats::archives::blueprints::StairsRecResolver`.
pub use crate::formats::archives::blueprints::StairsRecResolver;

/// Re-export `crate::formats::archives::blueprints::VerticalProfile`.
pub use crate::formats::archives::blueprints::VerticalProfile;

/// Re-export `crate::formats::archives::blueprints::VerticalProfileResolver`.
pub use crate::formats::archives::blueprints::VerticalProfileResolver;

/// Re-export `crate::formats::archives::blueprints::WallRec`.
pub use crate::formats::archives::blueprints::WallRec;

/// Re-export `crate::formats::archives::blueprints::WallRecResolver`.
pub use crate::formats::archives::blueprints::WallRecResolver;

/// Re-export `crate::formats::archives::blueprints::WindowRec`.
pub use crate::formats::archives::blueprints::WindowRec;

/// Re-export `crate::formats::archives::blueprints::WindowRecResolver`.
pub use crate::formats::archives::blueprints::WindowRecResolver;

/// Re-export `crate::formats::archives::satellite::ArchivedSatLevel`.
pub use crate::formats::archives::satellite::ArchivedSatLevel;

/// Re-export `crate::formats::archives::satellite::ArchivedSatTile`.
pub use crate::formats::archives::satellite::ArchivedSatTile;

/// Re-export `crate::formats::archives::satellite::ArchivedTbdSatIndexV2`.
pub use crate::formats::archives::satellite::ArchivedTbdSatIndexV2;

/// Re-export `crate::formats::archives::satellite::SatLevel`.
pub use crate::formats::archives::satellite::SatLevel;

/// Re-export `crate::formats::archives::satellite::SatLevelResolver`.
pub use crate::formats::archives::satellite::SatLevelResolver;

/// Re-export `crate::formats::archives::satellite::SatTile`.
pub use crate::formats::archives::satellite::SatTile;

/// Re-export `crate::formats::archives::satellite::SatTileResolver`.
pub use crate::formats::archives::satellite::SatTileResolver;

/// Re-export `crate::formats::archives::satellite::TbdSatIndexV2`.
pub use crate::formats::archives::satellite::TbdSatIndexV2;

/// Re-export `crate::formats::archives::satellite::TbdSatIndexV2Resolver`.
pub use crate::formats::archives::satellite::TbdSatIndexV2Resolver;
#[cfg(test)]
mod tests;
