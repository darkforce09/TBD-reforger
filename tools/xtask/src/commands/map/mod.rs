//! Forward map commands with the active checkout as their repository input.
use anyhow::Result;
use repository_layout::prelude::find_repository_root;

/// Runs `map blueprint-from-voxels`: the voxel-dump interpreter of the blueprint compiler.
pub(crate) fn run(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::run(&find_repository_root()?, args)?)
}

/// Runs `map ingest-blueprints`: the building-blueprint ingest into `assets/terrains`.
pub(crate) fn ingest_blueprints(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::blueprint_ingestion::run(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map parity-report`: the Workbench parity oracle replay and its agreement report.
pub(crate) fn parity_report(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::parity_report::run(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map world-los`: the world occluder over the committed catalogue.
pub(crate) fn world_line_of_sight(args: &[String]) -> Result<u8> {
    Ok(map_asset_verification::world_line_of_sight::run(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map voxels-from-mesh`: a voxel dump ray-marched from a mesh file.
pub(crate) fn run_mesh_voxelization(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::run_mesh_voxelization(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map bvh-parity`: the one-number line-of-sight parity proof.
pub(crate) fn run_occlusion_sidecar_parity(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::run_occlusion_sidecar_parity(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map bvh-emit`: the binary `.bvh` occlusion sidecar of one building.
pub(crate) fn run_occlusion_sidecar_emission(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::run_occlusion_sidecar_emission(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map bvh-batch`: a building prefab closure walked out of the game paks.
pub(crate) fn run_occlusion_sidecar_batch(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::run_occlusion_sidecar_batch(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map xob-inspect`: what the XOB decoder reads from one mesh file.
pub(crate) fn run_xob_inspection(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::run_xob_inspection(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map pak-cat`: one game-pak entry read out.
pub(crate) fn run_pak_file_print(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::run_pak_file_print(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map instances-verify`: socket instances matched against a Workbench recon dump.
pub(crate) fn run_instance_verification(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::run_instance_verification(
        &find_repository_root()?,
        args,
    )?)
}

/// Runs `map rotation-pin`: the Euler-composition hypotheses ranked against a recon sample.
pub(crate) fn run_rotation_validation(args: &[String]) -> Result<u8> {
    Ok(blueprint_compiler::run_rotation_validation(
        &find_repository_root()?,
        args,
    )?)
}

pub(crate) mod cli;
pub(crate) mod dispatch;
