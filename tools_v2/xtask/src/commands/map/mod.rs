//! Forward map commands with the active checkout as their repository input.
use crate::root::find_repo_root;
use anyhow::Result;

pub fn run(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run(&find_repo_root()?, args)
}

pub fn ingest_blueprints(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::ingest::run(&find_repo_root()?, args)
}

pub fn parity_report(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::parity_report::run(&find_repo_root()?, args)
}

pub fn world_line_of_sight(args: &[String]) -> Result<u8> {
    developer_tools::map_verification::world_line_of_sight::run(&find_repo_root()?, args)
}

pub fn run_voxels_from_mesh(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_voxels_from_mesh(&find_repo_root()?, args)
}

pub fn run_bvh_parity(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_bvh_parity(&find_repo_root()?, args)
}

pub fn run_bvh_emit(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_bvh_emit(&find_repo_root()?, args)
}

pub fn run_bvh_batch(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_bvh_batch(&find_repo_root()?, args)
}

pub fn run_xob_inspect(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_xob_inspect(&find_repo_root()?, args)
}

pub fn run_pak_cat(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_pak_cat(&find_repo_root()?, args)
}

pub fn run_instances_verify(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_instances_verify(&find_repo_root()?, args)
}

pub fn run_rotation_pin(args: &[String]) -> Result<u8> {
    developer_tools::blueprint::run_rotation_pin(&find_repo_root()?, args)
}
