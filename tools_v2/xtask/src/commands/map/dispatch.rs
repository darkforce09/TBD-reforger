use super::cli::MapCmd;
use crate::*;
use anyhow::Result;

pub(crate) fn run(cmd: MapCmd) -> Result<u8> {
    match cmd {
        MapCmd::ExportTerrain { args } => crate::commands::map::terrain_export::run(&args),
        MapCmd::IngestBlueprints { args } => commands::map::ingest_blueprints(&args),
        MapCmd::ParityReport { args } => commands::map::parity_report(&args),
        MapCmd::BlueprintFromVoxels { args } => commands::map::run(&args),
        MapCmd::VoxelsFromMesh { args } => commands::map::run_voxels_from_mesh(&args),
        MapCmd::BvhParity { args } => commands::map::run_bvh_parity(&args),
        MapCmd::BvhEmit { args } => commands::map::run_bvh_emit(&args),
        MapCmd::BvhBatch { args } => commands::map::run_bvh_batch(&args),
        MapCmd::XobInspect { args } => commands::map::run_xob_inspect(&args),
        MapCmd::PakCat { args } => commands::map::run_pak_cat(&args),
        MapCmd::InstancesVerify { args } => commands::map::run_instances_verify(&args),
        MapCmd::RotationPin { args } => commands::map::run_rotation_pin(&args),
        MapCmd::WorldLos { args } => commands::map::world_line_of_sight(&args),
    }
}
