use super::cli::MapCmd;
use crate::commands;
use anyhow::Result;

pub(crate) fn run(cmd: MapCmd) -> Result<u8> {
    match cmd {
        MapCmd::ExportTerrain { args } => {
            Ok(world_export_pipeline::export_terrain_driver::run(&args)?)
        }
        MapCmd::TileIndex { args } => Ok(world_export_pipeline::map_tile_index::run(&args)?),
        MapCmd::IngestBlueprints { args } => commands::map::ingest_blueprints(&args),
        MapCmd::ParityReport { args } => commands::map::parity_report(&args),
        MapCmd::BlueprintFromVoxels { args } => commands::map::run(&args),
        MapCmd::VoxelsFromMesh { args } => commands::map::run_mesh_voxelization(&args),
        MapCmd::BvhParity { args } => commands::map::run_occlusion_sidecar_parity(&args),
        MapCmd::BvhEmit { args } => commands::map::run_occlusion_sidecar_emission(&args),
        MapCmd::BvhBatch { args } => commands::map::run_occlusion_sidecar_batch(&args),
        MapCmd::XobInspect { args } => commands::map::run_xob_inspection(&args),
        MapCmd::PakCat { args } => commands::map::run_pak_file_print(&args),
        MapCmd::InstancesVerify { args } => commands::map::run_instance_verification(&args),
        MapCmd::RotationPin { args } => commands::map::run_rotation_validation(&args),
        MapCmd::WorldLos { args } => commands::map::world_line_of_sight(&args),
    }
}
