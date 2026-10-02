//! Role: Module boundary for formats/archives/models.
//! Position: `io/archives/models` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Re-export `crate::io::archives::version::ARCHIVE_SCHEMA_VERSION`.
pub use crate::io::archives::version::ARCHIVE_SCHEMA_VERSION;

/// Re-export `crate::io::archives::roads::ArchivedRoadNetworkArchive`.
pub use crate::io::archives::roads::ArchivedRoadNetworkArchive;

/// Re-export `crate::io::archives::roads::ArchivedRoadSegmentArchive`.
pub use crate::io::archives::roads::ArchivedRoadSegmentArchive;

/// Re-export `crate::io::archives::roads::RoadNetworkArchive`.
pub use crate::io::archives::roads::RoadNetworkArchive;

/// Re-export `crate::io::archives::roads::RoadNetworkArchiveResolver`.
pub use crate::io::archives::roads::RoadNetworkArchiveResolver;

/// Re-export `crate::io::archives::roads::RoadSegmentArchive`.
pub use crate::io::archives::roads::RoadSegmentArchive;

/// Re-export `crate::io::archives::roads::RoadSegmentArchiveResolver`.
pub use crate::io::archives::roads::RoadSegmentArchiveResolver;

/// Re-export `crate::io::archives::labels::ArchivedHeightLabel`.
pub use crate::io::archives::labels::ArchivedHeightLabel;

/// Re-export `crate::io::archives::labels::ArchivedMapLabelsArchive`.
pub use crate::io::archives::labels::ArchivedMapLabelsArchive;

/// Re-export `crate::io::archives::labels::ArchivedRoadNameLabel`.
pub use crate::io::archives::labels::ArchivedRoadNameLabel;

/// Re-export `crate::io::archives::labels::ArchivedTownLabel`.
pub use crate::io::archives::labels::ArchivedTownLabel;

/// Re-export `crate::io::archives::labels::HeightLabel`.
pub use crate::io::archives::labels::HeightLabel;

/// Re-export `crate::io::archives::labels::HeightLabelResolver`.
pub use crate::io::archives::labels::HeightLabelResolver;

/// Re-export `crate::io::archives::labels::MapLabelsArchive`.
pub use crate::io::archives::labels::MapLabelsArchive;

/// Re-export `crate::io::archives::labels::MapLabelsArchiveResolver`.
pub use crate::io::archives::labels::MapLabelsArchiveResolver;

/// Re-export `crate::io::archives::labels::RoadNameLabel`.
pub use crate::io::archives::labels::RoadNameLabel;

/// Re-export `crate::io::archives::labels::RoadNameLabelResolver`.
pub use crate::io::archives::labels::RoadNameLabelResolver;

/// Re-export `crate::io::archives::labels::TownLabel`.
pub use crate::io::archives::labels::TownLabel;

/// Re-export `crate::io::archives::labels::TownLabelResolver`.
pub use crate::io::archives::labels::TownLabelResolver;

/// Re-export `crate::io::archives::water::ArchivedWaterBody`.
pub use crate::io::archives::water::ArchivedWaterBody;

/// Re-export `crate::io::archives::water::ArchivedWaterLine`.
pub use crate::io::archives::water::ArchivedWaterLine;

/// Re-export `crate::io::archives::water::ArchivedWaterVectorsArchive`.
pub use crate::io::archives::water::ArchivedWaterVectorsArchive;

/// Re-export `crate::io::archives::water::WaterBody`.
pub use crate::io::archives::water::WaterBody;

/// Re-export `crate::io::archives::water::WaterBodyResolver`.
pub use crate::io::archives::water::WaterBodyResolver;

/// Re-export `crate::io::archives::water::WaterLine`.
pub use crate::io::archives::water::WaterLine;

/// Re-export `crate::io::archives::water::WaterLineResolver`.
pub use crate::io::archives::water::WaterLineResolver;

/// Re-export `crate::io::archives::water::WaterVectorsArchive`.
pub use crate::io::archives::water::WaterVectorsArchive;

/// Re-export `crate::io::archives::water::WaterVectorsArchiveResolver`.
pub use crate::io::archives::water::WaterVectorsArchiveResolver;

/// Re-export `crate::io::archives::prefabs::ArchivedKindCensus`.
pub use crate::io::archives::prefabs::ArchivedKindCensus;

/// Re-export `crate::io::archives::prefabs::ArchivedPrefabCatalogArchive`.
pub use crate::io::archives::prefabs::ArchivedPrefabCatalogArchive;

/// Re-export `crate::io::archives::prefabs::ArchivedPrefabEntry`.
pub use crate::io::archives::prefabs::ArchivedPrefabEntry;

/// Re-export `crate::io::archives::prefabs::ArchivedTypeInventory`.
pub use crate::io::archives::prefabs::ArchivedTypeInventory;

/// Re-export `crate::io::archives::prefabs::KindCensus`.
pub use crate::io::archives::prefabs::KindCensus;

/// Re-export `crate::io::archives::prefabs::KindCensusResolver`.
pub use crate::io::archives::prefabs::KindCensusResolver;

/// Re-export `crate::io::archives::prefabs::PrefabCatalogArchive`.
pub use crate::io::archives::prefabs::PrefabCatalogArchive;

/// Re-export `crate::io::archives::prefabs::PrefabCatalogArchiveResolver`.
pub use crate::io::archives::prefabs::PrefabCatalogArchiveResolver;

/// Re-export `crate::io::archives::prefabs::PrefabEntry`.
pub use crate::io::archives::prefabs::PrefabEntry;

/// Re-export `crate::io::archives::prefabs::PrefabEntryResolver`.
pub use crate::io::archives::prefabs::PrefabEntryResolver;

/// Re-export `crate::io::archives::prefabs::TypeInventory`.
pub use crate::io::archives::prefabs::TypeInventory;

/// Re-export `crate::io::archives::prefabs::TypeInventoryResolver`.
pub use crate::io::archives::prefabs::TypeInventoryResolver;

/// Re-export `crate::io::archives::forest::ArchivedForestRegion`.
pub use crate::io::archives::forest::ArchivedForestRegion;

/// Re-export `crate::io::archives::forest::ArchivedForestRegionsArchive`.
pub use crate::io::archives::forest::ArchivedForestRegionsArchive;

/// Re-export `crate::io::archives::forest::ForestRegion`.
pub use crate::io::archives::forest::ForestRegion;

/// Re-export `crate::io::archives::forest::ForestRegionResolver`.
pub use crate::io::archives::forest::ForestRegionResolver;

/// Re-export `crate::io::archives::forest::ForestRegionsArchive`.
pub use crate::io::archives::forest::ForestRegionsArchive;

/// Re-export `crate::io::archives::forest::ForestRegionsArchiveResolver`.
pub use crate::io::archives::forest::ForestRegionsArchiveResolver;

/// Re-export `crate::io::archives::blueprints::ArchivedBlasEntry`.
pub use crate::io::archives::blueprints::ArchivedBlasEntry;

/// Re-export `crate::io::archives::blueprints::ArchivedBuildingBlueprint`.
pub use crate::io::archives::blueprints::ArchivedBuildingBlueprint;

/// Re-export `crate::io::archives::blueprints::ArchivedBuildingBlueprintArchive`.
pub use crate::io::archives::blueprints::ArchivedBuildingBlueprintArchive;

/// Re-export `crate::io::archives::blueprints::ArchivedBuildingLevel`.
pub use crate::io::archives::blueprints::ArchivedBuildingLevel;

/// Re-export `crate::io::archives::blueprints::ArchivedDoorRec`.
pub use crate::io::archives::blueprints::ArchivedDoorRec;

/// Re-export `crate::io::archives::blueprints::ArchivedFurnitureRec`.
pub use crate::io::archives::blueprints::ArchivedFurnitureRec;

/// Re-export `crate::io::archives::blueprints::ArchivedOccluderDescriptor`.
pub use crate::io::archives::blueprints::ArchivedOccluderDescriptor;

/// Re-export `crate::io::archives::blueprints::ArchivedStairsRec`.
pub use crate::io::archives::blueprints::ArchivedStairsRec;

/// Re-export `crate::io::archives::blueprints::ArchivedVerticalProfile`.
pub use crate::io::archives::blueprints::ArchivedVerticalProfile;

/// Re-export `crate::io::archives::blueprints::ArchivedWallRec`.
pub use crate::io::archives::blueprints::ArchivedWallRec;

/// Re-export `crate::io::archives::blueprints::ArchivedWindowRec`.
pub use crate::io::archives::blueprints::ArchivedWindowRec;

/// Re-export `crate::io::archives::blueprints::BlasEntry`.
pub use crate::io::archives::blueprints::BlasEntry;

/// Re-export `crate::io::archives::blueprints::BlasEntryResolver`.
pub use crate::io::archives::blueprints::BlasEntryResolver;

/// Re-export `crate::io::archives::blueprints::BuildingBlueprint`.
pub use crate::io::archives::blueprints::BuildingBlueprint;

/// Re-export `crate::io::archives::blueprints::BuildingBlueprintArchive`.
pub use crate::io::archives::blueprints::BuildingBlueprintArchive;

/// Re-export `crate::io::archives::blueprints::BuildingBlueprintArchiveResolver`.
pub use crate::io::archives::blueprints::BuildingBlueprintArchiveResolver;

/// Re-export `crate::io::archives::blueprints::BuildingBlueprintResolver`.
pub use crate::io::archives::blueprints::BuildingBlueprintResolver;

/// Re-export `crate::io::archives::blueprints::BuildingLevel`.
pub use crate::io::archives::blueprints::BuildingLevel;

/// Re-export `crate::io::archives::blueprints::BuildingLevelResolver`.
pub use crate::io::archives::blueprints::BuildingLevelResolver;

/// Re-export `crate::io::archives::blueprints::DoorRec`.
pub use crate::io::archives::blueprints::DoorRec;

/// Re-export `crate::io::archives::blueprints::DoorRecResolver`.
pub use crate::io::archives::blueprints::DoorRecResolver;

/// Re-export `crate::io::archives::blueprints::FurnitureRec`.
pub use crate::io::archives::blueprints::FurnitureRec;

/// Re-export `crate::io::archives::blueprints::FurnitureRecResolver`.
pub use crate::io::archives::blueprints::FurnitureRecResolver;

/// Re-export `crate::io::archives::blueprints::OccluderDescriptor`.
pub use crate::io::archives::blueprints::OccluderDescriptor;

/// Re-export `crate::io::archives::blueprints::OccluderDescriptorResolver`.
pub use crate::io::archives::blueprints::OccluderDescriptorResolver;

/// Re-export `crate::io::archives::blueprints::StairsRec`.
pub use crate::io::archives::blueprints::StairsRec;

/// Re-export `crate::io::archives::blueprints::StairsRecResolver`.
pub use crate::io::archives::blueprints::StairsRecResolver;

/// Re-export `crate::io::archives::blueprints::VerticalProfile`.
pub use crate::io::archives::blueprints::VerticalProfile;

/// Re-export `crate::io::archives::blueprints::VerticalProfileResolver`.
pub use crate::io::archives::blueprints::VerticalProfileResolver;

/// Re-export `crate::io::archives::blueprints::WallRec`.
pub use crate::io::archives::blueprints::WallRec;

/// Re-export `crate::io::archives::blueprints::WallRecResolver`.
pub use crate::io::archives::blueprints::WallRecResolver;

/// Re-export `crate::io::archives::blueprints::WindowRec`.
pub use crate::io::archives::blueprints::WindowRec;

/// Re-export `crate::io::archives::blueprints::WindowRecResolver`.
pub use crate::io::archives::blueprints::WindowRecResolver;

/// Re-export `crate::io::archives::satellite::ArchivedSatLevel`.
pub use crate::io::archives::satellite::ArchivedSatLevel;

/// Re-export `crate::io::archives::satellite::ArchivedSatTile`.
pub use crate::io::archives::satellite::ArchivedSatTile;

/// Re-export `crate::io::archives::satellite::ArchivedTbdSatIndexV2`.
pub use crate::io::archives::satellite::ArchivedTbdSatIndexV2;

/// Re-export `crate::io::archives::satellite::SatLevel`.
pub use crate::io::archives::satellite::SatLevel;

/// Re-export `crate::io::archives::satellite::SatLevelResolver`.
pub use crate::io::archives::satellite::SatLevelResolver;

/// Re-export `crate::io::archives::satellite::SatTile`.
pub use crate::io::archives::satellite::SatTile;

/// Re-export `crate::io::archives::satellite::SatTileResolver`.
pub use crate::io::archives::satellite::SatTileResolver;

/// Re-export `crate::io::archives::satellite::TbdSatIndexV2`.
pub use crate::io::archives::satellite::TbdSatIndexV2;

/// Re-export `crate::io::archives::satellite::TbdSatIndexV2Resolver`.
pub use crate::io::archives::satellite::TbdSatIndexV2Resolver;
#[cfg(test)]
mod tests;
