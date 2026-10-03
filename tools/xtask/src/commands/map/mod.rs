//! Forward map commands with the active checkout as their repository input.
use anyhow::Result;
use repository_layout::find_repository_root;

pub fn run(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run(&find_repository_root()?, args)
}

pub fn ingest_blueprints(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::ingest::run(&find_repository_root()?, args)
}

pub fn parity_report(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::parity_report::run(&find_repository_root()?, args)
}

pub fn world_line_of_sight(args: &[String]) -> Result<u8> {
    developer_tools::map_verification::world_line_of_sight::run(&find_repository_root()?, args)
}

pub fn run_voxels_from_mesh(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_voxels_from_mesh(&find_repository_root()?, args)
}

pub fn run_bvh_parity(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_bvh_parity(&find_repository_root()?, args)
}

pub fn run_bvh_emit(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_bvh_emit(&find_repository_root()?, args)
}

pub fn run_bvh_batch(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_bvh_batch(&find_repository_root()?, args)
}

pub fn run_xob_inspect(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_xob_inspect(&find_repository_root()?, args)
}

pub fn run_pak_cat(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_pak_cat(&find_repository_root()?, args)
}

pub fn run_instances_verify(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_instances_verify(&find_repository_root()?, args)
}

pub fn run_rotation_pin(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_rotation_pin(&find_repository_root()?, args)
}

pub(crate) mod cli;
pub(crate) mod dispatch;

pub(crate) mod terrain_export;
pub(crate) mod tile_index;
